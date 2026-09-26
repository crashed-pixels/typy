use std::{env, path::Path};

pub(super) struct Config {
    pub debug: bool,
    pub file_path: Option<String>,
}

/// File execution always starts at a main.tp entry.
fn validate_file_path(path: &str) -> Result<(), String> {
    if Path::new(path)
        .file_name()
        .is_none_or(|name| name != "main.tp")
    {
        return Err("EntryError: only main.tp can be executed; libraries use lib.tp".into());
    }
    Ok(())
}

/// Parses command-line arguments and returns a Config.
///
/// Supported arguments:
/// - `--debug` or `-d`: Enable debug output
/// - `<main.tp>`: Execute the specified file instead of starting REPL
///
/// # Errors
///
/// Returns an error if an unknown argument is provided or if the file
/// path is invalid.
pub(super) fn parse_args() -> Result<Config, String> {
    let args: Vec<String> = env::args().collect();

    let mut debug = false;
    let mut file_path = None;

    for arg in &args[1..] {
        if arg == "--debug" || arg == "-d" {
            debug = true;
        } else if arg.starts_with('-') {
            return Err(format!("Unknown argument: {}", arg));
        } else {
            // Treat non-flag arguments as file paths
            if file_path.is_some() {
                return Err("Only one file can be executed at a time".to_string());
            }
            validate_file_path(arg)?;
            file_path = Some(arg.clone());
        }
    }

    Ok(Config { debug, file_path })
}
