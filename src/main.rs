use std::process::ExitCode;

fn main() -> ExitCode {
    ExitCode::from(stx::cli::run(std::env::args_os()))
}
