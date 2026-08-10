use ordered_float::NotNan;
use std::io::Read;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TokenType {
    Integer,
    String,
    Decimal,

    KeywordIf,
    KeywordElse,
    KeywordWhile,
    KeywordForeach,
    KeywordBreak,
    KeywordContinue,
    KeywordReturn,
    KeywordClass,
    KeywordPublic,
    KeywordPrivate,
    KeywordProtected,
    KeywordConst,
    KeywordLevel,
    KeywordVolatile,
    KeywordThis,
    KeywordParent,

    KeywordLet,

    ConstTrue,
    ConstFalse,

    ConstructTurbofish,

    OperatorAdd,
    OperatorSub,
    OperatorMul,
    OperatorDiv,
    OperatorMod,
    OperatorPow,
    OperatorAssign,
    OperatorEqual,
    OperatorNotEqual,
    OperatorLessThan,
    OperatorLessThanEqual,
    OperatorGreaterThan,
    OperatorGreaterThanEqual,
    OperatorAnd,
    OperatorOr,
    OperatorNot,

    OperatorTyper,
    OperatorInner,
    OperatorTypeof,
    OperatorInstanceof,

    OpenBrace,
    CloseBrace,
    OpenParen,
    CloseParen,

    Terminator,

    Identifier,

    Error,
    Eof,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TokenValue {
    String(String),
    I64(i64),
    Double(NotNan<f64>),
    None,
    // Only used for error tokens
    StringRef(&'static str),
}

impl TokenValue {
    pub fn integer(self) -> i64 {
        match self {
            TokenValue::I64(x) => x,
            _ => unreachable!(),
        }
    }
    pub fn double(self) -> NotNan<f64> {
        match self {
            TokenValue::Double(x) => x,
            _ => unreachable!(),
        }
    }
    pub fn string(self) -> String {
        match self {
            TokenValue::String(x) => x,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub value: TokenValue,
    pub line: usize,
    pub column_s: usize,
    pub column: usize,
}

pub struct FileScanner {
    chars: Vec<char>,
    start: usize,
    cursor: usize,
    line: usize,
    column_s: usize,
    column: usize,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum TokenSynthesizationError {
    IntParseError(std::num::ParseIntError),
    FloatParseError(ordered_float::ParseNotNanError<std::num::ParseFloatError>),
}

impl From<std::num::ParseIntError> for TokenSynthesizationError {
    fn from(value: std::num::ParseIntError) -> Self {
        TokenSynthesizationError::IntParseError(value)
    }
}

impl From<ordered_float::ParseNotNanError<std::num::ParseFloatError>> for TokenSynthesizationError {
    fn from(value: ordered_float::ParseNotNanError<std::num::ParseFloatError>) -> Self {
        TokenSynthesizationError::FloatParseError(value)
    }
}

impl FileScanner {
    pub fn synthesize_filescanner_str(src: &str) -> FileScanner {
        FileScanner {
            chars: src.chars().collect(),
            start: 0,
            cursor: 0,
            line: 1,
            column_s: 1,
            column: 1,
        }
    }

    #[allow(dead_code)] // Used in currently commented-out tests.
    pub fn synthesize_filescanner<T: Read>(src: &mut T) -> FileScanner {
        let mut u8buf = Vec::new();
        if let Err(x) = src.read_to_end(&mut u8buf) {
            eprintln!("Error while reading reader: {}", x)
        }
        FileScanner {
            chars: String::from_utf8_lossy(&u8buf).chars().collect(),
            start: 0,
            cursor: 0,
            line: 1,
            column_s: 1,
            column: 1,
        }
    }

    fn peek(&self) -> Option<&char> {
        self.chars.get(self.cursor)
    }

    fn peek_peek(&self) -> Option<&char> {
        self.chars.get(self.cursor + 1)
    }

    fn can_advance(&self) -> bool {
        self.cursor < self.chars.len()
    }

    fn at_end(&self) -> bool {
        self.cursor == self.chars.len()
    }

    fn advance_line(&mut self) {
        self.line += 1;
        self.column_s = 1;
        self.column = 1;
    }

    fn advance(&mut self) -> Option<&char> {
        if self.can_advance() {
            self.column += 1;
            self.cursor += 1;
            if self.cursor > 0 && self.chars[self.cursor - 1] == '\n' {
                self.advance_line();
            }
            Some(&self.chars[self.cursor - 1])
        } else {
            None
        }
    }

    fn advance_if(&mut self, c: char) -> bool {
        if self.peek() == Some(&c) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn discard_whitespace(&mut self) {
        while let Some(x) = self.peek() {
            match x.is_whitespace() {
                true => {
                    self.advance();
                }
                false => match x {
                    &'/' if self.peek_peek() == Some(&'/') => loop {
                        if self.peek() != Some(&'\n') {
                            self.advance();
                        } else {
                            break;
                        }
                    },
                    _ => break,
                },
            }
        }
    }

    fn synthesize_token(&self, t_type: TokenType) -> Token {
        let val: TokenValue = match t_type {
            TokenType::String => TokenValue::String(String::from_iter(
                &self.chars[self.start + 1..self.cursor - 1],
            )),
            TokenType::Identifier => {
                TokenValue::String(String::from_iter(&self.chars[self.start..self.cursor]))
            }
            _ => TokenValue::None,
        };
        Token {
            token_type: t_type,
            value: val,
            line: self.line,
            column_s: self.column_s,
            column: self.column - 1,
        }
    }

    fn synthesize_num_token(&self, t_type: TokenType) -> Result<Token, TokenSynthesizationError> {
        let val: TokenValue = match t_type {
            TokenType::Integer => String::from_iter(&self.chars[self.start..self.cursor])
                .parse::<i64>()
                .map(TokenValue::I64)?,
            TokenType::Decimal => String::from_iter(&self.chars[self.start..self.cursor])
                .parse::<NotNan<f64>>()
                .map(TokenValue::Double)?,
            _ => TokenValue::None,
        };
        Ok(Token {
            token_type: t_type,
            value: val,
            line: self.line,
            column_s: self.column_s,
            column: self.column - 1,
        })
    }

    pub fn error_token(&self, msg: &str) -> Token {
        Token {
            token_type: TokenType::Error,
            value: TokenValue::String(msg.to_string()),
            line: self.line,
            column_s: self.column_s,
            column: self.column - 1,
        }
    }

    pub const fn empty_error_token(msg: &'static str) -> Token {
        Token {
            token_type: TokenType::Error,
            value: TokenValue::StringRef(msg),
            line: 0,
            column_s: 0,
            column: 0,
        }
    }

    fn parse_number(&mut self) -> Token {
        let mut is_decimal = false;

        while self.peek().is_some_and(|x| x.is_ascii_digit()) {
            self.advance();
        }

        if self.peek().is_some_and(|x| x == &'.') {
            is_decimal = true;
            self.advance();
            while self.peek().is_some_and(|x| x.is_ascii_digit()) {
                self.advance();
            }
        }

        let res = self.synthesize_num_token(if is_decimal {
            TokenType::Decimal
        } else {
            TokenType::Integer
        });

        if let Ok(x) = res {
            x
        } else if let Err(x) = res {
            self.error_token(format!("{:?}", x).as_str())
        } else {
            unreachable!()
        }
    }

    fn check_identifier(&self) -> TokenType {
        match &self.chars[self.start..self.cursor] {
            x if x == ['i', 'f'] => TokenType::KeywordIf,
            x if x == ['e', 'l', 's', 'e'] => TokenType::KeywordElse,
            x if x == ['w', 'h', 'i', 'l', 'e'] => TokenType::KeywordWhile,
            x if x == ['f', 'o', 'r', 'e', 'a', 'c', 'h'] => TokenType::KeywordForeach,
            x if x == ['b', 'r', 'e', 'a', 'k'] => TokenType::KeywordBreak,
            x if x == ['c', 'o', 'n', 't', 'i', 'n', 'u', 'e'] => TokenType::KeywordContinue,
            x if x == ['r', 'e', 't', 'u', 'r', 'n'] => TokenType::KeywordReturn,
            x if x == ['c', 'l', 'a', 's', 's'] => TokenType::KeywordClass,
            x if x == ['p', 'u', 'b', 'l', 'i', 'c'] => TokenType::KeywordPublic,
            x if x == ['p', 'r', 'i', 'v', 'a', 't', 'e'] => TokenType::KeywordPrivate,
            x if x == ['p', 'r', 'o', 't', 'e', 'c', 't', 'e', 'd'] => TokenType::KeywordProtected,
            x if x == ['c', 'o', 'n', 's', 't'] => TokenType::KeywordConst,
            x if x == ['l', 'e', 'v', 'e', 'l'] => TokenType::KeywordLevel,
            x if x == ['v', 'o', 'l', 'a', 't', 'i', 'l', 'e'] => TokenType::KeywordVolatile,
            x if x == ['t', 'h', 'i', 's'] => TokenType::KeywordThis,
            x if x == ['p', 'a', 'r', 'e', 'n', 't'] => TokenType::KeywordParent,
            x if x == ['l', 'e', 't'] => TokenType::KeywordLet,
            x if x == ['t', 'y', 'p', 'e', 'o', 'f'] => TokenType::OperatorTypeof,
            x if x == ['i', 'n', 's', 't', 'a', 'n', 'c', 'e', 'o', 'f'] => {
                TokenType::OperatorInstanceof
            }
            x if x == ['f', 'a', 'l', 's', 'e'] => TokenType::ConstFalse,
            x if x == ['t', 'r', 'u', 'e'] => TokenType::ConstTrue,
            _ => TokenType::Identifier,
        }
    }

    fn parse_identifier(&mut self) -> Token {
        while self
            .peek()
            .is_some_and(|x| x.is_alphanumeric() || x == &'_' || x == &'-')
        {
            self.advance();
        }

        self.synthesize_token(self.check_identifier())
    }

    fn parse_string(&mut self, quote_char: char) -> Token {
        while self.peek().is_some_and(|x| *x != quote_char) {
            self.advance();
        }
        self.advance();

        self.synthesize_token(TokenType::String)
    }

    fn parse_turbofish(&mut self) -> Token {
        if !self.advance_if('<') {
            return self.error_token("Unexpected token.");
        }
        let start_marker = self.cursor;
        while self.peek().is_some_and(|x| *x != '>') {
            if self.peek().is_some_and(|x| x.is_ascii_digit() || x == &'.') {
                self.advance();
            } else {
                return self.error_token("Unexpected token.");
            }
        }
        let end_marker = self.cursor;
        self.advance();
        let inner_num = String::from_iter(&self.chars[start_marker..end_marker])
            .parse::<NotNan<f64>>()
            .unwrap();

        let mut ret = self.synthesize_token(TokenType::ConstructTurbofish);
        ret.value = TokenValue::Double(inner_num);

        ret
    }

    fn parse(&mut self) -> Option<Token> {
        self.discard_whitespace();
        if self.at_end() {
            return Some(Token {
                token_type: TokenType::Eof,
                value: TokenValue::None,
                line: self.line,
                column_s: self.column_s,
                column: self.column,
            });
        }
        self.start = self.cursor;
        self.column_s = self.column;
        #[allow(clippy::manual_map)]
        match self.advance() {
            Some(x) => Some(match *x {
                '"' => self.parse_string('"'),
                '\'' => self.parse_string('\''),
                '`' => self.parse_string('`'),
                '+' => self.synthesize_token(TokenType::OperatorAdd),
                '-' => self.synthesize_token(TokenType::OperatorSub),
                '*' => self.synthesize_token(TokenType::OperatorMul),
                '/' => self.synthesize_token(TokenType::OperatorDiv),
                '%' => self.synthesize_token(TokenType::OperatorMod),
                '{' => self.synthesize_token(TokenType::OpenBrace),
                '}' => self.synthesize_token(TokenType::CloseBrace),
                '(' => self.synthesize_token(TokenType::OpenParen),
                ')' => self.synthesize_token(TokenType::CloseParen),
                ';' => self.synthesize_token(TokenType::Terminator),
                '=' => match self.advance_if('=') {
                    true => self.synthesize_token(TokenType::OperatorEqual),
                    false => self.synthesize_token(TokenType::OperatorAssign),
                },
                '<' => match self.advance_if('=') {
                    true => self.synthesize_token(TokenType::OperatorLessThanEqual),
                    false => self.synthesize_token(TokenType::OperatorLessThan),
                },
                '>' => match self.advance_if('=') {
                    true => self.synthesize_token(TokenType::OperatorGreaterThanEqual),
                    false => self.synthesize_token(TokenType::OperatorGreaterThan),
                },
                '!' => match self.advance_if('=') {
                    true => self.synthesize_token(TokenType::OperatorNotEqual),
                    false => self.synthesize_token(TokenType::OperatorNot),
                },
                '&' => match self.advance_if('&') {
                    true => self.synthesize_token(TokenType::OperatorAnd),
                    false => self.error_token("Unexpected token."),
                },
                '|' => match self.advance_if('|') {
                    true => self.synthesize_token(TokenType::OperatorOr),
                    false => self.error_token("Unexpected token."),
                },
                ':' => match self.advance_if(':') {
                    true => self.parse_turbofish(),
                    false => self.synthesize_token(TokenType::OperatorTyper),
                },
                '^' => self.synthesize_token(TokenType::OperatorPow),
                '.' => self.synthesize_token(TokenType::OperatorInner),
                _ if x.is_ascii_digit() => self.parse_number(),
                _ if x.is_alphanumeric() => self.parse_identifier(),
                '_' => self.parse_identifier(),
                _ => self.error_token("Unexpected token."),
            }),
            None => None,
        }
    }
}

impl Iterator for FileScanner {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.parse()
    }
}

mod test_compiler_scanner {
    #[allow(unused_imports)]
    use ordered_float::NotNan;

    #[allow(unused_imports)]
    use crate::compiler::scanner::{FileScanner, TokenType, TokenValue};

    #[test]
    fn test_single_number() -> Result<(), &'static str> {
        let mut scanner = FileScanner::synthesize_filescanner_str("2");
        assert_eq!(scanner.start, 0);
        assert_eq!(scanner.cursor, 0);
        assert_eq!(scanner.chars, vec!['2']);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Integer);
        assert_eq!(res.value, TokenValue::I64(2));
        assert_eq!(res.line, 1);
        assert_eq!(res.column, 1);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Eof);
        assert_eq!(res.value, TokenValue::None);
        assert_eq!(res.line, 1);
        assert_eq!(res.column, 2);

        Ok(())
    }

    #[test]
    fn test_single_string() -> Result<(), &'static str> {
        let mut scanner = FileScanner::synthesize_filescanner_str("\"Hello, World!\"");
        assert_eq!(scanner.start, 0);
        assert_eq!(scanner.cursor, 0);
        assert_eq!(
            scanner.chars,
            vec!['"', 'H', 'e', 'l', 'l', 'o', ',', ' ', 'W', 'o', 'r', 'l', 'd', '!', '"']
        );

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::String);
        assert_eq!(res.value, TokenValue::String("Hello, World!".to_string()));
        assert_eq!(res.line, 1);
        assert_eq!(res.column_s, 1);
        assert_eq!(res.column, 15);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Eof);
        assert_eq!(res.value, TokenValue::None);
        assert_eq!(res.line, 1);
        assert_eq!(res.column, 16);

        Ok(())
    }

    #[test]
    fn test_single_identifier() -> Result<(), &'static str> {
        let mut scanner = FileScanner::synthesize_filescanner_str("hello_world");
        assert_eq!(scanner.start, 0);
        assert_eq!(scanner.cursor, 0);
        assert_eq!(
            scanner.chars,
            vec!['h', 'e', 'l', 'l', 'o', '_', 'w', 'o', 'r', 'l', 'd']
        );

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Identifier);
        assert_eq!(res.value, TokenValue::String("hello_world".to_string()));
        assert_eq!(res.line, 1);
        assert_eq!(res.column_s, 1);
        assert_eq!(res.column, 11);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Eof);
        assert_eq!(res.value, TokenValue::None);
        assert_eq!(res.line, 1);
        assert_eq!(res.column, 12);

        Ok(())
    }

    #[test]
    fn test_single_decimal() -> Result<(), &'static str> {
        let mut scanner = FileScanner::synthesize_filescanner_str("3.15");
        assert_eq!(scanner.start, 0);
        assert_eq!(scanner.cursor, 0);
        assert_eq!(scanner.chars, vec!['3', '.', '1', '5']);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Decimal);
        assert_eq!(res.value, TokenValue::Double(NotNan::new(3.15).unwrap()));
        assert_eq!(res.line, 1);
        assert_eq!(res.column_s, 1);
        assert_eq!(res.column, 4);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Eof);
        assert_eq!(res.value, TokenValue::None);
        assert_eq!(res.line, 1);
        assert_eq!(res.column, 5);

        Ok(())
    }

    #[test]
    fn test_single_operator() -> Result<(), &'static str> {
        let mut scanner = FileScanner::synthesize_filescanner_str("+-*/%=<>^:");
        assert_eq!(scanner.start, 0);
        assert_eq!(scanner.cursor, 0);
        assert_eq!(
            scanner.chars,
            vec!['+', '-', '*', '/', '%', '=', '<', '>', '^', ':']
        );

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::OperatorAdd);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::OperatorSub);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::OperatorMul);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::OperatorDiv);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::OperatorMod);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::OperatorAssign);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::OperatorLessThan);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::OperatorGreaterThan);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::OperatorPow);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::OperatorTyper);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Eof);
        assert_eq!(res.value, TokenValue::None);

        Ok(())
    }

    #[test]
    fn test_single_keyword() -> Result<(), &'static str> {
        let mut scanner = FileScanner::synthesize_filescanner_str(
            "if else while foreach break continue return class const level volatile public private protected this parent",
        );

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::KeywordIf);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::KeywordElse);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::KeywordWhile);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::KeywordForeach);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::KeywordBreak);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::KeywordContinue);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::KeywordReturn);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::KeywordClass);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::KeywordConst);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::KeywordLevel);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::KeywordVolatile);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::KeywordPublic);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::KeywordPrivate);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::KeywordProtected);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::KeywordThis);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::KeywordParent);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Eof);
        assert_eq!(res.value, TokenValue::None);

        Ok(())
    }

    #[test]
    fn test_single_turbofish() -> Result<(), &'static str> {
        let mut scanner = FileScanner::synthesize_filescanner_str("::<1.0>");
        assert_eq!(scanner.start, 0);
        assert_eq!(scanner.cursor, 0);
        assert_eq!(scanner.chars, vec![':', ':', '<', '1', '.', '0', '>']);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::ConstructTurbofish);
        assert_eq!(res.value, TokenValue::Double(NotNan::new(1.0).unwrap()));

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Eof);
        assert_eq!(res.value, TokenValue::None);

        Ok(())
    }

    #[test]
    fn test_single_error() -> Result<(), &'static str> {
        let mut scanner = FileScanner::synthesize_filescanner_str("?");
        assert_eq!(scanner.start, 0);
        assert_eq!(scanner.cursor, 0);
        assert_eq!(scanner.chars, vec!['?']);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Error);
        assert_eq!(
            res.value,
            TokenValue::String("Unexpected token.".to_string())
        );

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Eof);
        assert_eq!(res.value, TokenValue::None);

        Ok(())
    }

    #[test]
    fn test_single_comment() -> Result<(), &'static str> {
        let mut scanner = FileScanner::synthesize_filescanner_str("// Hello, World!\n2");
        assert_eq!(scanner.start, 0);
        assert_eq!(scanner.cursor, 0);
        assert_eq!(
            scanner.chars,
            vec![
                '/', '/', ' ', 'H', 'e', 'l', 'l', 'o', ',', ' ', 'W', 'o', 'r', 'l', 'd', '!',
                '\n', '2'
            ]
        );

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Integer);
        assert_eq!(res.value, TokenValue::I64(2));

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Eof);
        assert_eq!(res.value, TokenValue::None);

        Ok(())
    }

    #[test]
    fn test_single_whitespace() -> Result<(), &'static str> {
        let mut scanner = FileScanner::synthesize_filescanner_str(" 2 ");
        assert_eq!(scanner.start, 0);
        assert_eq!(scanner.cursor, 0);
        assert_eq!(scanner.chars, vec![' ', '2', ' ']);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Integer);
        assert_eq!(res.value, TokenValue::I64(2));
        assert_eq!(res.column, 2);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Eof);
        assert_eq!(res.value, TokenValue::None);

        Ok(())
    }

    #[test]
    fn test_single_eof() -> Result<(), &'static str> {
        let mut scanner = FileScanner::synthesize_filescanner_str("");
        assert_eq!(scanner.start, 0);
        assert_eq!(scanner.cursor, 0);
        assert_eq!(scanner.chars, vec![]);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Eof);
        assert_eq!(res.value, TokenValue::None);

        Ok(())
    }

    #[test]
    fn test_assignment_statement() -> Result<(), &'static str> {
        let mut scanner = FileScanner::synthesize_filescanner_str("x = 2");
        assert_eq!(scanner.start, 0);
        assert_eq!(scanner.cursor, 0);
        assert_eq!(scanner.chars, vec!['x', ' ', '=', ' ', '2']);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Identifier);
        assert_eq!(res.value, TokenValue::String("x".to_string()));

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::OperatorAssign);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Integer);
        assert_eq!(res.value, TokenValue::I64(2));

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Eof);
        assert_eq!(res.value, TokenValue::None);

        Ok(())
    }

    #[test]
    fn test_conditional_return_statement() -> Result<(), &'static str> {
        let mut scanner = FileScanner::synthesize_filescanner_str("if x == 2 { return 1.0 }");
        assert_eq!(scanner.start, 0);
        assert_eq!(scanner.cursor, 0);
        assert_eq!(
            scanner.chars,
            vec![
                'i', 'f', ' ', 'x', ' ', '=', '=', ' ', '2', ' ', '{', ' ', 'r', 'e', 't', 'u',
                'r', 'n', ' ', '1', '.', '0', ' ', '}'
            ]
        );

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::KeywordIf);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Identifier);
        assert_eq!(res.value, TokenValue::String("x".to_string()));

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::OperatorEqual);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Integer);
        assert_eq!(res.value, TokenValue::I64(2));

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::OpenBrace);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::KeywordReturn);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Decimal);
        assert_eq!(res.value, TokenValue::Double(NotNan::new(1.0).unwrap()));

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::CloseBrace);
        assert_eq!(res.value, TokenValue::None);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Eof);
        assert_eq!(res.value, TokenValue::None);

        Ok(())
    }

    #[test]
    fn test_level_turbofishd_field_class() -> Result<(), &'static str> {
        let mut scanner = FileScanner::synthesize_filescanner_str(
            "\
class MyClass {
    level i64 myInt::<4.3>;
    level f64 myFloat::<3.15>;
}",
        );

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::KeywordClass);
        assert_eq!(res.value, TokenValue::None);
        assert_eq!(res.line, 1);
        assert_eq!(res.column_s, 1);
        assert_eq!(res.column, 5);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Identifier);
        assert_eq!(res.value, TokenValue::String("MyClass".to_string()));
        assert_eq!(res.line, 1);
        assert_eq!(res.column_s, 7);
        assert_eq!(res.column, 13);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::OpenBrace);
        assert_eq!(res.value, TokenValue::None);
        assert_eq!(res.line, 1);
        assert_eq!(res.column, 15);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::KeywordLevel);
        assert_eq!(res.value, TokenValue::None);
        assert_eq!(res.line, 2);
        assert_eq!(res.column_s, 5);
        assert_eq!(res.column, 9);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Identifier);
        assert_eq!(res.value, TokenValue::String("i64".to_string()));
        assert_eq!(res.line, 2);
        assert_eq!(res.column_s, 11);
        assert_eq!(res.column, 13);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Identifier);
        assert_eq!(res.value, TokenValue::String("myInt".to_string()));
        assert_eq!(res.line, 2);
        assert_eq!(res.column_s, 15);
        assert_eq!(res.column, 19);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::ConstructTurbofish);
        assert_eq!(res.value, TokenValue::Double(NotNan::new(4.3).unwrap()));
        assert_eq!(res.line, 2);
        assert_eq!(res.column_s, 20);
        assert_eq!(res.column, 26);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Terminator);
        assert_eq!(res.value, TokenValue::None);
        assert_eq!(res.line, 2);
        assert_eq!(res.column, 27);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::KeywordLevel);
        assert_eq!(res.value, TokenValue::None);
        assert_eq!(res.line, 3);
        assert_eq!(res.column_s, 5);
        assert_eq!(res.column, 9);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Identifier);
        assert_eq!(res.value, TokenValue::String("f64".to_string()));
        assert_eq!(res.line, 3);
        assert_eq!(res.column_s, 11);
        assert_eq!(res.column, 13);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Identifier);
        assert_eq!(res.value, TokenValue::String("myFloat".to_string()));
        assert_eq!(res.line, 3);
        assert_eq!(res.column_s, 15);
        assert_eq!(res.column, 21);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::ConstructTurbofish);
        assert_eq!(res.value, TokenValue::Double(NotNan::new(3.15).unwrap()));
        assert_eq!(res.line, 3);
        assert_eq!(res.column_s, 22);
        assert_eq!(res.column, 29);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Terminator);
        assert_eq!(res.value, TokenValue::None);
        assert_eq!(res.line, 3);
        assert_eq!(res.column, 30);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::CloseBrace);
        assert_eq!(res.value, TokenValue::None);
        assert_eq!(res.line, 4);
        assert_eq!(res.column, 1);

        let res = scanner.next().ok_or("No token.")?;
        assert_eq!(res.token_type, TokenType::Eof);
        assert_eq!(res.value, TokenValue::None);
        assert_eq!(res.line, 4);
        assert_eq!(res.column, 2);

        Ok(())
    }
}
