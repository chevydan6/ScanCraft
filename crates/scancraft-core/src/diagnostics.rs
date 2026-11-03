//! Optional bounded stderr notes. Required results and cleanup never use this channel.
use std::{
    fmt,
    io::{self, Write},
    sync::atomic::{AtomicU64, Ordering},
};
const MAX_LINE: usize = 4096;
const MAX_NOTES: u64 = 1024 * 1024;
static NOTE_BYTES: AtomicU64 = AtomicU64::new(0);
struct Line {
    text: String,
    truncated: bool,
}
impl fmt::Write for Line {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let mut n = text
            .len()
            .min((MAX_LINE - 16).saturating_sub(self.text.len()));
        while !text.is_char_boundary(n) {
            n -= 1;
        }
        self.text.push_str(&text[..n]);
        if n != text.len() {
            self.truncated = true;
            return Err(fmt::Error);
        }
        Ok(())
    }
}
fn line(args: fmt::Arguments<'_>) -> Vec<u8> {
    let mut out = Line {
        text: String::new(),
        truncated: false,
    };
    let _ = fmt::write(&mut out, args);
    if out.truncated {
        out.text.push_str(" [truncated]");
    }
    out.text.push('\n');
    out.text.into_bytes()
}
fn write(output: &mut impl Write, args: fmt::Arguments<'_>, budget: &AtomicU64) -> io::Result<()> {
    let bytes = line(args);
    if budget
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
            n.checked_add(bytes.len() as u64)
                .filter(|&n| n <= MAX_NOTES)
        })
        .is_ok()
    {
        output.write_all(&bytes)?;
    }
    Ok(())
}
/// Best effort: diagnostic write failures are ignored, never promoted to a panic
/// before restoration or final IPC. Each note is <=4096 UTF-8 bytes (including
/// truncation marker/newline), with <=1MiB attempted per process. Suppressed notes
/// are not retried. Other stderr writers/panic hooks are outside these bounds.
/// A blocking sink can still block; this is not an asynchronous logger/deadline.
pub fn note(args: fmt::Arguments<'_>) {
    let _ = write(&mut io::stderr().lock(), args, &NOTE_BYTES);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lines_preserve_utf8_and_mark_truncation() {
        let data = "é".repeat(MAX_LINE);
        let bytes = line(format_args!("context: {data}"));
        assert!(bytes.len() <= MAX_LINE);
        assert!(
            std::str::from_utf8(&bytes)
                .unwrap()
                .ends_with(" [truncated]\n")
        );
        assert_eq!(line(format_args!("completed {}", 7)), b"completed 7\n");
    }
    #[test]
    fn budget_is_exact_and_failed_writes_are_not_retried() {
        let budget = AtomicU64::new(MAX_NOTES - 2);
        let mut bytes = Vec::new();
        write(&mut bytes, format_args!("x"), &budget).unwrap();
        assert_eq!(budget.load(Ordering::Relaxed), MAX_NOTES);
        write(&mut bytes, format_args!("omitted"), &budget).unwrap();
        assert_eq!(bytes, b"x\n");
        struct Failed;
        impl Write for Failed {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let budget = AtomicU64::new(0);
        assert!(write(&mut Failed, format_args!("failed"), &budget).is_err());
        assert_eq!(budget.load(Ordering::Relaxed), 7);
        // Partial or failed diagnostics do not acquire authority over caller work.
        let outcome = std::panic::catch_unwind(|| {
            write(&mut Failed, format_args!("still optional"), &budget)
        });
        assert!(outcome.unwrap().is_err());
    }
}
