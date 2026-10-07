//! Watch-mode mechanics, behind one interface: folder watching, event
//! relevance, and the trailing-edge debounce.
//!
//! The module owns exactly the three mechanics `mdo --watch` needs; the
//! render itself and how the CLI reports it belong to the caller, passed as
//! a render callback. `watch::run` registers the watch, runs the callback
//! once immediately, and then once per settled quiet window whenever the
//! watched file plausibly changed content. A callback `Err` (e.g. a
//! transient filesystem state mid-rename) is ignored and never ends watch
//! mode, matching the CLI's contract; channel death ends the watch
//! gracefully.
//!
//! Both the debounce timing and the loop clock are injectable at the pump
//! level, so unit tests script fake event channels with a virtual clock and
//! cover burst collapse, late events, sibling noise, watcher error storms,
//! and channel death cross-platform by construction.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, channel, RecvError, RecvTimeoutError};
use std::time::{Duration, Instant};

use notify::{recommended_watcher, Event, EventKind, RecursiveMode, Watcher};

use crate::RenderError;

/// Trailing-edge debounce window: once a relevant event arrives the pump
/// keeps draining/absorbing further relevant events for this long before
/// rendering, so a burst (e.g. truncate + write, or temp-file write +
/// rename) collapses into exactly one render of the final content. This is
/// unlike a leading-edge "ignore anything for N ms after the last render"
/// debounce, which can drop the trailing event of a burst if the burst runs
/// longer than the window.
const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(200);

/// Run watch mode for `input`: register a parent-directory watch, run
/// `render` once immediately, then re-run it once per settled burst of
/// events that plausibly changed the file's content.
///
/// `render` runs once immediately — before any event is seen — so callers
/// that must do a one-time thing after the initial render (e.g. the
/// `--open` browser launch) guard that in their own callback state. Its
/// `Err` is ignored: a failed render must never end watch mode.
///
/// Errors propagate only from registration itself, before any render: a
/// watch we cannot deliver aborts up front rather than rendering once and
/// failing later. Once pumping, the loop returns `Ok(())` only when the
/// watcher channel dies; it otherwise runs until killed.
pub fn run<F: FnMut() -> Result<(), RenderError>>(
    input: &Path,
    render: &mut F,
) -> notify::Result<()> {
    // Watch the parent DIRECTORY rather than the file itself. Editors that
    // save atomically (write a temp file, then rename it over the target)
    // replace the target's inode; a watch on the file itself goes dead the
    // moment that happens because the inode/handle notify was watching is
    // gone. Watching the directory survives renames, deletes, and recreates
    // — we just have to filter directory events down to ones that touch our
    // target file.
    //
    // Resolve the target once, up front. Comparing later events against
    // this fixed absolute path — rather than re-canonicalizing each event's
    // path — matters because the target may momentarily not exist
    // mid-rename, which would make canonicalize fail and misclassify a
    // perfectly relevant event.
    //
    // Known limitation: canonicalizing a symlinked input means we watch the
    // TARGET's parent, so edits made through the link are seen, but an
    // editor atomically replacing the symlink itself is not. Watching both
    // parents isn't worth the complexity until someone actually hits this.
    let input_absolute = std::fs::canonicalize(input).unwrap_or_else(|_| {
        std::env::current_dir()
            .map(|cwd| cwd.join(input))
            .unwrap_or_else(|_| input.to_path_buf())
    });
    let watch_dir = input_absolute
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let target_file_name = input_absolute.file_name().map(|n| n.to_os_string());

    // Register the watch BEFORE the initial render so a save that lands
    // while that render runs is queued by the watcher rather than silently
    // lost; the pump below drains it and re-renders. Registration
    // failure aborts before any rendering, which is fine — the user asked
    // for a watch we cannot deliver.

    let (tx, rx) = channel();
    let mut watcher = recommended_watcher(tx)?;
    watcher.watch(&watch_dir, RecursiveMode::NonRecursive)?;

    let _ = render();

    // Still named after the file, not the directory: that's what the user
    // asked to watch, even though the underlying notify watch is scoped one
    // level up.
    println!("👀 Watching {:?} for changes... (Ctrl+C to stop)", input);

    pump(
        &rx,
        &watch_dir,
        target_file_name.as_deref(),
        render,
        DEFAULT_DEBOUNCE,
        &SystemClock,
    );
    Ok(())
}

/// The event pump: block for the next event, skip everything that isn't a
/// relevant change to the target file, drain/absorb bursts through the
/// trailing-edge debounce window, then render once per settled burst.
///
/// `debounce` and `clock` are the injectable timing seams (unit tests
/// script relative arrival times against a virtual clock; production uses
/// the system monotonic clock).
fn pump<R: EventRx, F: FnMut() -> Result<(), RenderError>, C: Clock>(
    rx: &R,
    watch_dir: &Path,
    target_file_name: Option<&OsStr>,
    render: &mut F,
    debounce: Duration,
    clock: &C,
) {
    loop {
        // Block for the next event. Anything that isn't a relevant,
        // content-changing event (wrong file, Access events, watcher
        // errors) is skipped without starting a debounce window.
        let event = match rx.recv() {
            Ok(Ok(event)) => event,
            Ok(Err(e)) => {
                eprintln!("⚠️  Watcher error: {}", e);
                continue;
            }
            Err(_) => return, // watcher/channel gone; nothing left to watch
        };

        if !is_relevant_event(&event, target_file_name, watch_dir) {
            continue;
        }

        // Drain/absorb further relevant events for a quiet window, then
        // render once. This covers direct writes, truncate+rewrite, and
        // atomic saves (temp file write followed by a rename onto the
        // target) uniformly: whichever event kinds the editor and platform
        // happen to emit (Create, Modify, Rename-as-Modify(Name), Remove
        // followed by a Create), each just extends the quiet window until
        // the burst settles. Only RELEVANT events extend the deadline:
        // sibling-file noise merely waits out the remaining window, so a
        // busy directory (a build churning artifacts next to the watched
        // file) cannot postpone the render indefinitely.
        let mut deadline = clock.now() + debounce;
        loop {
            let remaining = deadline.saturating_duration_since(clock.now());
            if remaining.is_zero() {
                break;
            }
            match rx.recv_timeout(remaining) {
                Ok(Ok(event)) => {
                    if is_relevant_event(&event, target_file_name, watch_dir) {
                        deadline = clock.now() + debounce;
                    }
                    // Irrelevant events neither extend nor cut the window.
                }
                Ok(Err(e)) => eprintln!("⚠️  Watcher error: {}", e),
                Err(_) => break, // quiet window elapsed (or channel closed)
            }
        }

        println!("🔁 File changed, re-rendering...");
        // A failed render (e.g. one that ran while the file was momentarily
        // absent mid-rename) must not end watch mode; its reporting already
        // happened inside the callback, so just loop and wait for the next
        // event.
        let _ = render();
    }
}

/// True if `event` plausibly changed the content of the file named
/// `target_file_name` inside `watch_dir`. We match by file name (and, when
/// notify reports one, by parent directory) rather than canonicalizing the
/// event's path, since the target can momentarily not exist mid-rename.
/// Access events (reads, permission-bit-only changes) are excluded; every
/// other kind (Create, Modify, Remove, Any, Other) is treated as a possible
/// content change so recreation-after-remove and atomic renames are all
/// covered by the same check.
fn is_relevant_event(event: &Event, target_file_name: Option<&OsStr>, watch_dir: &Path) -> bool {
    if matches!(event.kind, EventKind::Access(_)) {
        return false;
    }

    let Some(target_file_name) = target_file_name else {
        return false;
    };

    event.paths.iter().any(|p| {
        p.file_name() == Some(target_file_name)
            && match p.parent() {
                Some(parent) => parent == watch_dir,
                None => true,
            }
    })
}

/// One item on the watch event channel: a real event, or a watcher-level
/// error delivered as a channel payload (the queue keeps flowing either
/// way).
type EventItem = Result<Event, notify::Error>;

/// The receiving half of the watch event channel narrowed to the exact
/// shape the pump uses. Abstracted so unit tests script fake sequences.
trait EventRx {
    fn recv(&self) -> Result<EventItem, RecvError>;
    fn recv_timeout(&self, timeout: Duration) -> Result<EventItem, RecvTimeoutError>;
}

impl EventRx for mpsc::Receiver<EventItem> {
    fn recv(&self) -> Result<EventItem, RecvError> {
        mpsc::Receiver::recv(self)
    }

    fn recv_timeout(&self, timeout: Duration) -> Result<EventItem, RecvTimeoutError> {
        mpsc::Receiver::recv_timeout(self, timeout)
    }
}

/// Where the pump reads its clock. Production uses the system monotonic
/// clock; tests inject a virtual one.
trait Clock {
    fn now(&self) -> Instant;
}

struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::rc::Rc;

    use notify::event::{AccessKind, ModifyKind};

    /// Virtual monotonic clock shared with the fake channel: entries in a
    /// script declare how long after "now" their payload is ready.
    #[derive(Clone)]
    struct VirtualClock(Rc<Cell<Instant>>);

    impl VirtualClock {
        fn new() -> Self {
            Self(Rc::new(Cell::new(Instant::now())))
        }

        fn advance(&self, by: Duration) {
            self.0.set(self.0.get() + by);
        }
    }

    impl Clock for VirtualClock {
        fn now(&self) -> Instant {
            self.0.get()
        }
    }

    /// One scripted channel entry: a payload plus how long after the
    /// current virtual time it becomes available.
    struct Scripted {
        ready_after: Duration,
        payload: EventItem,
    }

    /// Fake channel half driving `pump` from a script. Events ready within
    /// the polled timeout are delivered on that poll; anything later goes
    /// past the window (timed out), exactly as a real channel would.
    struct FakeRx {
        clock: VirtualClock,
        script: RefCell<VecDeque<Scripted>>,
    }

    impl FakeRx {
        fn now_or_elapsed(&self, timeout: Duration) -> Result<EventItem, RecvTimeoutError> {
            let next = self.script.borrow().front().map(|s| s.ready_after);
            match next {
                Some(ready_after) if ready_after <= timeout => {
                    self.clock.advance(ready_after);
                    let scripted = self
                        .script
                        .borrow_mut()
                        .pop_front()
                        .expect("front checked above");
                    Ok(scripted.payload)
                }
                _ => {
                    self.clock.advance(timeout);
                    Err(RecvTimeoutError::Timeout)
                }
            }
        }
    }

    impl EventRx for FakeRx {
        fn recv(&self) -> Result<EventItem, RecvError> {
            match self.script.borrow_mut().pop_front() {
                Some(scripted) => {
                    self.clock.advance(scripted.ready_after);
                    Ok(scripted.payload)
                }
                None => Err(RecvError),
            }
        }

        fn recv_timeout(&self, timeout: Duration) -> Result<EventItem, RecvTimeoutError> {
            self.now_or_elapsed(timeout)
        }
    }

    fn event_ready_after(ready_after: u64, kind: EventKind, path: PathBuf) -> Scripted {
        Scripted {
            ready_after: Duration::from_millis(ready_after),
            payload: Ok(Event::new(kind).add_path(path)),
        }
    }

    fn watcher_error_after(ready_after: u64) -> Scripted {
        Scripted {
            ready_after: Duration::from_millis(ready_after),
            payload: Err(notify::Error::io(std::io::Error::other("watcher failed"))),
        }
    }

    /// A content-changing event for `name` inside `watch_dir`.
    fn modify(name: &str, watch_dir: &Path, ready_after: u64) -> Scripted {
        event_ready_after(
            ready_after,
            EventKind::Modify(ModifyKind::Any),
            watch_dir.join(name),
        )
    }

    /// A change to a file that is not the watched target.
    fn sibling(name: &str, watch_dir: &Path, ready_after: u64) -> Scripted {
        event_ready_after(
            ready_after,
            EventKind::Modify(ModifyKind::Any),
            watch_dir.join(name),
        )
    }

    fn access(watch_dir: &Path, ready_after: u64) -> Scripted {
        event_ready_after(
            ready_after,
            EventKind::Access(AccessKind::Any),
            watch_dir.join("sample.md"),
        )
    }

    /// Pump over a script, counting renders in the callback.
    fn pump_scripted(script: Vec<Scripted>) -> (usize, VirtualClock) {
        let clock = VirtualClock::new();
        let renders = Rc::new(Cell::new(0_usize));
        let rx = FakeRx {
            clock: clock.clone(),
            script: RefCell::new(script.into()),
        };
        pump(
            &rx,
            Path::new("/watch"),
            Some(OsStr::new("sample.md")),
            &mut || {
                renders.set(renders.get() + 1);
                Ok(())
            },
            Duration::from_millis(200),
            &clock,
        );
        (renders.get(), clock)
    }

    #[test]
    fn burst_collapses_into_one_render() {
        // Three events inside one window: one render of the final content.
        let (renders, _clock) = pump_scripted(vec![
            modify("sample.md", Path::new("/watch"), 50),
            modify("sample.md", Path::new("/watch"), 70),
            modify("sample.md", Path::new("/watch"), 60),
        ]);
        assert_eq!(renders, 1, "a settled burst renders exactly once");
    }

    #[test]
    fn burst_running_longer_than_window_still_renders_the_trailing_event() {
        // Events every 80ms outrun a 200ms quiet window for a while; no
        // render fires mid-burst — the trailing event is never dropped.
        let (renders, _clock) = pump_scripted(vec![
            modify("sample.md", Path::new("/watch"), 50),
            modify("sample.md", Path::new("/watch"), 80),
            modify("sample.md", Path::new("/watch"), 80),
            modify("sample.md", Path::new("/watch"), 80),
            modify("sample.md", Path::new("/watch"), 80),
        ]);
        assert_eq!(renders, 1, "a long burst settles into exactly one render");
    }

    #[test]
    fn event_after_the_window_renders_again() {
        // One burst, then quiet past the window, then another event: the
        // first event settles into render one; the late event is NOT
        // dropped — it becomes the next outer event and renders again.
        let (renders, _clock) = pump_scripted(vec![
            modify("sample.md", Path::new("/watch"), 50),
            modify("sample.md", Path::new("/watch"), 700),
        ]);
        assert_eq!(renders, 2, "events after the window are fresh triggers");
    }

    #[test]
    fn sibling_noise_renders_nothing() {
        // Sibling-file churn alone never triggers or extends a window.
        let (renders, _clock) = pump_scripted(vec![
            sibling("other.md", Path::new("/watch"), 50),
            sibling("build.log", Path::new("/watch"), 80),
        ]);
        assert_eq!(renders, 0, "no target file in any event, no render");
    }

    #[test]
    fn sibling_noise_waits_out_but_does_not_extend_the_window() {
        // A sibling event arriving well past the current window cannot
        // extend the deadline: the pump waits out only the remaining window
        // (bounded by one debounce) and renders; the sibling is then seen
        // as the next outer event, classified irrelevant, and skipped.
        let (renders, _clock) = pump_scripted(vec![
            modify("sample.md", Path::new("/watch"), 50),
            sibling("other.md", Path::new("/watch"), 1000),
        ]);
        assert_eq!(renders, 1, "the sibling burst never postponed the render");
    }

    #[test]
    fn access_events_are_never_relevant() {
        let (renders, _clock) = pump_scripted(vec![access(Path::new("/watch"), 50)]);
        assert_eq!(renders, 0, "Access events are excluded");
    }

    #[test]
    fn channel_death_before_any_event_ends_the_watch_gracefully() {
        let (renders, _clock) = pump_scripted(vec![]);
        assert_eq!(renders, 0);
    }

    #[test]
    fn channel_death_during_drain_renders_and_exits() {
        // An event lands, the window is open, the channel dies: the
        // pending burst still renders, then the watch ends.
        let (renders, _clock) = pump_scripted(vec![modify("sample.md", Path::new("/watch"), 50)]);
        assert_eq!(renders, 1);
    }

    #[test]
    fn watcher_error_storm_renders_nothing_and_survives() {
        let script: Vec<Scripted> = (0..50u64)
            .map(|i| watcher_error_after(i * 10 + 1))
            .collect();
        let (renders, _clock) = pump_scripted(script);
        assert_eq!(renders, 0, "watcher errors are not renders");
    }

    #[test]
    fn watcher_errors_inside_the_drain_do_not_extend_the_window() {
        // Errors arriving inside a window neither extend nor cut it: they
        // wait out the remaining window like any other noise.
        let script = vec![
            modify("sample.md", Path::new("/watch"), 50),
            watcher_error_after(60),
            watcher_error_after(60),
        ];
        let (renders, _clock) = pump_scripted(script);
        assert_eq!(renders, 1);
    }

    #[test]
    fn callback_errors_never_end_watch_mode() {
        // A render callback that reports a transient filesystem failure
        // (e.g. the file was absent mid-rename) must not end watch mode.
        let clock = VirtualClock::new();
        let renders = Rc::new(Cell::new(0_usize));
        let failures = Rc::new(Cell::new(0_usize));
        let rx = FakeRx {
            clock: clock.clone(),
            script: RefCell::new(
                vec![
                    // Two bursts separated past the window: render One and
                    // render two must BOTH happen for the errors never to
                    // have ended watch mode.
                    modify("sample.md", Path::new("/watch"), 50),
                    modify("sample.md", Path::new("/watch"), 700),
                ]
                .into(),
            ),
        };
        let renders_flag = renders.clone();
        let failures_flag = failures.clone();
        pump(
            &rx,
            Path::new("/watch"),
            Some(OsStr::new("sample.md")),
            &mut || {
                renders_flag.set(renders_flag.get() + 1);
                failures_flag.set(failures_flag.get() + 1);
                Err(RenderError::Read {
                    path: PathBuf::from("/watch/sample.md"),
                    source: std::io::Error::other("transient"),
                })
            },
            Duration::from_millis(200),
            &clock,
        );
        assert_eq!(renders.get(), 2, "every burst was rendered");
        assert_eq!(failures.get(), 2, "every callback Err surfaced to it");
    }

    #[test]
    fn relevance_requires_name_and_parent_match() {
        let watch_dir = Path::new("/watch");
        let named = Some(OsStr::new("sample.md"));

        let same_dir =
            Event::new(EventKind::Modify(ModifyKind::Any)).add_path(watch_dir.join("sample.md"));
        assert!(is_relevant_event(&same_dir, named, watch_dir));

        // A bare relative path has parent Some("") — never equal to a real
        // watch dir, so it is not relevant. Real events carry absolute
        // paths, so this only matters to the filter's strictness, not to
        // production labeling.
        let bare_relative =
            Event::new(EventKind::Modify(ModifyKind::Any)).add_path(PathBuf::from("sample.md"));
        assert!(!is_relevant_event(&bare_relative, named, watch_dir));

        let other_dir = Event::new(EventKind::Modify(ModifyKind::Any))
            .add_path(PathBuf::from("/elsewhere/sample.md"));
        assert!(!is_relevant_event(&other_dir, named, watch_dir));

        let other_name =
            Event::new(EventKind::Modify(ModifyKind::Any)).add_path(watch_dir.join("other.md"));
        assert!(!is_relevant_event(&other_name, named, watch_dir));

        // No resolvable target: nothing is ever relevant.
        let none: Option<&OsStr> = None;
        assert!(!is_relevant_event(&same_dir, none, watch_dir));
    }
}
