use ordered_float::NotNan;

use super::{
    ast::{ASTVarRef, ASTVariable},
    r#type::TypeIdentifier,
};

type FASTOptSubexpr = Option<Box<FASTNode>>;
type FASTSubexpr = Box<FASTNode>;

#[derive(Debug, PartialEq, Eq)]
pub struct FASTBlock {
    pub variables: Vec<ASTVariable>,
    pub exprs: Vec<FASTNode>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FASTContent {
    // Constants
    Integer(i64),
    Decimal(NotNan<f64>),
    String(String),
    Type(TypeIdentifier),
    False,
    True,
    // Unary Operators
    Negate(FASTSubexpr),
    Not(FASTSubexpr),
    Variable(ASTVarRef),
    Typeof(FASTSubexpr),
    Instanceof(FASTSubexpr),
    // Binary Operators
    Set(ASTVarRef, FASTSubexpr),
    Add(FASTSubexpr, FASTSubexpr),
    Subtract(FASTSubexpr, FASTSubexpr),
    Multiply(FASTSubexpr, FASTSubexpr),
    Divide(FASTSubexpr, FASTSubexpr),
    Pow(FASTSubexpr, FASTSubexpr),
    Modulus(FASTSubexpr, FASTSubexpr),
    Equal(FASTSubexpr, FASTSubexpr),
    NotEqual(FASTSubexpr, FASTSubexpr),
    Greater(FASTSubexpr, FASTSubexpr),
    Lesser(FASTSubexpr, FASTSubexpr),
    GreaterE(FASTSubexpr, FASTSubexpr),
    LesserE(FASTSubexpr, FASTSubexpr),
    And(FASTSubexpr, FASTSubexpr),
    Or(FASTSubexpr, FASTSubexpr),
    Reference(FASTReference),
    // Multi-expressions
    Brace(FASTBlock),
    // Ternary Operators
    If(FASTSubexpr, FASTSubexpr, FASTOptSubexpr),
}

#[derive(Debug, PartialEq, Eq)]
pub enum FASTReference {
    Root,
    Variable(ASTVarRef),
    Inner(Box<FASTReference>, usize),
}

#[derive(Debug, PartialEq, Eq)]
pub struct FASTNode {
    pub node: FASTContent,
    pub ret_type: TypeIdentifier,
    pub inter_type: TypeIdentifier,
}
