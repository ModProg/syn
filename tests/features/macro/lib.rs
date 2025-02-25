#![allow(clippy::toplevel_ref_arg, clippy::uninlined_format_args)]

use proc_macro::TokenStream;
use std::io::Write;
use std::process;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

const DEBUG_NOTE: &str = ":  use --release
  Syn's test suite has some tests that run on every source file
  and test case in the rust-lang/rust repo, which can be pretty
  slow in debug mode. Consider running cargo test with `--release`
  to speed things up.";

const FEATURES_ERROR: &str = ":  use --all-features
  Syn's test suite normally only works with all-features enabled.
  Run again with `--all-features`, or run with `--features test`
  to bypass this check.";

#[proc_macro]
pub fn check(_input: TokenStream) -> TokenStream {
    let ref mut stderr = StandardStream::stderr(ColorChoice::Auto);
    let mut needs_newline = true;

    if cfg!(debug_assertions) {
        let yellow = ColorSpec::new().set_fg(Some(Color::Yellow)).clone();
        _ = writeln!(stderr);
        _ = stderr.set_color(yellow.clone().set_bold(true));
        _ = write!(stderr, "NOTE");
        for line in DEBUG_NOTE.lines() {
            _ = stderr.set_color(&yellow);
            _ = writeln!(stderr, "{}", line);
        }
        _ = stderr.reset();
        _ = writeln!(stderr);
        needs_newline = false;
    }

    if cfg!(not(feature = "all-features")) {
        let red = ColorSpec::new().set_fg(Some(Color::Red)).clone();
        if needs_newline {
            _ = writeln!(stderr);
        }
        _ = stderr.set_color(red.clone().set_bold(true));
        _ = write!(stderr, "ERROR");
        for line in FEATURES_ERROR.lines() {
            _ = stderr.set_color(&red);
            _ = writeln!(stderr, "{}", line);
        }
        _ = stderr.reset();
        _ = writeln!(stderr);
        process::exit(1);
    }

    TokenStream::new()
}
