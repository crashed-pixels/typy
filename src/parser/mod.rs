mod definitions;
pub use crate::ast::{Expr, FunctionDef, Operator, Parameter, Stmt};
use crate::tokenizer::Token;
use crate::types::Type;
use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec::Vec,
};

/// A recursive descent parser.
///
/// The parser consumes a flat stream of tokens and produces a hierarchical
/// abstract syntax tree. It follows standard precedence rules:
/// 1. Comparison operators (lowest precedence)
/// 2. Addition and subtraction
/// 3. Multiplication and division
/// 4. Atoms and parentheses (highest precedence)
pub struct Parser {
    /// The token stream to parse.
    tokens: Vec<Token>,
    /// The current position in the token stream.
    pos: usize,
}

impl Parser {
    /// Creates a new parser for the given token stream.
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    /// Returns the current token, or `Eof` if at the end.
    fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    /// Consumes the current token if it matches the expected token.
    ///
    /// Returns an error if the current token does not match.
    fn eat(&mut self, expected: Token) -> Result<(), String> {
        if self.current() == &expected {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!(
                "SyntaxError: expected {:?}, but got {:?}",
                expected,
                self.current()
            ))
        }
    }

    /// Parses the entire token stream into a list of statements.
    ///
    /// Leading and trailing newlines are ignored. Parsing continues until
    /// the end of file is reached.
    pub fn parse(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();

        // Skip leading newlines
        while self.current() == &Token::NewLine {
            self.pos += 1;
        }

        // Parse statements until EOF
        while self.current() != &Token::Eof {
            let stmt = self.parse_statement()?;
            stmts.push(stmt);

            // Skip newlines between statements
            while self.current() == &Token::NewLine {
                self.pos += 1;
            }
        }

        Ok(stmts)
    }

    /// Parses a single statement.
    ///
    /// Statement grammar:
    /// ```text
    /// statement = if-statement
    ///           | NAME ':' TYPE ['=' expression]
    ///           | NAME '=' expression
    ///           | expression
    /// ```
    fn parse_statement(&mut self) -> Result<Stmt, String> {
        let stmt = self.parse_statement_inner()?;
        if !matches!(
            stmt,
            Stmt::If { .. } | Stmt::Function(_) | Stmt::Class { .. }
        ) && !matches!(self.current(), Token::NewLine | Token::Eof)
        {
            return Err(format!(
                "SyntaxError: expected newline, but got {:?}",
                self.current()
            ));
        }
        Ok(stmt)
    }

    fn parse_statement_inner(&mut self) -> Result<Stmt, String> {
        match self.current() {
            Token::Def => return self.parse_function().map(Stmt::Function),
            Token::Class => return self.parse_class(),
            Token::Pass => {
                self.pos += 1;
                return Ok(Stmt::Pass);
            }
            Token::Return => {
                self.pos += 1;
                let value = if matches!(self.current(), Token::NewLine | Token::Eof) {
                    None
                } else {
                    Some(self.parse_expression()?)
                };
                return Ok(Stmt::Return(value));
            }
            _ => {}
        }
        // Try to parse an if statement
        if self.current() == &Token::If {
            return self.parse_if_statement();
        }

        // Try to parse a variable declaration or assignment
        if let Token::Name(name) = self.current() {
            let name = name.clone();
            let saved_pos = self.pos;
            self.pos += 1;

            // Check for type annotation: name: type [= expr]
            if self.current() == &Token::Colon {
                self.pos += 1;
                let typ = self.parse_type()?;
                let initializer = if self.current() == &Token::Assign {
                    self.pos += 1;
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                return Ok(Stmt::VariableDecl {
                    name,
                    typ,
                    initializer,
                });
            }

            // Check for assignment: name = expr
            if self.current() == &Token::Assign {
                self.pos += 1;
                let value = self.parse_expression()?;
                return Ok(Stmt::Assign { name, value });
            }

            // Not a declaration or assignment, backtrack and parse as expression
            self.pos = saved_pos;
        }

        // Parse as expression statement
        let expr = self.parse_expression()?;
        if self.current() == &Token::Assign {
            self.pos += 1;
            let value = self.parse_expression()?;
            if let Expr::Attribute { object, name } = expr {
                return Ok(Stmt::SetAttribute {
                    object: *object,
                    name,
                    value,
                });
            }
            return Err("SyntaxError: invalid assignment target".to_string());
        }
        Ok(Stmt::Expr(expr))
    }

    /// Parses a type annotation.
    ///
    /// Builtins are recognized here; nominal names are resolved by the checker.
    fn parse_type(&mut self) -> Result<Type, String> {
        if self.current() == &Token::None {
            self.pos += 1;
            return Ok(Type::None);
        }
        let name = self.parse_name()?;
        Ok(match name.as_str() {
            "int" => Type::Int,
            "bool" => Type::Bool,
            _ => Type::Named(name),
        })
    }

    /// Parses an if statement.
    ///
    /// Grammar:
    /// ```text
    /// if-statement = 'if' expression ':' block
    ///                ('elif' expression ':' block)*
    ///                ['else' ':' block]
    /// ```
    fn parse_if_statement(&mut self) -> Result<Stmt, String> {
        self.eat(Token::If)?;
        let condition = self.parse_expression()?;
        self.eat(Token::Colon)?;
        let then_branch = self.parse_block()?;

        // Parse elif branches
        let mut elif_branches = Vec::new();
        while self.current() == &Token::Elif {
            self.pos += 1;
            let condition = self.parse_expression()?;
            self.eat(Token::Colon)?;
            let branch = self.parse_block()?;
            elif_branches.push((condition, branch));
        }

        // Parse optional else branch
        let else_branch = if self.current() == &Token::Else {
            self.pos += 1;
            self.eat(Token::Colon)?;
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(Stmt::If {
            condition,
            then_branch,
            elif_branches,
            else_branch,
        })
    }

    /// Parses a block of statements.
    ///
    /// A block is an indented sequence of statements, preceded by a newline
    /// and indent token, and followed by a dedent token.
    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.eat(Token::NewLine)?;
        while self.current() == &Token::NewLine {
            self.pos += 1;
        }
        self.eat(Token::Indent)?;

        let mut stmts = Vec::new();

        // Skip leading newlines in the block
        while self.current() == &Token::NewLine {
            self.pos += 1;
        }

        // Parse statements until dedent or EOF
        while self.current() != &Token::Dedent && self.current() != &Token::Eof {
            stmts.push(self.parse_statement()?);

            // Skip newlines between statements
            while self.current() == &Token::NewLine {
                self.pos += 1;
            }
        }

        self.eat(Token::Dedent)?;

        if stmts.is_empty() {
            return Err("SyntaxError: expected an indented block".to_string());
        }

        Ok(stmts)
    }

    /// Parses an expression with comparison operators.
    ///
    /// Grammar:
    /// ```text
    /// expression = addition (('==' | '!=' | '<' | '>' | '<=' | '>=') addition)*
    /// ```
    fn parse_expression(&mut self) -> Result<Expr, String> {
        self.parse_binary_op(
            Self::parse_addition,
            &[
                (Token::Eq, Operator::Eq),
                (Token::NotEq, Operator::NotEq),
                (Token::Less, Operator::Less),
                (Token::Greater, Operator::Greater),
                (Token::LessEq, Operator::LessEq),
                (Token::GreaterEq, Operator::GreaterEq),
            ],
        )
    }

    /// Parses an expression with addition and subtraction operators.
    ///
    /// Grammar:
    /// ```text
    /// addition = term (('+' | '-') term)*
    /// ```
    fn parse_addition(&mut self) -> Result<Expr, String> {
        self.parse_binary_op(
            Self::parse_term,
            &[
                (Token::Plus, Operator::Plus),
                (Token::Minus, Operator::Minus),
            ],
        )
    }

    /// Parses an expression with multiplication and division operators.
    ///
    /// Grammar:
    /// ```text
    /// term = factor (('*' | '/') factor)*
    /// ```
    fn parse_term(&mut self) -> Result<Expr, String> {
        self.parse_binary_op(
            Self::parse_factor,
            &[
                (Token::Star, Operator::Star),
                (Token::Slash, Operator::Slash),
            ],
        )
    }

    /// Parses a binary operation at a given precedence level.
    ///
    /// This is a helper that eliminates duplication across expression parsing
    /// methods. It takes a parser function for the next precedence level and
    /// a list of operator tokens to recognize at this level.
    fn parse_binary_op<F>(
        &mut self,
        parse_operand: F,
        operators: &[(Token, Operator)],
    ) -> Result<Expr, String>
    where
        F: Fn(&mut Self) -> Result<Expr, String>,
    {
        let mut left = parse_operand(self)?;

        loop {
            // Check if the current token matches any operator at this level
            let op = operators
                .iter()
                .find(|(token, _)| self.current() == token)
                .map(|(_, op)| *op);

            if let Some(operator) = op {
                self.pos += 1;
                let right = parse_operand(self)?;
                left = Expr::BinaryOp {
                    left: Box::new(left),
                    op: operator,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    /// Parses an atomic expression (a factor).
    ///
    /// Grammar:
    /// ```text
    /// factor = NUMBER
    ///        | NAME
    ///        | 'True'
    ///        | 'False'
    ///        | '(' expression ')'
    /// ```
    fn parse_factor(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_atom()?;
        loop {
            match self.current() {
                Token::Dot => {
                    self.pos += 1;
                    expr = Expr::Attribute {
                        object: Box::new(expr),
                        name: self.parse_name()?,
                    };
                }
                Token::LParen => {
                    self.pos += 1;
                    let mut arguments = Vec::new();
                    while self.current() != &Token::RParen {
                        arguments.push(self.parse_expression()?);
                        if self.current() != &Token::Comma {
                            break;
                        }
                        self.pos += 1;
                    }
                    self.eat(Token::RParen)?;
                    expr = Expr::Call {
                        callee: Box::new(expr),
                        arguments,
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_atom(&mut self) -> Result<Expr, String> {
        match self.current() {
            Token::None => {
                self.pos += 1;
                Ok(Expr::None)
            }
            Token::Number(n) => {
                let value = *n;
                self.pos += 1;
                Ok(Expr::Number(value))
            }
            Token::True => {
                self.pos += 1;
                Ok(Expr::Bool(true))
            }
            Token::False => {
                self.pos += 1;
                Ok(Expr::Bool(false))
            }
            Token::Name(name) => {
                let var_name = name.clone();
                self.pos += 1;
                Ok(Expr::Name(var_name))
            }
            Token::LParen => {
                self.pos += 1;
                let expr = self.parse_expression()?;
                self.eat(Token::RParen)?;
                Ok(expr)
            }
            _ => Err(format!(
                "SyntaxError: expected expression but got {:?}",
                self.current()
            )),
        }
    }
}
