use super::{
    ast::*,
    astbuilder::ASTBuilder,
    scanner::{Token, TokenType},
};

type InfixParser<T> = fn(&mut ASTBuilder<T>, ASTNode, Token) -> ASTNode;
type PrefixParser<T> = fn(&mut ASTBuilder<T>, Token) -> ASTNode;

pub struct PrattEntry<T: Iterator<Item = Token>> {
    pub(super) precedence: Precedence,
    pub(super) prefix: Option<PrefixParser<T>>,
    pub(super) infix: Option<InfixParser<T>>,
}

pub const fn get_pratt_entry<T: Iterator<Item = Token>>(token_type: &TokenType) -> PrattEntry<T> {
    match token_type {
        TokenType::Integer => PrattEntry {
            precedence: Precedence::NONE,
            prefix: Some(ASTBuilder::parse_literal),
            infix: None,
        },
        TokenType::String => PrattEntry {
            precedence: Precedence::NONE,
            prefix: Some(ASTBuilder::parse_literal),
            infix: None,
        },
        TokenType::Decimal => PrattEntry {
            precedence: Precedence::NONE,
            prefix: Some(ASTBuilder::parse_literal),
            infix: None,
        },

        TokenType::KeywordIf => PrattEntry {
            precedence: Precedence::NONE,
            prefix: Some(ASTBuilder::parse_if),
            infix: None,
        },
        TokenType::KeywordElse => PrattEntry {
            precedence: Precedence::NONE,
            prefix: None,
            infix: None,
        },
        TokenType::KeywordWhile => PrattEntry {
            precedence: Precedence::NONE,
            prefix: None,
            infix: None,
        },
        TokenType::KeywordForeach => PrattEntry {
            precedence: Precedence::NONE,
            prefix: None,
            infix: None,
        },
        TokenType::KeywordBreak => PrattEntry {
            precedence: Precedence::NONE,
            prefix: None,
            infix: None,
        },
        TokenType::KeywordContinue => PrattEntry {
            precedence: Precedence::NONE,
            prefix: None,
            infix: None,
        },
        TokenType::KeywordReturn => PrattEntry {
            precedence: Precedence::NONE,
            prefix: None,
            infix: None,
        },
        TokenType::KeywordClass => PrattEntry {
            precedence: Precedence::NONE,
            prefix: None,
            infix: None,
        },
        TokenType::KeywordLevel => PrattEntry {
            precedence: Precedence::NONE,
            prefix: None,
            infix: None,
        },
        TokenType::KeywordConst => PrattEntry {
            precedence: Precedence::NONE,
            prefix: None,
            infix: None,
        },
        TokenType::KeywordVolatile => PrattEntry {
            precedence: Precedence::NONE,
            prefix: None,
            infix: None,
        },
        TokenType::KeywordPublic => PrattEntry {
            precedence: Precedence::NONE,
            prefix: None,
            infix: None,
        },
        TokenType::KeywordPrivate => PrattEntry {
            precedence: Precedence::NONE,
            prefix: None,
            infix: None,
        },
        TokenType::KeywordProtected => PrattEntry {
            precedence: Precedence::NONE,
            prefix: None,
            infix: None,
        },
        TokenType::KeywordThis => PrattEntry {
            precedence: Precedence::NONE,
            prefix: Some(ASTBuilder::parse_literal),
            infix: None,
        },
        TokenType::KeywordParent => PrattEntry {
            precedence: Precedence::NONE,
            prefix: Some(ASTBuilder::parse_literal),
            infix: None,
        },
        TokenType::KeywordLet => PrattEntry {
            precedence: Precedence::PRIMARY,
            prefix: Some(ASTBuilder::parse_let),
            infix: None,
        },

        TokenType::ConstTrue => PrattEntry {
            precedence: Precedence::NONE,
            prefix: Some(ASTBuilder::parse_literal),
            infix: None,
        },
        TokenType::ConstFalse => PrattEntry {
            precedence: Precedence::NONE,
            prefix: Some(ASTBuilder::parse_literal),
            infix: None,
        },

        TokenType::ConstructTurbofish => PrattEntry {
            precedence: Precedence::NONE,
            prefix: None,
            infix: None,
        },

        TokenType::OperatorAdd => PrattEntry {
            precedence: Precedence::TERM,
            prefix: None,
            infix: Some(ASTBuilder::parse_binary),
        },
        TokenType::OperatorSub => PrattEntry {
            precedence: Precedence::TERM,
            prefix: Some(ASTBuilder::parse_unary),
            infix: Some(ASTBuilder::parse_binary),
        },
        TokenType::OperatorMul => PrattEntry {
            precedence: Precedence::FACTOR,
            prefix: None,
            infix: Some(ASTBuilder::parse_binary),
        },
        TokenType::OperatorDiv => PrattEntry {
            precedence: Precedence::FACTOR,
            prefix: None,
            infix: Some(ASTBuilder::parse_binary),
        },
        TokenType::OperatorMod => PrattEntry {
            precedence: Precedence::POW,
            prefix: None,
            infix: Some(ASTBuilder::parse_binary),
        },
        TokenType::OperatorPow => PrattEntry {
            precedence: Precedence::POW,
            prefix: None,
            infix: Some(ASTBuilder::parse_binary),
        },
        TokenType::OperatorAssign => PrattEntry {
            precedence: Precedence::ASSIGNMENT,
            prefix: None,
            infix: Some(ASTBuilder::parse_binary),
        },
        TokenType::OperatorEqual => PrattEntry {
            precedence: Precedence::EQUALITY,
            prefix: None,
            infix: Some(ASTBuilder::parse_binary),
        },
        TokenType::OperatorNotEqual => PrattEntry {
            precedence: Precedence::EQUALITY,
            prefix: None,
            infix: Some(ASTBuilder::parse_binary),
        },
        TokenType::OperatorLessThan => PrattEntry {
            precedence: Precedence::COMPARISON,
            prefix: None,
            infix: Some(ASTBuilder::parse_binary),
        },
        TokenType::OperatorLessThanEqual => PrattEntry {
            precedence: Precedence::COMPARISON,
            prefix: None,
            infix: Some(ASTBuilder::parse_binary),
        },
        TokenType::OperatorGreaterThan => PrattEntry {
            precedence: Precedence::COMPARISON,
            prefix: None,
            infix: Some(ASTBuilder::parse_binary),
        },
        TokenType::OperatorGreaterThanEqual => PrattEntry {
            precedence: Precedence::COMPARISON,
            prefix: None,
            infix: Some(ASTBuilder::parse_binary),
        },
        TokenType::OperatorAnd => PrattEntry {
            precedence: Precedence::AND,
            prefix: None,
            infix: Some(ASTBuilder::parse_binary),
        },
        TokenType::OperatorOr => PrattEntry {
            precedence: Precedence::OR,
            prefix: None,
            infix: Some(ASTBuilder::parse_binary),
        },
        TokenType::OperatorNot => PrattEntry {
            precedence: Precedence::UNARY,
            prefix: Some(ASTBuilder::parse_unary),
            infix: None,
        },

        TokenType::OperatorTyper => PrattEntry {
            precedence: Precedence::NONE,
            prefix: None,
            infix: None,
        },
        TokenType::OperatorInner => PrattEntry {
            precedence: Precedence::CALL,
            prefix: None,
            infix: Some(ASTBuilder::parse_inner),
        },
        TokenType::OperatorTypeof => PrattEntry {
            precedence: Precedence::CALL,
            prefix: Some(ASTBuilder::parse_unary),
            infix: None,
        },
        TokenType::OperatorInstanceof => PrattEntry {
            precedence: Precedence::CALL,
            prefix: Some(ASTBuilder::parse_unary),
            infix: None,
        },

        TokenType::OpenBrace => PrattEntry {
            precedence: Precedence::NONE,
            prefix: Some(ASTBuilder::parse_braces),
            infix: None,
        },
        TokenType::CloseBrace => PrattEntry {
            precedence: Precedence::NONE,
            prefix: None,
            infix: None,
        },
        TokenType::OpenParen => PrattEntry {
            precedence: Precedence::NONE,
            prefix: Some(ASTBuilder::parse_grouping),
            infix: None,
        },
        TokenType::CloseParen => PrattEntry {
            precedence: Precedence::NONE,
            prefix: None,
            infix: None,
        },

        TokenType::Comma => PrattEntry {
            precedence: Precedence::NONE,
            prefix: None,
            infix: None,
        },

        TokenType::Terminator => PrattEntry {
            precedence: Precedence::NONE,
            prefix: None,
            infix: None,
        },

        TokenType::Identifier => PrattEntry {
            precedence: Precedence::TERMINATOR,
            prefix: Some(ASTBuilder::parse_identifier),
            infix: None,
        },

        TokenType::Error => PrattEntry {
            precedence: Precedence::NONE,
            prefix: None,
            infix: None,
        },
        TokenType::Eof => PrattEntry {
            precedence: Precedence::NONE,
            prefix: None,
            infix: None,
        },
    }
}
