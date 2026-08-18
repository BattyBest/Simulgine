use super::r#type::TypeIdentifier;
use super::scanner::Token;
use num_derive::FromPrimitive;
use num_derive::ToPrimitive;
use ordered_float::NotNan;

type ASTOptSubexpr = Option<Box<ASTNode>>;
type ASTSubexpr = Box<ASTNode>;

#[derive(PartialEq, Debug)]
pub struct ASTNodeDefinitionClass {
    pub(crate) name: String,
    pub(crate) fields: Vec<ASTNodeDefinitionField>,
    pub(crate) props: Vec<ASTDefinitionClassProp>,
}

// Currently equivalent to ASTVariable but don't mind.
#[derive(PartialEq, Debug)]
pub struct ASTDefinitionClassProp {
    pub(crate) name: String,
    pub(crate) t: TypeIdentifier,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FieldAccessLevel {
    Public,
    Protected,
    Private,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FieldChangeLevel {
    Const,
    Level,
    Volatile,
}

#[derive(PartialEq, Debug)]
pub struct ASTNodeDefinitionField {
    pub(crate) name: String,
    pub(crate) type_identifier: String,
    pub(crate) access_level: FieldAccessLevel,
    pub(crate) change_level: FieldChangeLevel,
    pub(crate) turbofish: NotNan<f64>,
    pub(crate) body: Option<ASTNode>,
    pub(crate) initial: Option<ASTNode>,
    pub(crate) token: Token,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ASTVariable {
    pub t: TypeIdentifier,
    pub name: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ASTBlock {
    pub variables: Vec<ASTVariable>,
    pub exprs: Vec<ASTNode>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ASTVarRef {
    pub indx: u8,
    pub indx_up: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ASTNode {
    pub token: Token,
    pub inner: ASTNodeInner,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ASTNodeInner {
    // Constants
    Integer(i64),
    Decimal(NotNan<f64>),
    String(String),
    Type(TypeIdentifier),
    False,
    True,
    // Unary Operators
    Negate(ASTSubexpr),
    Not(ASTSubexpr),
    Variable(ASTVarRef),
    Typeof(ASTSubexpr),
    Instanceof(ASTSubexpr),
    // Binary Operators
    Set(ASTVarRef, ASTSubexpr),
    Add(ASTSubexpr, ASTSubexpr),
    Subtract(ASTSubexpr, ASTSubexpr),
    Multiply(ASTSubexpr, ASTSubexpr),
    Divide(ASTSubexpr, ASTSubexpr),
    Pow(ASTSubexpr, ASTSubexpr),
    Modulus(ASTSubexpr, ASTSubexpr),
    Equal(ASTSubexpr, ASTSubexpr),
    NotEqual(ASTSubexpr, ASTSubexpr),
    Greater(ASTSubexpr, ASTSubexpr),
    Lesser(ASTSubexpr, ASTSubexpr),
    GreaterE(ASTSubexpr, ASTSubexpr),
    LesserE(ASTSubexpr, ASTSubexpr),
    And(ASTSubexpr, ASTSubexpr),
    Or(ASTSubexpr, ASTSubexpr),
    Inner(ASTSubexpr, String),
    // Multi-expressions
    Brace(ASTBlock),
    // Ternary Operators
    If(ASTSubexpr, ASTSubexpr, ASTOptSubexpr),
    // Error
    Error,
}

#[derive(PartialEq, Debug)]
#[allow(clippy::upper_case_acronyms)]
pub struct AST {
    pub(crate) root: Vec<ASTNodeDefinitionClass>,
}

#[derive(FromPrimitive, ToPrimitive)]
#[allow(clippy::upper_case_acronyms)]
pub enum Precedence {
    NONE = 0,
    ASSIGNMENT, // =
    OR,         // ||
    AND,        // &&
    EQUALITY,   // == !=
    COMPARISON, // < > <= >=
    TERM,       // + -
    FACTOR,     // * /
    POW,        // ^ %
    UNARY,      // ! -
    CALL,       // . ()
    PRIMARY,    // let
    TERMINATOR, // ;
}
