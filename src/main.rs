//! # mdo
//!
//! `mdo` is a small command-line tool that converts Markdown (`.md`) files to HTML.
//!
//! By default it produces a complete, HTML5-compliant document styled with
//! [simple.css](https://simplecss.org/) (vendored at build time, no network access at runtime).
//!
//! ## Usage
//!
//! ```sh
//! mdo [OPTIONS] <INPUT>
//! ```
//!
//! If no output path is given, the output is written next to the input with
//! the extension changed to `.html` (e.g. `foo.md` → `foo.html`). Existing
//! files are overwritten.
//!
//! Options:
//! - `-o, --output <FILE>`  Write to `<FILE>` instead of the derived name
//! - `-w, --watch`          Keep running and re-render on file changes
//! - `-b, --bare`           Emit only the HTML fragment (no `<html>`, `<head>`, `<body>`, no CSS)
//! - `--css <FILE>`         Append custom CSS after mdo's default styling
//! - `--unsafe-html`        Preserve raw HTML from the Markdown source
//! - `-v, --verbose`        Report render-workflow timings on stderr
//! - `--setup`               Show a cautious first-run setup
//!
//! Without `--watch`, the tool converts once and exits.
//!
//! ## Credits
//!
//! Forked with gratitude from Hafiz Ali Raza's original Markdown-to-HTML CLI.
//! Bundles [simple.css](https://simplecss.org/) (© 2020 Kev Quirk, MIT).

use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use clap::Parser;
use mdo_cli::{
    convert, file_manager, launch_browser, open_setup_sample, temp_output_for, watch,
    ConvertOutcome, ConvertRequest, RenderError, StageTimings,
};

/// Markdown to HTML converter. Converts once by default; pass --watch to keep watching.
#[derive(Parser)]
#[command(name = "mdo", author, version, about)]
struct Cli {
    /// Input Markdown file
    input: Option<PathBuf>,

    /// Output HTML file (defaults to <input>.html alongside the input,
    /// or to a temp directory when --open is used). Existing files are overwritten.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Watch the input file and re-render on every change
    #[arg(short, long)]
    watch: bool,

    /// Emit only the HTML fragment (no <html>, <head>, <body>, no CSS)
    #[arg(short, long)]
    bare: bool,

    /// Append a custom CSS file after mdo's default styling
    #[arg(long, value_name = "FILE")]
    css: Option<PathBuf>,

    /// Preserve raw HTML from the Markdown source instead of sanitizing it
    #[arg(long)]
    unsafe_html: bool,

    /// Report timing diagnostics for the render workflow (read, markdown,
    /// sanitize, assemble, write, total) on stderr. The generated HTML is
    /// unchanged.
    #[arg(short, long)]
    verbose: bool,

    /// Render to a temp directory and launch the system default browser.
    /// The source folder is left untouched unless --output is given.
    #[arg(long)]
    open: bool,

    /// Show a first-run setup with safe next steps for new users.
    #[arg(long)]
    setup: bool,

    /// Install per-user file-manager integration for Markdown files.
    ///
    /// On Windows this registers Open as HTML with Explorer. On Linux this
    /// writes an XDG desktop entry and icon.
    #[arg(long)]
    install_file_manager: bool,

    /// Remove per-user file-manager integration installed by mdo.
    #[arg(long)]
    uninstall_file_manager: bool,

    /// When installing on Linux, make Open as HTML the default Markdown
    /// handler. Windows still requires choosing the default app interactively.
    #[arg(long)]
    set_default: bool,
}

fn setup_is_interactive() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

/// What the single setup decision resolved to. Setup stays one clear choice
/// per run: install (or reinstall) the integration, or leave things unchanged.
#[derive(Debug, PartialEq, Eq)]
enum SetupIntegrationChoice {
    Install,
    LeaveUnchanged,
    Help,
    Invalid,
}

/// The prompt adapts to the cheap owned-state check: an already-configured
/// user is offered leave-unchanged (default) or reinstall, instead of being
/// greeted like a first-time installer.
fn setup_integration_prompt(already_installed: bool) -> &'static str {
    if already_installed {
        "Reinstall Open as HTML file-manager integration? [y/N] "
    } else {
        "Install Open as HTML file-manager integration now? [Y/n] "
    }
}

fn setup_integration_choice(answer: &str, already_installed: bool) -> SetupIntegrationChoice {
    match answer.trim().to_ascii_lowercase().as_str() {
        "" => {
            if already_installed {
                SetupIntegrationChoice::LeaveUnchanged
            } else {
                SetupIntegrationChoice::Install
            }
        }
        "y" | "yes" => SetupIntegrationChoice::Install,
        "n" | "no" => SetupIntegrationChoice::LeaveUnchanged,
        "?" | "h" | "help" => SetupIntegrationChoice::Help,
        _ => SetupIntegrationChoice::Invalid,
    }
}

fn print_landing_page() {
    println!(
        "\
Open Markdown as HTML.

  mdo FILE.md          create FILE.html
  mdo --open FILE.md   open rendered HTML in your browser
  mdo --setup          set up file-manager integration
  mdo --help           show all options"
    );
}

fn print_first_run_setup(can_install_file_manager: bool) {
    println!(
        "\
Welcome to mdo.

mdo turns Markdown files into standalone HTML and can open the result in your
default browser without leaving generated files beside your notes.

Try these when you are ready:

  mdo notes.md                 render notes.html beside notes.md
  mdo --open notes.md          render to a temp path and open the browser
  mdo --watch notes.md         keep notes.html updated while you edit
  mdo --help                   show every command-line option
"
    );

    if can_install_file_manager {
        println!(
            "\
Optional desktop integration:

  mdo --install-file-manager   add an \"Open as HTML\" file-manager action
  mdo --uninstall-file-manager remove that integration later

The installer is per-user. It does not need admin rights and does not change
your default Markdown app unless you opt into the platform-specific default
handler flow.
"
        );
    } else {
        println!(
            "\
Optional desktop integration:

This platform does not have a built-in mdo installer yet. You can still wire
your file manager to run `mdo --open <file>`; see the README for platform
recipes.
"
        );
    }
}

fn run_first_run_setup() -> io::Result<()> {
    let interactive = setup_is_interactive();
    let can_install_file_manager = cfg!(any(target_os = "linux", target_os = "windows"));

    print_first_run_setup(can_install_file_manager);

    if !interactive {
        return Ok(());
    }

    if can_install_file_manager {
        let already_installed = file_manager::integration_installed();
        if already_installed {
            println!("Open as HTML file-manager integration is already installed for this user.");
        }
        loop {
            print!("{}", setup_integration_prompt(already_installed));
            io::stdout().flush()?;

            let mut answer = String::new();
            if io::stdin().read_line(&mut answer)? == 0 {
                return Ok(());
            }

            match setup_integration_choice(&answer, already_installed) {
                SetupIntegrationChoice::Install => {
                    match file_manager::install(false) {
                        Ok(()) => {
                            if already_installed {
                                println!(
                                    "Integration reinstalled. No default app was changed by mdo."
                                );
                            } else {
                                println!(
                                    "Integration installed. No default app was changed by mdo."
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!("Could not install file-manager integration: {e}");
                            println!("No default app was changed by mdo.");
                            wait_for_setup_close()?;
                            return Err(e);
                        }
                    }
                    break;
                }
                SetupIntegrationChoice::LeaveUnchanged => {
                    if already_installed {
                        println!(
                            "Left unchanged. Run `mdo --uninstall-file-manager` if you ever want to remove it."
                        );
                    } else {
                        println!(
                            "No changes made. Run `mdo --install-file-manager` whenever you are ready."
                        );
                    }
                    break;
                }
                SetupIntegrationChoice::Help => {
                    if already_installed {
                        println!(
                            "Reinstalling rewrites mdo's own per-user registration in place. \
                             It is reversible with `mdo --uninstall-file-manager`."
                        );
                    } else {
                        println!(
                            "This adds an \"Open as HTML\" action for Markdown files in your file manager. \
                             It is reversible with `mdo --uninstall-file-manager`."
                        );
                    }
                }
                SetupIntegrationChoice::Invalid => println!("Please answer y or n."),
            }
        }
    }

    wait_for_setup_close()?;
    println!("Opening a welcome sample in your browser...");
    match open_setup_sample() {
        Ok(()) => println!("🌐 Opened welcome sample in default browser"),
        Err(e) => eprintln!("⚠️  Failed to open welcome sample: {e}"),
    }
    Ok(())
}

fn wait_for_setup_close() -> io::Result<()> {
    println!("Press Enter to close this setup.");
    let mut ignored = String::new();
    io::stdin().read_line(&mut ignored)?;
    Ok(())
}

fn main() -> notify::Result<()> {
    let args = Cli::parse();

    // Parse first so Clap retains ownership of --help, --version, and errors.
    // Only the truly argument-free invocation gets the short landing page.
    if std::env::args_os().len() == 1 {
        print_landing_page();
        return Ok(());
    }

    if args.install_file_manager && args.uninstall_file_manager {
        eprintln!("❌ Choose only one of --install-file-manager or --uninstall-file-manager");
        std::process::exit(2);
    }

    if args.setup {
        if args.input.is_some()
            || args.output.is_some()
            || args.watch
            || args.bare
            || args.css.is_some()
            || args.unsafe_html
            || args.verbose
            || args.open
            || args.install_file_manager
            || args.uninstall_file_manager
            || args.set_default
        {
            eprintln!("❌ --setup cannot be combined with render or integration options");
            std::process::exit(2);
        }

        if let Err(e) = run_first_run_setup() {
            eprintln!("❌ Setup failed: {e}");
            std::process::exit(1);
        }

        return Ok(());
    }

    if args.set_default && !args.install_file_manager {
        eprintln!("❌ --set-default can only be used with --install-file-manager");
        std::process::exit(2);
    }

    if args.bare && args.css.is_some() {
        eprintln!("❌ --css cannot be combined with --bare because bare output emits no CSS");
        std::process::exit(2);
    }

    if args.install_file_manager || args.uninstall_file_manager {
        if args.input.is_some()
            || args.output.is_some()
            || args.watch
            || args.bare
            || args.css.is_some()
            || args.unsafe_html
            || args.verbose
            || args.open
            || args.setup
        {
            eprintln!(
                "❌ File-manager integration commands cannot be combined with render options"
            );
            std::process::exit(2);
        }

        let result = if args.install_file_manager {
            file_manager::install(args.set_default)
        } else {
            file_manager::uninstall()
        };

        if let Err(e) = result {
            eprintln!("❌ File-manager integration failed: {e}");
            std::process::exit(1);
        }

        return Ok(());
    }

    let input = match args.input {
        Some(input) => input,
        None => {
            eprintln!("❌ Missing input Markdown file");
            eprintln!("Run `mdo --help` for usage or `mdo --setup` for a first-run guide.");
            std::process::exit(2);
        }
    };

    // Output precedence:
    //   1. explicit --output           (always wins)
    //   2. --open without --output     → temp dir (don't pollute the source folder)
    //   3. neither                     → next to the input
    let (output, private_output) = match (args.output.clone(), args.open) {
        (Some(p), _) => (p, false),
        (None, true) => match temp_output_for(&input) {
            Ok(path) => (path, true),
            Err(e) => {
                eprintln!("❌ Failed to prepare temp output directory: {}", e);
                std::process::exit(1);
            }
        },
        (None, false) => (input.with_extension("html"), false),
    };

    if args.watch {
        // Watch's contract: register before rendering (a save that lands
        // while the initial render runs is queued by the watcher rather
        // than silently lost), run the render callback once immediately,
        // then pump. In watch mode we deliberately do NOT apply the one-shot
        // exit-code check below: a one-time --open failure must not end
        // watch mode, since the whole point is to keep re-rendering on
        // future edits.
        let mut pending_open_launch = args.open;
        return watch::run(&input, &mut || {
            let result = render_and_report(
                &input,
                &output,
                args.bare,
                args.unsafe_html,
                args.css.as_deref(),
                private_output,
                args.verbose,
            );
            if pending_open_launch {
                // The launch hook is a one-time post-initial-render duty
                // (the pump runs the callback immediately): consume the
                // pending flag on the first callback whether or not that
                // render succeeded, exactly as the pre-module flow did.
                if result.is_ok() {
                    launch_open_output(&output, args.verbose);
                }
                pending_open_launch = false;
            }
            result.map(|_| ())
        });
    }

    let converted = render_and_report(
        &input,
        &output,
        args.bare,
        args.unsafe_html,
        args.css.as_deref(),
        private_output,
        args.verbose,
    )
    .is_ok();

    // Track whether --open promised a browser launch and failed to deliver
    // one, so the one-shot exit code below can be honest about it.
    let mut open_launch_failed = false;
    if args.open && converted {
        open_launch_failed = launch_open_output(&output, args.verbose);
    }

    // Exit non-zero on a failed one-shot render so scripts and the docs
    // pipeline can detect errors. A failed --open browser launch is the
    // same kind of broken promise even though the render itself succeeded:
    // `mdo --open` only fully succeeds when the file is both rendered AND
    // opened, so it exits non-zero here too.
    if converted && !open_launch_failed {
        return Ok(());
    }
    std::process::exit(1);
}

/// Run one render request and report it. All rendering output lives behind
/// `report_render`, so the one-shot path and watch-mode re-renders report
/// identically. The typed outcome still comes back: the watch callback
/// feeds its `Err` into `watch::run`, which must not end watch mode on a
/// transient filesystem state mid-rename.
fn render_and_report(
    input: &Path,
    output: &Path,
    bare: bool,
    unsafe_html: bool,
    css_override: Option<&Path>,
    private_output: bool,
    verbose: bool,
) -> Result<(), RenderError> {
    let outcome = convert(ConvertRequest {
        input,
        output,
        bare,
        unsafe_html,
        private_output,
        css_override,
    });
    report_render(input, output, &outcome, verbose);
    outcome.map(|_| ())
}

/// `--open`'s post-render launch hook: initiate the browser launch and
/// report it. Returns whether the launch failed to deliver on the promise —
/// the one-shot exit code owes this honesty; watch mode ignores failures
/// and keeps watching to re-render future edits.
fn launch_open_output(output: &Path, verbose: bool) -> bool {
    // Browser launch is reported separately from the render workflow:
    // launch_browser only initiates the launch (fire-and-forget), so
    // this measures initiation, not the browser starting up.
    let launch_start = Instant::now();
    match launch_browser(output) {
        Ok(()) => {
            if verbose {
                eprintln!(
                    "⏱  Browser launch initiated in {:.3} ms",
                    launch_start.elapsed().as_secs_f64() * 1000.0
                );
            }
            println!("🌐 Opened {:?} in default browser", output);
            false
        }
        Err(e) => {
            // The render already succeeded and nothing removes the
            // rendered file on a launch failure, so point the user at it
            // instead of silently downgrading this to a warning.
            eprintln!("❌ Failed to launch browser: {}", e);
            eprintln!("   Rendered output is available at {:?}", output);
            true
        }
    }
}

/// Print one render workflow's result. The render pipeline prints nothing
/// itself, so all rendering output lives here: success relocates v0.6's
/// single ✅ line, failure prints the historical ❌ line for that failure
/// site, and the verbose timing report goes to STDERR only. Returns whether
/// the render succeeded, so exit-code policy stays local to `main`.
fn report_render(
    input: &Path,
    output: &Path,
    result: &Result<ConvertOutcome, RenderError>,
    verbose: bool,
) -> bool {
    match result {
        Ok(outcome) => {
            println!("✅ Converted {:?} → {:?}", input, output);
            if verbose {
                report_render_timings(input, &outcome.timings);
            }
            true
        }
        Err(err) => {
            print_render_failure(err);
            false
        }
    }
}

/// Print a typed render failure with the exact line the library used to
/// print at each failure site, so CLI output stays byte-identical.
fn print_render_failure(err: &RenderError) {
    match err {
        RenderError::Read { path, source } => eprintln!("❌ Failed to read {:?}: {}", path, source),
        RenderError::ReadCssOverride { path, source } => {
            eprintln!("❌ Failed to read CSS override {:?}: {}", path, source)
        }
        RenderError::CreateOutputDir { path, source } => {
            eprintln!("❌ Failed to create {:?}: {}", path, source)
        }
        RenderError::Write { path, source } => {
            eprintln!("❌ Failed to write to {:?}: {}", path, source)
        }
    }
}

/// Print per-stage and total timings for a completed render to STDERR only.
/// Performance diagnostics belong in the terminal: they never touch stdout
/// or the generated HTML (v0.6 quiet output). `total` is the wall-clock time
/// of the whole workflow, so it is always at least the sum of the stages.
fn report_render_timings(input: &Path, timings: &StageTimings) {
    let ms = |d: Duration| d.as_secs_f64() * 1000.0;
    eprintln!("⏱  Render workflow for {:?}:", input);
    eprintln!("   read     {:>10.3} ms", ms(timings.read));
    eprintln!("   markdown {:>10.3} ms", ms(timings.markdown));
    eprintln!("   sanitize {:>10.3} ms", ms(timings.sanitize));
    eprintln!("   assemble {:>10.3} ms", ms(timings.assemble));
    eprintln!("   write    {:>10.3} ms", ms(timings.write));
    eprintln!("   total    {:>10.3} ms", ms(timings.total));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_time_setup_defaults_to_install() {
        assert_eq!(
            setup_integration_choice("", false),
            SetupIntegrationChoice::Install
        );
        assert_eq!(
            setup_integration_choice("y\n", false),
            SetupIntegrationChoice::Install
        );
        assert_eq!(
            setup_integration_choice("no", false),
            SetupIntegrationChoice::LeaveUnchanged
        );
    }

    #[test]
    fn already_installed_setup_defaults_to_leave_unchanged() {
        assert_eq!(
            setup_integration_choice("", true),
            SetupIntegrationChoice::LeaveUnchanged
        );
        assert_eq!(
            setup_integration_choice("n\n", true),
            SetupIntegrationChoice::LeaveUnchanged
        );
        assert_eq!(
            setup_integration_choice("YES", true),
            SetupIntegrationChoice::Install
        );
    }

    #[test]
    fn setup_answers_share_help_and_invalid_handling() {
        for already_installed in [false, true] {
            assert_eq!(
                setup_integration_choice("?", already_installed),
                SetupIntegrationChoice::Help
            );
            assert_eq!(
                setup_integration_choice("help", already_installed),
                SetupIntegrationChoice::Help
            );
            assert_eq!(
                setup_integration_choice("maybe", already_installed),
                SetupIntegrationChoice::Invalid
            );
        }
    }

    #[test]
    fn setup_prompt_matches_detected_state() {
        assert!(setup_integration_prompt(false).starts_with("Install "));
        assert!(setup_integration_prompt(false).contains("[Y/n]"));
        assert!(setup_integration_prompt(true).starts_with("Reinstall "));
        assert!(setup_integration_prompt(true).contains("[y/N]"));
    }
}
