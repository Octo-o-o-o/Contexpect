fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(u8::try_from(ctxpect_cli::run(std::env::args_os())).unwrap_or(1))
}
