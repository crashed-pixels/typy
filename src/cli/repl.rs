use super::session::Session;
use std::io::{self, BufRead, Write};

/// The REPL owns buffering and prompts; evaluation lives in Session.
/// Injected streams make EOF, continuation handling and recovery testable.
pub(super) fn run(
    input: &mut impl BufRead,
    output: &mut impl Write,
    errors: &mut impl Write,
    session: &mut Session,
) -> io::Result<()> {
    writeln!(output, "=== TyPy (v {}) ===", env!("CARGO_PKG_VERSION"))?;
    let mut pending = None;
    let mut eof = false;
    while !eof {
        let mut source = String::new();
        let mut in_block = false;
        loop {
            let line = if let Some(line) = pending.take() {
                line
            } else {
                write!(output, "{}", if in_block { "... " } else { ">>> " })?;
                output.flush()?;
                let mut line = String::new();
                input.read_line(&mut line)?;
                line
            };
            if line.is_empty() {
                eof = true;
                writeln!(output)?;
                break;
            }
            if in_block && ends_block(&line) {
                if !line.trim().is_empty() {
                    pending = Some(line);
                }
                break;
            }
            source.push_str(&line);
            if !in_block {
                if line.trim_end().ends_with(':') {
                    in_block = true;
                } else {
                    break;
                }
            }
        }
        if !source.trim().is_empty()
            && let Err(error) = session.execute(&source, output)
        {
            writeln!(errors, "{error}")?;
        }
    }
    Ok(())
}

fn ends_block(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty()
        || (!line.starts_with([' ', '\t']) && trimmed != "else:" && !trimmed.starts_with("elif "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn functions_and_classes_work_with_consecutive_blocks_and_eof() {
        let source = "def twice(n: int) -> int:\n    return n + n\nclass Boxed:\n    x: int = 21\n    def get(self) -> int:\n        return twice(self.x)\nb = Boxed()\nb.get()\n";
        let mut output = Vec::new();
        let mut errors = Vec::new();
        run(
            &mut Cursor::new(source),
            &mut output,
            &mut errors,
            &mut Session::new(false),
        )
        .unwrap();
        assert!(errors.is_empty());
        assert!(String::from_utf8(output).unwrap().contains("42\n"));
    }
    #[test]
    fn type_errors_do_not_break_a_persistent_repl() {
        let mut output = Vec::new();
        let mut errors = Vec::new();
        run(
            &mut Cursor::new("def f(x: int) -> int:\n    return x\n\nf(True)\nf(42)\n"),
            &mut output,
            &mut errors,
            &mut Session::new(false),
        )
        .unwrap();
        assert!(String::from_utf8(errors).unwrap().contains("TypeError"));
        assert!(String::from_utf8(output).unwrap().contains("42\n"));
    }
    #[test]
    fn pending_function_is_defined_at_eof() {
        let mut output = Vec::new();
        let mut errors = Vec::new();
        let mut session = Session::new(false);
        run(
            &mut Cursor::new("def f() -> int:\n    return 42\n"),
            &mut output,
            &mut errors,
            &mut session,
        )
        .unwrap();
        session.execute("f()", &mut output).unwrap();
        assert!(errors.is_empty());
        assert!(String::from_utf8(output).unwrap().ends_with("42\n"));
    }
}
