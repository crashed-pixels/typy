mod config;
mod repl;
mod session;

use session::Session;
use std::{fs, io};

pub(super) fn run() -> Result<(), String> {
    let config = config::parse_args()?;
    let mut output = io::stdout().lock();
    let mut session = Session::new(config.debug);
    if let Some(path) = config.file_path {
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("Failed to read file '{path}': {error}"))?;
        session.execute(&source, &mut output)
    } else {
        repl::run(
            &mut io::stdin().lock(),
            &mut output,
            &mut io::stderr().lock(),
            &mut session,
        )
        .map_err(|error| error.to_string())
    }
}
