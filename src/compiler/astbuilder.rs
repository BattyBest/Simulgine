use ordered_float::NotNan;
use std::iter::Peekable;

use num_traits::{FromPrimitive, ToPrimitive};

use crate::compiler::scanner::FileScanner;

use super::{
    ast::*,
    pratttable::{get_pratt_entry, PrattEntry},
    r#type::*,
    scanner::{Token, TokenType, TokenValue},
};

static EMPTY_ERROR_TOKEN: Token = FileScanner::empty_error_token("Non-existent token.");

pub(super) struct ASTBuilder<T: Iterator<Item = Token>> {
    token_generator: Peekable<T>,
    any_error: bool,
    error_recovery: bool,
    blocks: Vec<Vec<ASTVariable>>,
}

pub trait ErrorUnwrap<T> {
    fn error_expect(self) -> T;
}

impl ErrorUnwrap<Token> for Option<Token> {
    fn error_expect(self) -> Token {
        match self {
            Some(x) => x,
            None => {
                eprintln!("Expected more tokens.");

                FileScanner::empty_error_token("Non-existent token.")
            }
        }
    }
}

impl<'a> ErrorUnwrap<&'a Token> for Option<&'a Token> {
    fn error_expect(self) -> &'a Token {
        match self {
            Some(x) => x,
            None => {
                eprintln!("Expected more tokens.");

                &EMPTY_ERROR_TOKEN
            }
        }
    }
}

static LET_HINT: &str =
    "Let expressions follow this format: let [variable name]: [variable type] = [initial value];";
static FIELD_HINT: &str =
    "Field Definitions follow this format, [] indicating optionals: [Access] [Volatility] Type Name[::<stage>] [body] [= initial value]";

impl<T: Iterator<Item = Token>> ASTBuilder<T> {
    pub(crate) fn synthesize_astbuilder(token_gen: T) -> ASTBuilder<T> {
        ASTBuilder {
            token_generator: token_gen.peekable(),
            any_error: false,
            error_recovery: false,
            blocks: Vec::new(),
        }
    }

    // Sorry for the extremely ugly ASCII escape sequence formatting
    fn error_at(&mut self, message: &'static str, token: &Token) -> ASTNode {
        self.any_error = true;

        if !self.error_recovery {
            eprintln!("\x1b[91mError occured at \x1b[93mline {}\x1b[91m, \x1b[93mcolumn {}\x1b[91m:\x1b[0m {}", token.line, token.column, message);
            if TokenType::Error == token.token_type {
                if let TokenValue::String(x) = &token.value {
                    eprintln!("Further details: {}", x);
                }
            }
        };
        self.error_recovery = true;

        ASTNode {
            token: token.clone(),
            inner: ASTNodeInner::Error,
        }
    }

    fn dyn_error_at(&mut self, msg: String, token: &Token) -> ASTNode {
        self.any_error = true;

        if !self.error_recovery {
            eprintln!("\x1b[91mError occured at \x1b[93mline {}\x1b[91m, \x1b[93mcolumn {}\x1b[91m:\x1b[0m {}", token.line, token.column, msg);
            if TokenType::Error == token.token_type {
                if let TokenValue::String(x) = &token.value {
                    eprintln!("Further details: {}", x);
                }
            }
        };
        self.error_recovery = true;

        ASTNode {
            token: token.clone(),
            inner: ASTNodeInner::Error,
        }
    }

    fn advance_if(&mut self, token_t: TokenType) -> bool {
        let peek = self.token_generator.peek().error_expect();

        if peek.token_type == token_t {
            self.token_generator.next();
            true
        } else {
            false
        }
    }

    fn advance(&mut self) -> Token {
        self.token_generator.next().error_expect()
    }

    fn consume(&mut self, token_t: TokenType) -> Token {
        let token = self.advance();

        if token.token_type == token_t {
            token
        } else {
            self.dyn_error_at(format!("Expected token of type {:?}", token_t), &token);
            FileScanner::empty_error_token("Fake token.")
        }
    }

    fn consume_hint(&mut self, token_t: TokenType, msg: &str) -> Token {
        let token = self.advance();

        if token.token_type == token_t {
            token
        } else {
            self.dyn_error_at(
                format!("Expected token of type {:?}\n💡 Hint: {}", token_t, msg),
                &token,
            );
            FileScanner::empty_error_token("Fake token.")
        }
    }

    // fn consume_of_many(&mut self, token_t: &[TokenType]) -> Token {
    //     let token = self.advance();

    //     if token_t.contains(&token.token_type) {
    //         token
    //     } else {
    //         self.dyn_error_at(format!("Expected token any of type {:?}", token_t), &token);
    //         FileScanner::empty_error_token("Fake token.")
    //     }
    // }

    fn consume_if(&mut self, token_t: &[TokenType]) -> Option<Token> {
        self.token_generator
            .next_if(|x| token_t.contains(&x.token_type))
    }

    fn consume_if_exist(&mut self, token_t: TokenType) -> Option<Token> {
        if self
            .token_generator
            .peek()
            .is_none_or(|x| x.token_type == TokenType::Eof)
        {
            return None;
        };
        let tok = self.advance();
        if token_t == tok.token_type {
            Some(tok)
        } else {
            self.dyn_error_at(
                format!(
                    "Expected token of type {:?}, got {:?}",
                    token_t, tok.token_type
                ),
                &tok,
            );
            Some(FileScanner::empty_error_token("Fake token."))
        }
    }

    pub(super) fn parse_literal(&mut self, token: Token) -> ASTNode {
        let c = token.clone();
        let inner = match token.token_type {
            TokenType::Integer => ASTNodeInner::Integer(token.value.integer()),
            TokenType::String => ASTNodeInner::String(token.value.string()),
            TokenType::Decimal => ASTNodeInner::Decimal(token.value.double()),
            TokenType::KeywordThis => ASTNodeInner::Variable(ASTVarRef {
                indx: 0,
                indx_up: self.blocks.len() as u8,
            }),
            TokenType::KeywordParent => ASTNodeInner::Variable(ASTVarRef {
                indx: 1,
                indx_up: self.blocks.len() as u8,
            }),
            TokenType::ConstTrue => ASTNodeInner::True,
            TokenType::ConstFalse => ASTNodeInner::False,
            _ => unreachable!(),
        };

        ASTNode { token: c, inner }
    }

    pub(super) fn parse_grouping(&mut self, token: Token) -> ASTNode {
        let inner = self.expression();
        self.consume(match &token.token_type {
            TokenType::OpenParen => TokenType::CloseParen,
            _ => unreachable!("Non-grouping token called parse_grouping, somehow."),
        });

        inner
    }

    pub(super) fn parse_braces(&mut self, token: Token) -> ASTNode {
        let mut inner = vec![];
        if self.blocks.len() >= u8::MAX.into() {
            return self.error_at(
                "Max block nesting depth reached. You can only nest blocks 2^8-1 (255) times.",
                &token,
            );
        }
        self.blocks.push(Vec::new());
        while self.token_generator.peek().error_expect().token_type != TokenType::CloseBrace {
            if self.error_recovery {
                return ASTNode {
                    token,
                    inner: ASTNodeInner::Error,
                };
            }
            inner.push(self.statement());
        }
        let vars = self.blocks.pop().unwrap();
        self.consume(TokenType::CloseBrace);

        ASTNode {
            token: token.clone(),
            inner: ASTNodeInner::Brace(ASTBlock {
                variables: vars,
                exprs: inner,
            }),
        }
    }

    pub(super) fn parse_binary(&mut self, prev: ASTNode, token: Token) -> ASTNode {
        let token_rule: PrattEntry<T> = get_pratt_entry(&token.token_type);

        let binding =
            FromPrimitive::from_u8(ToPrimitive::to_u8(&token_rule.precedence).expect("") - 1)
                .expect("");
        let right = self.parse_precedence(match token_rule.precedence {
            Precedence::ASSIGNMENT => &Precedence::ASSIGNMENT,
            _ => &binding,
        });

        let inner = match token.token_type {
            TokenType::OperatorAdd => ASTNodeInner::Add(Box::new(prev), Box::new(right)),
            TokenType::OperatorSub => ASTNodeInner::Subtract(Box::new(prev), Box::new(right)),
            TokenType::OperatorMul => ASTNodeInner::Multiply(Box::new(prev), Box::new(right)),
            TokenType::OperatorDiv => ASTNodeInner::Divide(Box::new(prev), Box::new(right)),
            TokenType::OperatorMod => ASTNodeInner::Modulus(Box::new(prev), Box::new(right)),
            TokenType::OperatorPow => ASTNodeInner::Pow(Box::new(prev), Box::new(right)),
            TokenType::OperatorAssign => {
                let x = match prev.inner {
                    ASTNodeInner::Variable(x) => x,
                    _ => {
                        return self.error_at("Need a lvalue (local variable reference) to the left of an assignment.", &token);
                    }
                };
                ASTNodeInner::Set(x, Box::new(right))
            }
            TokenType::OperatorEqual => ASTNodeInner::Equal(Box::new(prev), Box::new(right)),
            TokenType::OperatorNotEqual => ASTNodeInner::NotEqual(Box::new(prev), Box::new(right)),
            TokenType::OperatorLessThan => ASTNodeInner::Lesser(Box::new(prev), Box::new(right)),
            TokenType::OperatorLessThanEqual => {
                ASTNodeInner::LesserE(Box::new(prev), Box::new(right))
            }
            TokenType::OperatorGreaterThan => {
                ASTNodeInner::Greater(Box::new(prev), Box::new(right))
            }
            TokenType::OperatorGreaterThanEqual => {
                ASTNodeInner::GreaterE(Box::new(prev), Box::new(right))
            }
            TokenType::OperatorAnd => ASTNodeInner::And(Box::new(prev), Box::new(right)),
            TokenType::OperatorOr => ASTNodeInner::Or(Box::new(prev), Box::new(right)),
            _ => unreachable!(),
        };

        ASTNode {
            token: token.clone(),
            inner,
        }
    }

    pub(super) fn parse_inner(&mut self, prev: ASTNode, token: Token) -> ASTNode {
        let id = self.consume(TokenType::Identifier);
        if self.error_recovery {
            return ASTNode {
                token: token.clone(),
                inner: ASTNodeInner::Error,
            };
        };
        let str = id.value.string();

        let mut node = ASTNode {
            token: token.clone(),
            inner: ASTNodeInner::Inner(Box::new(prev), str),
        };

        while self.consume_if(&[TokenType::OperatorInner]).is_some() {
            let id = self.consume(TokenType::Identifier);
            let str = id.clone().value.string();

            node = ASTNode {
                token: id,
                inner: ASTNodeInner::Inner(Box::new(node), str),
            };
        }

        node
    }

    pub(super) fn parse_if(&mut self, token: Token) -> ASTNode {
        let condition = self.expression();

        let true_expr = self.expression();

        let mut false_expr = None;

        if self.advance_if(TokenType::KeywordElse) {
            false_expr = Some(Box::new(self.expression()));
        };

        ASTNode {
            token: token.clone(),
            inner: ASTNodeInner::If(Box::new(condition), Box::new(true_expr), false_expr),
        }
    }

    fn parse_precedence(&mut self, prec: &Precedence) -> ASTNode {
        let prefix_token = self.advance();

        let prefix_rule = get_pratt_entry(&prefix_token.token_type);
        if let Some(x) = prefix_rule.prefix {
            let mut res = x(self, prefix_token);
            let mut rule = get_pratt_entry(&self.token_generator.peek().error_expect().token_type);

            while ToPrimitive::to_u8(prec).expect("") <= rule.precedence as u8 {
                let new_token = self.token_generator.next().error_expect();
                if let Some(x) = rule.infix {
                    res = x(self, res, new_token);
                } else {
                    return self.error_at("Expected operator or terminator.", &new_token);
                }
                rule = get_pratt_entry(&self.token_generator.peek().error_expect().token_type);
            }

            res
        } else {
            self.error_at("Expected constant or unary operator.", &prefix_token)
        }
    }

    pub(super) fn parse_let(&mut self, token: Token) -> ASTNode {
        let var = self.consume_hint(TokenType::Identifier, LET_HINT);
        if self.any_error {
            return ASTNode {
                token,
                inner: ASTNodeInner::Error,
            };
        }
        self.consume_hint(TokenType::OperatorTyper, LET_HINT);
        if self.any_error {
            return ASTNode {
                token,
                inner: ASTNodeInner::Error,
            };
        }
        let t = self.consume_hint(TokenType::Identifier, LET_HINT);
        if self.any_error {
            return ASTNode {
                token,
                inner: ASTNodeInner::Error,
            };
        }
        self.consume_hint(TokenType::OperatorAssign, LET_HINT);
        if self.any_error {
            return ASTNode {
                token,
                inner: ASTNodeInner::Error,
            };
        }
        let expr = self.expression();
        if self.any_error {
            return ASTNode {
                token,
                inner: ASTNodeInner::Error,
            };
        }

        let n_vars = self.blocks.pop();
        if n_vars.is_none() {
            return self.dyn_error_at(
                "Cannot use let outside of a brace-block.".to_string(),
                &token,
            );
        }
        let mut n_vars = n_vars.unwrap();
        n_vars.push(ASTVariable {
            name: var.value.string(),
            t: TypeIdentifier::UnlinkedType(t.value.string()),
        });

        let ret = ASTNode {
            token: token.clone(),
            inner: ASTNodeInner::Set(
                ASTVarRef {
                    indx: (n_vars.len() - 1) as u8,
                    indx_up: 0,
                },
                Box::new(expr),
            ),
        };

        self.blocks.push(n_vars);

        ret
    }

    fn lookup_var(&mut self, name: &str) -> Option<ASTVarRef> {
        let block = self.blocks.pop();
        match block {
            Some(x) => {
                // Reversed to allow for variable aliasing
                let indx = x.iter().rev().position(|y| y.name == name);
                // Re-reverse
                let indx = indx.map(|indx| x.len() - indx - 1);

                let ret = match indx {
                    Some(indx) => Some(ASTVarRef {
                        indx: indx as u8,
                        indx_up: 0,
                    }),
                    None => {
                        let recursive = self.lookup_var(name);

                        recursive.map(|x| ASTVarRef {
                            indx: x.indx,
                            indx_up: x.indx_up + 1,
                        })
                    }
                };

                self.blocks.push(x);

                ret
            }
            None => None,
        }
    }

    pub(super) fn parse_identifier(&mut self, token: Token) -> ASTNode {
        let val = token.value.clone().string();

        let inner = match self.lookup_var(&val) {
            Some(x) => ASTNodeInner::Variable(x),
            None => ASTNodeInner::Type(TypeIdentifier::UnlinkedType(val)),
        };

        ASTNode { token, inner }
    }

    fn expression(&mut self) -> ASTNode {
        self.parse_precedence(&Precedence::ASSIGNMENT)
    }

    fn statement(&mut self) -> ASTNode {
        let ret = self.expression();
        self.consume(TokenType::Terminator);
        if self.error_recovery {
            while !self.advance_if(TokenType::Terminator) {
                if self.advance_if(TokenType::Eof) {
                    return ret;
                }
                self.advance();
            }
        }

        ret
    }

    pub(super) fn parse_unary(&mut self, token: Token) -> ASTNode {
        let token_rule: PrattEntry<T> = get_pratt_entry(&token.token_type);
        let inner = self.parse_precedence(&token_rule.precedence);

        let inner = match token.token_type {
            TokenType::OperatorSub => ASTNodeInner::Negate(Box::new(inner)),
            TokenType::OperatorNot => ASTNodeInner::Not(Box::new(inner)),
            TokenType::OperatorTypeof => ASTNodeInner::Typeof(Box::new(inner)),
            TokenType::OperatorInstanceof => ASTNodeInner::Instanceof(Box::new(inner)),
            _ => unreachable!(),
        };

        ASTNode { token, inner }
    }

    fn parse_field(&mut self) -> Option<ASTNodeDefinitionField> {
        // Access level

        let access_level_token = self.consume_if(&[
            TokenType::KeywordPublic,
            TokenType::KeywordProtected,
            TokenType::KeywordPrivate,
        ]);
        let access_level = match access_level_token.map(|x| x.token_type) {
            Some(TokenType::KeywordPublic) => FieldAccessLevel::Public,
            Some(TokenType::KeywordProtected) => FieldAccessLevel::Protected,
            Some(TokenType::KeywordPrivate) => FieldAccessLevel::Private,
            None => FieldAccessLevel::Protected,
            Some(_) => unreachable!(),
        };

        // Change level

        let change_level_token = self.consume_if(&[
            TokenType::KeywordConst,
            TokenType::KeywordLevel,
            TokenType::KeywordVolatile,
        ]);
        let change_level = match change_level_token.map(|x| x.token_type) {
            Some(TokenType::KeywordConst) => FieldChangeLevel::Const,
            Some(TokenType::KeywordLevel) => FieldChangeLevel::Level,
            Some(TokenType::KeywordVolatile) => FieldChangeLevel::Volatile,
            None => FieldChangeLevel::Volatile,
            Some(_) => unreachable!(),
        };

        // Type

        let type_token = self.consume_hint(TokenType::Identifier, FIELD_HINT);
        let TokenValue::String(type_identifier) = type_token.value else {
            unreachable!()
        };

        // Name

        let name_token = self.consume_hint(TokenType::Identifier, FIELD_HINT);
        let token = name_token.clone();
        let TokenValue::String(name) = name_token.value else {
            unreachable!()
        };

        // Turbofish

        let turbofish_token = self.consume_if(&[TokenType::ConstructTurbofish]);
        let TokenValue::Double(turbofish) = turbofish_token
            .map(|x| x.value)
            .unwrap_or(TokenValue::Double(unsafe { NotNan::new_unchecked(1.0) }))
        else {
            unreachable!()
        };

        // Body

        let mut body = None;
        let mut initial = None;

        if change_level == FieldChangeLevel::Volatile {
            body = Some(self.expression());
            assert_eq!(self.blocks.len(), 0);
            self.consume(TokenType::Terminator);
            if self.consume_if(&[TokenType::OperatorAssign]).is_some() {
                initial = Some(self.expression());
                self.consume(TokenType::Terminator);
            }
        } else if self.consume_if(&[TokenType::Terminator]).is_none() {
            initial = Some(self.expression());
            self.consume(TokenType::Terminator);
        };

        // Compose

        Some(ASTNodeDefinitionField {
            name,
            type_identifier,
            access_level,
            change_level,
            turbofish,
            body,
            token,
            initial,
        })
    }

    fn parse_class(&mut self) -> Option<ASTNodeDefinitionClass> {
        self.consume_if_exist(TokenType::KeywordClass)?;
        let TokenValue::String(name) = self.consume(TokenType::Identifier).value else {
            return None;
        };
        self.consume(TokenType::OpenBrace);
        let mut fields = Vec::new();
        while !self.advance_if(TokenType::CloseBrace) {
            fields.push(self.parse_field()?);
        }

        Some(ASTNodeDefinitionClass { name, fields })
    }
}

pub fn build_ast<T: Iterator<Item = Token>>(token_generator: T) -> Option<AST> {
    let mut builder = ASTBuilder::synthesize_astbuilder(token_generator);
    let mut ast = AST { root: Vec::new() };

    while let Some(x) = builder.parse_class() {
        ast.root.push(x);
    }

    if builder.any_error {
        None
    } else {
        Some(ast)
    }
}

pub fn build_free_expr<T: Iterator<Item = Token>>(token_generator: T) -> Option<ASTNode> {
    let mut builder = ASTBuilder::synthesize_astbuilder(token_generator);

    let x = builder.expression();

    if builder.any_error {
        None
    } else {
        Some(x)
    }
}

#[cfg(test)]
mod tests {
    //   use super::*;
    //   use crate::compiler::scanner::{Token, TokenType, TokenValue};

    //   // Helper function to create a simple token
    //   fn create_token(token_type: TokenType, value: TokenValue) -> Token {
    //       Token {
    //           token_type,
    //           value,
    //           line: 0,
    //           column: 0,
    //           column_s: 0,
    //       }
    //   }

    //   // Test the error_at method
    //   #[test]
    //   fn test_error_at() {
    //       let tokens = vec![
    //           create_token(TokenType::Integer, TokenValue::I64(1)),
    //           create_token(TokenType::Eof, TokenValue::None),
    //       ];
    //       let mut builder = ASTBuilder::synthesize_astbuilder(tokens.into_iter());

    //       let error_node = builder.error_at(
    //           "Test error message",
    //           &create_token(TokenType::Integer, TokenValue::I64(1)),
    //       );
    //       assert_eq!(builder.any_error, true);
    //       assert_eq!(builder.error_recovery, true);
    //       assert!(matches!(error_node.inner, ASTNodeInner::Error));
    //   }

    //   // Test the consume method
    //   #[test]
    //   fn test_consume() {
    //       let tokens = vec![
    //           create_token(TokenType::Integer, TokenValue::I64(1)),
    //           create_token(TokenType::Eof, TokenValue::None),
    //       ];
    //       let mut builder = ASTBuilder::synthesize_astbuilder(tokens.into_iter());

    //       let token = builder.consume(TokenType::Integer);
    //       assert_eq!(token.token_type, TokenType::Integer);
    //       assert_eq!(token.value, TokenValue::I64(1));
    //   }

    //   // Test the parse_literal method
    //   #[test]
    //   fn test_parse_literal() {
    //       let tokens = vec![create_token(TokenType::Eof, TokenValue::None)];
    //       let mut builder = ASTBuilder::synthesize_astbuilder(tokens.into_iter());

    //       let token = create_token(TokenType::Integer, TokenValue::I64(1));
    //       let node = builder.parse_literal(token);
    //       assert_eq!(node.inner, ASTNodeInner::Integer(1))
    //   }

    //   // Test the parse_grouping method
    //   #[test]
    //   fn test_parse_grouping() {
    //       let tokens = vec![
    //           create_token(TokenType::OpenParen, TokenValue::None),
    //           create_token(TokenType::Integer, TokenValue::I64(1)),
    //           create_token(TokenType::CloseParen, TokenValue::None),
    //           create_token(TokenType::Eof, TokenValue::None),
    //       ];
    //       let mut builder = ASTBuilder::synthesize_astbuilder(tokens.into_iter());

    //       let token = builder.token_generator.next().expect("");
    //       let node = builder.parse_grouping(token);
    //       assert_eq!(node.inner, ASTNodeInner::Integer(1))
    //   }

    //   // Test the parse_binary method
    //   #[test]
    //   fn test_parse_binary() {
    //       let tokens = vec![
    //           create_token(TokenType::Integer, TokenValue::I64(1)),
    //           create_token(TokenType::OperatorAdd, TokenValue::None),
    //           create_token(TokenType::Integer, TokenValue::I64(2)),
    //           create_token(TokenType::Eof, TokenValue::None),
    //       ];
    //       let mut builder = ASTBuilder::synthesize_astbuilder(tokens.into_iter());

    //       let l_token = builder.token_generator.next().expect("");
    //       let left_node = builder.parse_literal(l_token);
    //       let token = builder.consume(TokenType::OperatorAdd);
    //       let node = builder.parse_binary(left_node, token);
    //       assert_eq!(
    //           node.inner,
    //           ASTNodeInner::Add(Box::new(ASTNode::Integer(1)), Box::new(ASTNode::Integer(2)),)
    //       );
    //   }

    //   // Test the parse_unary method
    //   #[test]
    //   fn test_parse_unary() {
    //       let tokens = vec![
    //           create_token(TokenType::OperatorSub, TokenValue::None),
    //           create_token(TokenType::Integer, TokenValue::I64(1)),
    //           create_token(TokenType::Eof, TokenValue::None),
    //       ];
    //       let mut builder = ASTBuilder::synthesize_astbuilder(tokens.into_iter());

    //       let token = builder.consume(TokenType::OperatorSub);
    //       let node = builder.parse_unary(token);
    //       assert_eq!(node, ASTNode::Negate(Box::new(ASTNode::Integer(1))))
    //   }

    //   // Test the expression method
    //   #[test]
    //   fn test_expression() {
    //       let tokens = vec![
    //           create_token(TokenType::Integer, TokenValue::I64(1)),
    //           create_token(TokenType::OperatorAdd, TokenValue::None),
    //           create_token(TokenType::Integer, TokenValue::I64(2)),
    //           create_token(TokenType::Eof, TokenValue::None),
    //       ];
    //       let mut builder = ASTBuilder::synthesize_astbuilder(tokens.into_iter());

    //       let node = builder.expression();
    //       println!("{:#?}", node);
    //       assert_eq!(
    //           node,
    //           ASTNode::Add(Box::new(ASTNode::Integer(1)), Box::new(ASTNode::Integer(2)),)
    //       );
    //   }

    //   #[test]
    //   fn test_sample_class() {
    //       let tokens = vec![
    //           create_token(TokenType::KeywordClass, TokenValue::None),
    //           create_token(
    //               TokenType::Identifier,
    //               TokenValue::String("MyClass".to_string()),
    //           ),
    //           create_token(TokenType::OpenBrace, TokenValue::None),
    //           create_token(TokenType::KeywordPublic, TokenValue::None),
    //           create_token(TokenType::Identifier, TokenValue::String("i64".to_string())),
    //           create_token(
    //               TokenType::Identifier,
    //               TokenValue::String("MyInt".to_string()),
    //           ),
    //           create_token(TokenType::OpenBrace, TokenValue::None),
    //           create_token(TokenType::Integer, TokenValue::I64(1)),
    //           create_token(TokenType::Terminator, TokenValue::None),
    //           create_token(TokenType::KeywordThis, TokenValue::None),
    //           create_token(TokenType::OperatorAdd, TokenValue::None),
    //           create_token(TokenType::Integer, TokenValue::I64(1)),
    //           create_token(TokenType::Terminator, TokenValue::None),
    //           create_token(TokenType::CloseBrace, TokenValue::None),
    //           create_token(TokenType::Terminator, TokenValue::None),
    //           create_token(TokenType::CloseBrace, TokenValue::None),
    //       ];

    //       let expected_res = AST {
    //           root: vec![ASTNodeDefinitionClass {
    //               name: "MyClass".to_string(),
    //               fields: vec![ASTNodeDefinitionField {
    //                   name: "MyInt".to_string(),
    //                   type_identifier: "i64".to_string(),
    //                   access_level: FieldAccessLevel::Public,
    //                   change_level: FieldChangeLevel::Volatile,
    //                   turbofish: NotNan::new(1.0).unwrap(),
    //                   body: Some(ASTNode::Brace(ASTBlock {
    //                       variables: vec![],
    //                       exprs: vec![
    //                           ASTNode::Integer(1),
    //                           ASTNode::Add(
    //                               Box::new(ASTNode::Variable(ASTVarRef {
    //                                   indx: 0,
    //                                   indx_up: 1,
    //                               })),
    //                               Box::new(ASTNode::Integer(1)),
    //                           ),
    //                       ],
    //                   })),
    //               }],
    //           }],
    //       };

    //       let res = build_ast(tokens.into_iter()).expect("No AST");

    //       assert_eq!(res, expected_res);
    //   }
}
