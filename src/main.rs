use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;
use typy::compiler::Compiler;
use typy::parser::Parser;
use typy::symbol::Interner;
use typy::tokenizer::try_tokenize_str;
use typy::types::TypeChecker;
use typy::vm::VM;

/// Configuration for the TyPy interpreter.
///
/// Holds runtime flags and optional file path for execution.
struct Config {
    /// Whether to enable debug output (tokens, AST, bytecode, VM state).
    debug: bool,
    /// Optional path to a .tp file to execute instead of starting REPL.
    file_path: Option<String>,
}

/// Reads a line from stdin with the given prompt.
///
/// # Panics
///
/// Panics if there's an error reading from stdin or flushing stdout.
/// This is acceptable for a CLI tool where I/O errors are fatal.
fn read_line(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().expect("Error on flushing stdout");

    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .expect("Error on reading line from stdin");

    line
}

/// Returns true if the line ends with a colon, indicating a block start.
///
/// This is a simple heuristic for the REPL to detect when to enter
/// multi-line input mode for constructs like `if`, `elif`, `else`.
fn needs_block(line: &str) -> bool {
    let trimmed = line.trim_end();
    trimmed.ends_with(':')
}

/// Returns true if the current line signals the end of a block.
///
/// A block ends when:
/// - The line is empty (after trimming)
/// - The line is not indented (not starting with space or tab)
/// - The line is not a continuation keyword like `else:` or `elif`
fn is_block_end(line: &str) -> bool {
    let trimmed = line.trim();

    // Empty line ends the block
    if trimmed.is_empty() {
        return true;
    }

    // Indented line continues the block
    if line.starts_with(' ') || line.starts_with('\t') {
        return false;
    }

    // Continuation keywords keep the block open
    if trimmed == "else:" || trimmed.starts_with("elif ") {
        return false;
    }

    // Any other non-indented line ends the block
    true
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
    let first = name.chars().next().unwrap();
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
fn parse_args() -> Result<Config, String> {
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

/// Reads and executes code from a file.
///
/// This function reads the entire file, tokenizes it, parses it, type-checks it,
/// compiles it, and executes it in the VM.
fn execute_file(
    path: &str,
    vm: &mut VM,
    interner: &mut Interner,
    type_checker: &mut TypeChecker,
    debug: bool,
) -> Result<(), String> {
    let source =
        fs::read_to_string(path).map_err(|e| format!("Failed to read file '{}': {}", path, e))?;

    execute_source(&source, vm, interner, type_checker, debug)
}

/// Checks and executes one atomic submission. The checker is committed only
/// after the VM succeeds; the VM rolls back its own globals on failure.
fn execute_source(
    source: &str,
    vm: &mut VM,
    interner: &mut Interner,
    type_checker: &mut TypeChecker,
    debug: bool,
) -> Result<(), String> {
    if debug {
        println!("[0] Source:\n{}", source);
    }
    let tokens = try_tokenize_str(source)?;
    if debug {
        println!("[1] Tokens: {:?}", tokens);
    }
    let ast = Parser::new(tokens).parse()?;
    if debug {
        println!("\n[2] AST: {:#?}", ast);
    }
    let mut candidate = type_checker.clone();
    let ast = candidate.check_and_resolve(&ast, interner)?;
    let bytecode = Compiler::new().compile(&ast, interner);
    if debug {
        println!("\n[3] Byte-code: {:?}", bytecode);
        println!("\n[4] Running:");
    }
    let result = vm.run_with_trace(&bytecode, interner, |event| {
        if debug {
            println!("{event}");
        }
    })?;
    *type_checker = candidate;
    if !result.is_none() {
        println!("{}", result);
    }
    Ok(())
}

/// Starts the interactive REPL (Read-Eval-Print Loop).
///
/// The REPL reads multi-line input, parses it, type-checks it, compiles it,
/// and executes it. It continues until EOF (Ctrl+D) is received.
fn run_repl(vm: &mut VM, interner: &mut Interner, type_checker: &mut TypeChecker, debug: bool) {
    println!("=== TyPy (v {}) ===", env!("CARGO_PKG_VERSION"));
    let mut pending = None;
    let mut eof = false;
    while !eof {
        let mut source = String::new();
        let mut in_block = false;
        loop {
            let line = pending
                .take()
                .unwrap_or_else(|| read_line(if in_block { "... " } else { ">>> " }));
            if line.is_empty() {
                eof = true;
                println!();
                break;
            }
            if in_block && is_block_end(&line) {
                if !line.trim().is_empty() {
                    pending = Some(line);
                }
                break;
            }
            source.push_str(&line);
            if !in_block {
                if needs_block(&line) {
                    in_block = true;
                } else {
                    break;
                }
            }
        }
        if !source.trim().is_empty()
            && let Err(error) = execute_source(&source, vm, interner, type_checker, debug)
        {
            eprintln!("{}", error);
        }
    }
}

/// Main entry point for the TyPy interpreter.
///
/// Parses command-line arguments and either:
/// - Executes a specified .tp file, or
/// - Starts an interactive REPL
fn main() {
    let config = match parse_args() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    if config.debug {
        println!("Debug mode is ON.");
    }

    let mut vm = VM::new();
    let mut interner = Interner::new();
    let mut type_checker = TypeChecker::new();

    if let Some(file_path) = config.file_path {
        // Execute file
        if let Err(e) = execute_file(
            &file_path,
            &mut vm,
            &mut interner,
            &mut type_checker,
            config.debug,
        ) {
            eprintln!("{}", e);
            process::exit(1);
        }
    } else {
        // Start REPL
        run_repl(&mut vm, &mut interner, &mut type_checker, config.debug);
    }
}
