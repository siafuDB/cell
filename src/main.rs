//! The `cell` command. Run `cell help` for every command, or see [`cli`].
//!
//! ```sh
//! cell check FILE.cell                  # validate one cell; exits 1 if it has errors
//! cell render FILE.cell                 # print its text with facts filled in
//! cell seal FILE.cell --key NAME.key    # set cell:hash and sign it
//! ```

#![forbid(unsafe_code)]

mod cli;

use std::process::ExitCode;

fn main() -> ExitCode {
    // nosemgrep: rust.lang.security.args.args -- argv[0] is skipped; the subcommand and file path are not used for any security decision.
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = cli::run(
        &args,
        &mut cli::Io {
            stdin: &mut std::io::stdin().lock(),
            stdout: &mut std::io::stdout().lock(),
            stderr: &mut std::io::stderr().lock(),
        },
    );
    ExitCode::from(code)
}
