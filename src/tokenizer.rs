use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::iter::Peekable;
use core::str::Chars;

/// A lexical token recognized by the interpreter.
///
/// This enum intentionally keeps a small, Python-like token set.
#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Def,
    Class,
    Return,
    Pass,
    None,
    Arrow,
    Comma,
    Dot,
    /// An integer literal.
    Number(i64),

    /// The boolean literal `True`.
    True,

    /// The boolean literal `False`.
    False,

    /// The `+` operator.
    Plus,

    /// The `-` operator.
    Minus,

    /// The `*` operator.
    Star,

    /// The `/` operator.
    Slash,

    /// The `==` operator.
    Eq,

    /// The `!=` operator.
    NotEq,

    /// The `>` operator.
    Greater,

    /// The `<` operator.
    Less,

    /// The `>=` operator.
    GreaterEq,

    /// The `<=` operator.
    LessEq,

    /// The `(` delimiter.
    LParen,

    /// The `)` delimiter.
    RParen,

    /// An identifier that is not a keyword.
    Name(String),

    /// The `=` assignment operator.
    Assign,

    /// The `:` delimiter.
    Colon,

    /// A logical line terminator.
    NewLine,

    /// An increase in indentation level.
    Indent,

    /// A decrease in indentation level.
    Dedent,

    /// The `if` keyword.
    If,

    /// The `elif` keyword.
    Elif,

    /// The `else` keyword.
    Else,

    /// End of input.
    Eof,
}

/// Tokenizes an owned string.
///
/// This entry point is preserved for backward compatibility. It delegates to
/// [`tokenize_str`], because the lexer only needs to borrow its input.
pub fn tokenize(input: String) -> Vec<Token> {
    tokenize_str(&input)
}

/// Tokenizes a string slice.
///
/// This is a more efficient and more idiomatic entry point when the caller
/// already has a `&str`.
///
/// # Panics
/// Panics on invalid source for backward compatibility. Use [`try_tokenize_str`]
/// when errors need to be reported without unwinding.
pub fn tokenize_str(input: &str) -> Vec<Token> {
    try_tokenize_str(input).unwrap_or_else(|error| panic!("{error}"))
}

/// Tokenizes source with recoverable diagnostics for invalid tokens, indentation,
/// and overflowing literals. LF, CRLF, and CR use the same newline semantics.
pub fn try_tokenize_str(input: &str) -> Result<Vec<Token>, String> {
    if input.contains('\r') {
        let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
        Tokenizer::new(&normalized).tokenize()
    } else {
        Tokenizer::new(input).tokenize()
    }
}

/// Internal lexical analyzer state.
///
/// The tokenizer is modeled after a small subset of CPython's tokenizer
/// behavior: it emits `NEWLINE`, `INDENT`, and `DEDENT` tokens.
///
/// This version is intentionally simpler than CPython's real tokenizer.
/// It does not support comments, string literals, floating point numbers,
/// continuation lines, or full CPython tab expansion rules.
struct Tokenizer<'input> {
    chars: Peekable<Chars<'input>>,
    tokens: Vec<Token>,
    indent_stack: Vec<usize>,
    at_line_start: bool,
}

impl<'input> Tokenizer<'input> {
    /// Creates a tokenizer for the given input.
    fn new(input: &'input str) -> Self {
        Self {
            chars: input.chars().peekable(),
            tokens: Vec::new(),
            indent_stack: vec![0],
            at_line_start: true,
        }
    }

    /// Runs the tokenizer to completion and returns the token stream.
    fn tokenize(mut self) -> Result<Vec<Token>, String> {
        while let Some(ch) = self.chars.next() {
            if ch == '\n' {
                self.push(Token::NewLine);
                self.at_line_start = true;
                continue;
            }

            if self.at_line_start {
                if ch == ' ' || ch == '\t' {
                    self.handle_indentation(ch)?;
                    continue;
                }

                self.emit_dedents_to_zero();
                self.at_line_start = false;
            }

            if ch.is_whitespace() {
                continue;
            }

            if ch.is_ascii_digit() {
                self.scan_number(ch)?;
                continue;
            }

            if is_name_start(ch) {
                self.scan_name(ch);
                continue;
            }

            if self.scan_operator(ch)? {
                continue;
            }

            self.scan_punctuation(ch)?;
        }

        self.finish();
        Ok(self.tokens)
    }

    /// Pushes a token into the output stream.
    fn push(&mut self, token: Token) {
        self.tokens.push(token);
    }

    /// Returns true if the next character is exactly `expected`.
    fn peek_is(&mut self, expected: char) -> bool {
        self.chars.peek() == Some(&expected)
    }

    /// Returns true if there are no more characters.
    fn peek_is_none(&mut self) -> bool {
        self.chars.peek().is_none()
    }

    /// Returns the current indentation level.
    ///
    /// The stack should always contain at least the base level `0`.
    /// If it ever does not, we conservatively return `0` instead of panicking.
    fn current_indent(&self) -> usize {
        self.indent_stack.last().copied().unwrap_or(0)
    }

    /// Handles indentation at the beginning of a logical line.
    ///
    /// This function consumes all leading spaces and tabs, decides whether
    /// the indentation changed, and emits `INDENT` or `DEDENT` tokens.
    ///
    /// Blank lines and trailing whitespace at EOF do not affect indentation.
    fn handle_indentation(&mut self, first: char) -> Result<(), String> {
        let mut indent_level = Self::indent_width(first);

        while let Some(&next_ch) = self.chars.peek() {
            match next_ch {
                ' ' => {
                    indent_level += 1;
                    self.chars.next();
                }
                '\t' => {
                    indent_level += 4;
                    self.chars.next();
                }
                _ => break,
            }
        }

        // Blank lines and trailing whitespace at EOF do not affect indentation.
        if self.peek_is('\n') || self.peek_is_none() {
            return Ok(());
        }

        let current_indent = self.current_indent();

        if indent_level > current_indent {
            self.indent_stack.push(indent_level);
            self.push(Token::Indent);
        } else if indent_level < current_indent {
            while self.current_indent() > indent_level {
                let _ = self.indent_stack.pop();
                self.push(Token::Dedent);
            }

            if self.current_indent() != indent_level {
                return Err(
                    "IndentationError: unindent does not match any outer indentation level"
                        .to_string(),
                );
            }
        }

        self.at_line_start = false;
        Ok(())
    }

    /// Returns the width of a single indentation character.
    ///
    /// This keeps the original simplified behavior: a tab advances by four
    /// columns. CPython uses more complex tab expansion rules.
    fn indent_width(ch: char) -> usize {
        if ch == '\t' { 4 } else { 1 }
    }

    /// Emits `DEDENT` tokens until indentation returns to zero.
    ///
    /// This is used when a non-whitespace character appears at the beginning
    /// of a line while the tokenizer is inside an indented block.
    fn emit_dedents_to_zero(&mut self) {
        while self.current_indent() > 0 {
            let _ = self.indent_stack.pop();
            self.push(Token::Dedent);
        }
    }

    /// Scans an integer literal.
    ///
    /// The first digit character has already been consumed.
    fn scan_number(&mut self, first: char) -> Result<(), String> {
        let mut text = String::new();
        text.push(first);

        while let Some(&next_ch) = self.chars.peek() {
            if next_ch.is_ascii_digit() {
                self.chars.next();
                text.push(next_ch);
            } else {
                break;
            }
        }

        let value = text
            .parse::<i64>()
            .map_err(|_| format!("SyntaxError: invalid integer literal: {}", text))?;

        self.push(Token::Number(value));
        Ok(())
    }

    /// Scans an identifier or keyword.
    ///
    /// The first identifier character has already been consumed.
    fn scan_name(&mut self, first: char) {
        let mut name = String::new();
        name.push(first);

        while let Some(&next_ch) = self.chars.peek() {
            if is_name_continue(next_ch) {
                self.chars.next();
                name.push(next_ch);
            } else {
                break;
            }
        }

        match keyword_token(&name) {
            Some(keyword) => self.push(keyword),
            None => self.push(Token::Name(name)),
        }
    }

    /// Tries to scan a multi-character or single-character operator.
    ///
    /// Returns `true` if the character was handled as an operator.
    fn scan_operator(&mut self, ch: char) -> Result<bool, String> {
        Ok(match ch {
            '-' if self.peek_is('>') => {
                self.chars.next();
                self.push(Token::Arrow);
                true
            }
            '<' => {
                if self.peek_is('=') {
                    self.chars.next();
                    self.push(Token::LessEq);
                } else {
                    self.push(Token::Less);
                }
                true
            }
            '>' => {
                if self.peek_is('=') {
                    self.chars.next();
                    self.push(Token::GreaterEq);
                } else {
                    self.push(Token::Greater);
                }
                true
            }
            '=' => {
                if self.peek_is('=') {
                    self.chars.next();
                    self.push(Token::Eq);
                } else {
                    self.push(Token::Assign);
                }
                true
            }
            '!' => {
                if self.peek_is('=') {
                    self.chars.next();
                    self.push(Token::NotEq);
                } else {
                    return Err(format!("SyntaxError: unknown token: {}", ch));
                }
                true
            }
            _ => false,
        })
    }

    /// Scans a single-character punctuation token.
    ///
    /// Unknown characters are treated as syntax errors.
    fn scan_punctuation(&mut self, ch: char) -> Result<(), String> {
        let token = match ch {
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '(' => Token::LParen,
            ')' => Token::RParen,
            ':' => Token::Colon,
            ',' => Token::Comma,
            '.' => Token::Dot,
            _ => return Err(format!("SyntaxError: unknown token: {}", ch)),
        };

        self.push(token);
        Ok(())
    }

    /// Finalizes the token stream.
    ///
    /// This adds a trailing `NEWLINE` if needed, closes all remaining open
    /// indentation levels with `DEDENT` tokens, and finally emits `EOF`.
    fn finish(&mut self) {
        if let Some(last_token) = self.tokens.last()
            && *last_token != Token::NewLine
        {
            self.push(Token::NewLine);
        }

        while self.indent_stack.len() > 1 {
            let _ = self.indent_stack.pop();
            self.push(Token::Dedent);
        }

        self.push(Token::Eof);
    }
}

/// Returns true if the character can start an identifier.
fn is_name_start(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_'
}

/// Returns true if the character can continue an identifier.
fn is_name_continue(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

/// Maps an identifier string to a keyword token, if applicable.
fn keyword_token(name: &str) -> Option<Token> {
    match name {
        "def" => Some(Token::Def),
        "class" => Some(Token::Class),
        "return" => Some(Token::Return),
        "pass" => Some(Token::Pass),
        "None" => Some(Token::None),
        "True" => Some(Token::True),
        "False" => Some(Token::False),
        "if" => Some(Token::If),
        "elif" => Some(Token::Elif),
        "else" => Some(Token::Else),
        _ => None,
    }
}
