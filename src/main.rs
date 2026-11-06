fn main() {
    let action = cjr::cli::parse(std::env::args_os().collect()).unwrap_or_else(|e| e.exit());
    match cjr::app::execute(action) {
        Ok(outcome) => outcome.terminate(),
        Err(error) => {
            use std::io::Write;
            if matches!(&error, cjr::Error::System(e) if e.kind() == std::io::ErrorKind::BrokenPipe)
            {
                std::process::exit(0);
            }
            let _ = writeln!(std::io::stderr().lock(), "cjr: {error}");
            std::process::exit(1);
        }
    }
}
