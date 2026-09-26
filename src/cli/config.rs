use std::{env, path::Path};

pub(super) struct Config {
    pub debug: bool,
    pub file_path: Option<String>,
}

/// Validates that a filename follows snake_case convention.
///
/// A valid snake_case filename:
/// - Contains only lowercase letters, digits, and underscores
/// - Does not start with a digit
/// - Is not empty
fn is_valid_snake_case(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // First character must be a lowercase letter or underscore
    let first = name.chars().next().unwrap_or('_');
    if !first.is_ascii_lowercase() && first != '_' {
        return false;
    }

    // All characters must be lowercase letters, digits, or underscores
    name.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Validates that a file path has the correct .tp extension and snake_case name.
///
/// Returns Ok(()) if valid, or an error message if invalid.
fn validate_file_path(path: &str) -> Result<(), String> {
    let path_obj = Path::new(path);

    // Check extension
    match path_obj.extension() {
        Some(ext) if ext == "tp" => {}
        Some(ext) => {
            return Err(format!(
                "Invalid file extension '.{}'. Expected '.tp'",
                ext.to_string_lossy()
            ));
        }
        None => {
            return Err("File must have '.tp' extension".to_string());
        }
    }

    // Check filename (without extension)
    match path_obj.file_stem() {
        Some(stem) => {
            let name = stem.to_string_lossy();
            if !is_valid_snake_case(&name) {
                return Err(format!(
                    "Filename '{}' must be in snake_case (lowercase letters, digits, underscores, cannot start with digit)",
                    name
                ));
            }
        }
        None => {
            return Err("Invalid file path".to_string());
        }
    }

    Ok(())
}

/// Parses command-line arguments and returns a Config.
///
/// Supported arguments:
/// - `--debug` or `-d`: Enable debug output
/// - `<file.tp>`: Execute the specified file instead of starting REPL
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
