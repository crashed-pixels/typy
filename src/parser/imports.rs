use super::*;
use crate::ast::ImportName;

impl Parser {
    pub(super) fn parse_dotted_name(&mut self) -> Result<String, String> {
        let mut name = self.parse_name()?;
        while self.current() == &Token::Dot {
            self.pos += 1;
            name.push('.');
            name.push_str(&self.parse_name()?);
        }
        Ok(name)
    }

    pub(super) fn parse_import(&mut self) -> Result<Stmt, String> {
        if self.current() == &Token::Import {
            self.pos += 1;
            let module = self.parse_dotted_name()?;
            let alias = self.import_alias(module.rsplit('.').next().unwrap_or(&module))?;
            return Ok(Stmt::Import { module, alias });
        }
        self.eat(Token::From)?;
        let module = self.parse_dotted_name()?;
        self.eat(Token::Import)?;
        let mut names = Vec::new();
        loop {
            if self.current() == &Token::Star {
                return Err("ImportError: wildcard imports are forbidden; name each import".into());
            }
            let name = self.parse_name()?;
            let alias = self.import_alias(&name)?;
            names.push(ImportName { name, alias });
            if self.current() != &Token::Comma {
                break;
            }
            self.pos += 1;
        }
        Ok(Stmt::FromImport { module, names })
    }

    fn import_alias(&mut self, default: &str) -> Result<String, String> {
        if self.current() == &Token::As {
            self.pos += 1;
            self.parse_name()
        } else {
            Ok(default.to_string())
        }
    }
}
