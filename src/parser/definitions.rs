use super::*;

impl Parser {
    pub(super) fn parse_name(&mut self) -> Result<String, String> {
        match self.current() {
            Token::Name(name) => {
                let name = name.clone();
                self.pos += 1;
                Ok(name)
            }
            token => Err(format!("SyntaxError: expected name, got {:?}", token)),
        }
    }

    pub(super) fn parse_function(&mut self) -> Result<FunctionDef, String> {
        self.eat(Token::Def)?;
        let name = self.parse_name()?;
        self.eat(Token::LParen)?;
        let mut parameters = Vec::new();
        while self.current() != &Token::RParen {
            let name = self.parse_name()?;
            let typ = if self.current() == &Token::Colon {
                self.pos += 1;
                Some(self.parse_type()?)
            } else {
                None
            };
            parameters.push(Parameter { name, typ });
            if self.current() != &Token::Comma {
                break;
            }
            self.pos += 1;
        }
        self.eat(Token::RParen)?;
        self.eat(Token::Arrow)?;
        let return_type = self.parse_type()?;
        self.eat(Token::Colon)?;
        Ok(FunctionDef {
            name,
            parameters,
            return_type,
            body: self.parse_block()?,
        })
    }

    pub(super) fn parse_class(&mut self) -> Result<Stmt, String> {
        self.eat(Token::Class)?;
        let name = self.parse_name()?;
        self.eat(Token::Colon)?;
        Ok(Stmt::Class {
            name,
            body: self.parse_block()?,
        })
    }
}
