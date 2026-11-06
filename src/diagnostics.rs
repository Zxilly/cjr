//! Best-effort stderr reporting. A closed diagnostic pipe must never panic.
use std::fmt::Arguments;
use std::io::Write;

pub(crate) fn emit(message: Arguments<'_>) {
    let _ = writeln!(std::io::stderr().lock(), "{message}");
}
