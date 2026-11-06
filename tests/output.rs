use std::io::{self, Write};

struct ClosedPipe;
impl Write for ClosedPipe {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::ErrorKind::BrokenPipe.into())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn report_write_failure_is_returned_without_panicking() {
    let result =
        cjr::app::execute_with_output(cjr::cli::Action::List { json: true }, &mut ClosedPipe);
    assert!(matches!(result, Err(cjr::Error::System(e)) if e.kind() == io::ErrorKind::BrokenPipe));
}
