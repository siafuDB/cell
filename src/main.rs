//! The `cell` command.
//!
//! ```sh
//! cell check FILE.cell    # validate one cell; exits 1 if it has errors
//! cell render FILE.cell   # print its text with facts filled in
//! ```

#![forbid(unsafe_code)]

use std::process::ExitCode;

use siafudb_cell::{parse, render, validate};

const USAGE: &str = "usage: cell check FILE.cell | cell render FILE.cell";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [cmd, file] = args.as_slice() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    if cmd != "check" && cmd != "render" {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let cell = match std::fs::read_to_string(file)
        .map_err(|e| format!("{file}: {e}"))
        .and_then(|t| parse(&t).map_err(|e| format!("{file}: {e}")))
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    if cmd == "render" {
        return match render(&cell, "") {
            Ok(t) => {
                println!("{t}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        };
    }
    let report = validate(&cell, None);
    for w in &report.warnings {
        println!("warning: {w}");
    }
    for e in &report.errors {
        println!("error: {e}");
    }
    if report.errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
