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
//! - `--setup`               Show a cautious first-run setup
//!
//! Without `--watch`, the tool converts once and exits.
//!
//! ## Credits
//!
//! Forked with gratitude from Hafiz Ali Raza's original Markdown-to-HTML CLI.
//! Bundles [simple.css](https://simplecss.org/) (© 2020 Kev Quirk, MIT).

use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use clap::Parser;
use mdo_cli::{
    convert_with_css_override, derive_output, file_manager, launch_browser, open_setup_sample,
    temp_output_for,
};
use notify::{recommended_watcher, EventKind, RecursiveMode, Watcher};

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
        loop {
            print!("Install Open as HTML file-manager integration now? [Y/n] ");
            io::stdout().flush()?;

            let mut answer = String::new();
            if io::stdin().read_line(&mut answer)? == 0 {
                return Ok(());
            }

            match answer.trim().to_ascii_lowercase().as_str() {
                "" | "y" | "yes" => {
                    match file_manager::install(false) {
                        Ok(()) => {
                            println!("Integration installed. No default app was changed by mdo.");
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
                "n" | "no" => {
                    println!(
                        "No changes made. Run `mdo --install-file-manager` whenever you are ready."
                    );
                    break;
                }
                "?" | "h" | "help" => {
                    println!(
                        "This adds an \"Open as HTML\" action for Markdown files in your file manager. \
                         It is reversible with `mdo --uninstall-file-manager`."
                    );
                }
                _ => println!("Please answer y or n."),
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
        (None, false) => (derive_output(&input), false),
    };

    let converted = convert_with_css_override(
        &input,
        &output,
        args.bare,
        args.unsafe_html,
        private_output,
        args.css.as_deref(),
    );

    if args.open && converted {
        match launch_browser(&output) {
            Ok(()) => println!("🌐 Opened {:?} in default browser", output),
            Err(e) => eprintln!("⚠️  Failed to launch browser: {}", e),
        }
    }

    if !args.watch {
        // Exit non-zero on a failed one-shot render so scripts and the docs
        // pipeline can detect errors. In watch mode we keep running so the
        // next successful edit re-renders.
        if converted {
            return Ok(());
        }
        std::process::exit(1);
    }

    let (tx, rx) = channel();
    let mut watcher = recommended_watcher(tx)?;
    watcher.watch(&input, RecursiveMode::NonRecursive)?;

    println!("👀 Watching {:?} for changes... (Ctrl+C to stop)", input);

    // Simple debounce: ignore events that fire within DEBOUNCE_MS of the last render.
    const DEBOUNCE: Duration = Duration::from_millis(200);
    let mut last_render = Instant::now() - DEBOUNCE;

    loop {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(Ok(event)) => {
                if matches!(event.kind, EventKind::Modify(_)) {
                    if last_render.elapsed() < DEBOUNCE {
                        continue;
                    }
                    println!("🔁 File changed, re-rendering...");
                    convert_with_css_override(
                        &input,
                        &output,
                        args.bare,
                        args.unsafe_html,
                        private_output,
                        args.css.as_deref(),
                    );
                    last_render = Instant::now();
                }
            }
            Ok(Err(e)) => eprintln!("⚠️  Watcher error: {}", e),
            Err(_) => {} // timeout
        }
    }
}
