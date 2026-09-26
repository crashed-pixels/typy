mod config;
mod loader;
mod repl;
mod session;

use session::Session;
use std::io::{self, Write};
use typy::modules::Program;

pub(super) fn run() -> Result<(), String> {
    let config = config::parse_args()?;
    let mut output = io::stdout().lock();
    let mut errors = io::stderr().lock();
    if let Some(path) = config.file_path {
        let (mut loader, entry) = loader::FileLoader::entry(&path)?;
        let program = Program::compile(entry, &mut loader)?;
        let mut trace_error = None;
        let result = program.run_with_options(
            128,
            |line| {
                output
                    .write_all(line.as_bytes())
                    .map_err(|error| format!("IOError: {error}"))
            },
            |event| {
                if config.debug && trace_error.is_none() {
                    trace_error = writeln!(errors, "{event}").err();
                }
            },
        );
        if let Some(error) = trace_error {
            return Err(format!("IOError: {error}"));
        }
        result.map(|_| ())
    } else {
        repl::run(
            &mut io::stdin().lock(),
            &mut output,
            &mut errors,
            &mut Session::new(config.debug),
        )
        .map_err(|error| error.to_string())
    }
}
