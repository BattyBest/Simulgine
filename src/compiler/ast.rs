use super::scanner::Token;
use super::simulgine_inst::Simulgine;
use super::simulgine_inst::UserClassIndx;
use super::simulgine_inst::UserObject;
use num_derive::FromPrimitive;
use num_derive::ToPrimitive;
use ordered_float::NotNan;

type ASTOptSubexpr = Option<Box<ASTNode>>;
type ASTSubexpr = Box<ASTNode>;
type FASTOptSubexpr = Option<Box<FASTNode>>;
type FASTSubexpr = Box<FASTNode>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    UserClass,
    UnlinkedType,
    Type,
    Error,
    I64,
    I32,
    I16,
    I8,
    U64,
    U32,
    U16,
    U8,
    Float,
    Double,
    String,
    Boolean,
    None,
}

#[derive(Debug, Clone)]
pub enum TypeIdentifier {
    UserClass(UserClassIndx),
    UnlinkedType(String),
    Type(Box<TypeIdentifier>),
    Error,
    I64,
    I32,
    I16,
    I8,
    U64,
    U32,
    U16,
    U8,
    Float,
    Double,
    String,
    Boolean,
    None,
}

impl From<TypeIdentifier> for Type {
    fn from(value: TypeIdentifier) -> Self {
        return (&value).into();
    }
}

impl From<&TypeIdentifier> for Type {
    fn from(value: &TypeIdentifier) -> Self {
        match value {
            TypeIdentifier::UserClass(_) => Type::UserClass,
            TypeIdentifier::UnlinkedType(_) => Type::UnlinkedType,
            TypeIdentifier::Type(_) => Type::Type,
            TypeIdentifier::Error => Type::Error,
            TypeIdentifier::I64 => Type::I64,
            TypeIdentifier::I32 => Type::I32,
            TypeIdentifier::I16 => Type::I16,
            TypeIdentifier::I8 => Type::I8,
            TypeIdentifier::U64 => Type::U64,
            TypeIdentifier::U32 => Type::U32,
            TypeIdentifier::U16 => Type::U16,
            TypeIdentifier::U8 => Type::U8,
            TypeIdentifier::Float => Type::Float,
            TypeIdentifier::Double => Type::Double,
            TypeIdentifier::String => Type::String,
            TypeIdentifier::Boolean => Type::Boolean,
            TypeIdentifier::None => Type::None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeIdentifierToTypeError;

impl TryFrom<&Type> for TypeIdentifier {
    fn try_from(value: &Type) -> Result<TypeIdentifier, TypeIdentifierToTypeError> {
        match value {
            Type::UserClass => Err(TypeIdentifierToTypeError),
            Type::UnlinkedType => Err(TypeIdentifierToTypeError),
            Type::Type => Err(TypeIdentifierToTypeError),
            Type::Error => Ok(TypeIdentifier::Error),
            Type::I64 => Ok(TypeIdentifier::I64),
            Type::I32 => Ok(TypeIdentifier::I32),
            Type::I16 => Ok(TypeIdentifier::I16),
            Type::I8 => Ok(TypeIdentifier::I8),
            Type::U64 => Ok(TypeIdentifier::U64),
            Type::U32 => Ok(TypeIdentifier::U32),
            Type::U16 => Ok(TypeIdentifier::U16),
            Type::U8 => Ok(TypeIdentifier::U8),
            Type::Float => Ok(TypeIdentifier::Float),
            Type::Double => Ok(TypeIdentifier::Double),
            Type::String => Ok(TypeIdentifier::String),
            Type::Boolean => Ok(TypeIdentifier::Boolean),
            Type::None => Ok(TypeIdentifier::None),
        }
    }

    type Error = TypeIdentifierToTypeError;
}

pub static EQUALLABLETYPES: [Type; 13] = [
    Type::UserClass,
    Type::U8,
    Type::U16,
    Type::U32,
    Type::U64,
    Type::I8,
    Type::I16,
    Type::I32,
    Type::I64,
    Type::Float,
    Type::Double,
    Type::String,
    Type::Boolean,
];

pub static TYPES: [Type; 17] = [
    Type::UserClass,
    Type::UnlinkedType,
    Type::Type,
    Type::Error,
    Type::U8,
    Type::U16,
    Type::U32,
    Type::U64,
    Type::I8,
    Type::I16,
    Type::I32,
    Type::I64,
    Type::Float,
    Type::Double,
    Type::String,
    Type::Boolean,
    Type::None,
];

pub static NUMBERS: [Type; 10] = [
    Type::U8,
    Type::U16,
    Type::U32,
    Type::U64,
    Type::I8,
    Type::I16,
    Type::I32,
    Type::I64,
    Type::Float,
    Type::Double,
];

pub static SIGNED_NUMBERS: [Type; 6] = [
    Type::I8,
    Type::I16,
    Type::I32,
    Type::I64,
    Type::Float,
    Type::Double,
];

pub static INTEGER_NUMBERS: [Type; 8] = [
    Type::U8,
    Type::U16,
    Type::U32,
    Type::U64,
    Type::I8,
    Type::I16,
    Type::I32,
    Type::I64,
];

pub static FLOATING_NUMBERS: [Type; 2] = [Type::Float, Type::Double];

impl TypeIdentifier {
    pub fn is_assignable_from(&self, x: &TypeIdentifier) -> bool {
        match self {
            TypeIdentifier::None => true,
            TypeIdentifier::UserClass(weak) => {
                if let TypeIdentifier::UserClass(other) = x {
                    weak == other
                } else {
                    false
                }
            }
            TypeIdentifier::Type(t) => {
                if let TypeIdentifier::Type(other) = x {
                    **t == **other
                } else {
                    false
                }
            }
            TypeIdentifier::Error => false,
            TypeIdentifier::UnlinkedType(_) => false,
            TypeIdentifier::I64 => [
                &TypeIdentifier::I64,
                &TypeIdentifier::I32,
                &TypeIdentifier::I16,
                &TypeIdentifier::I8,
                &TypeIdentifier::U32,
                &TypeIdentifier::U16,
                &TypeIdentifier::U8,
            ]
            .contains(&x),
            TypeIdentifier::I32 => [
                &TypeIdentifier::I32,
                &TypeIdentifier::I16,
                &TypeIdentifier::I8,
                &TypeIdentifier::U16,
                &TypeIdentifier::U8,
            ]
            .contains(&x),
            TypeIdentifier::I16 => [
                &TypeIdentifier::I16,
                &TypeIdentifier::I8,
                &TypeIdentifier::U8,
            ]
            .contains(&x),
            TypeIdentifier::I8 => x == &TypeIdentifier::I8,
            TypeIdentifier::U64 => [
                &TypeIdentifier::U64,
                &TypeIdentifier::U32,
                &TypeIdentifier::U16,
                &TypeIdentifier::U8,
            ]
            .contains(&x),
            TypeIdentifier::U32 => [
                &TypeIdentifier::U32,
                &TypeIdentifier::U16,
                &TypeIdentifier::U8,
            ]
            .contains(&x),
            TypeIdentifier::U16 => [&TypeIdentifier::U16, &TypeIdentifier::U8].contains(&x),
            TypeIdentifier::U8 => [&TypeIdentifier::U8].contains(&x),
            TypeIdentifier::Float => x == &TypeIdentifier::Float,
            TypeIdentifier::Double => {
                [&TypeIdentifier::Double, &TypeIdentifier::Float].contains(&x)
            }
            TypeIdentifier::String => x == &TypeIdentifier::String,
            TypeIdentifier::Boolean => x == &TypeIdentifier::Boolean,
        }
    }

    pub fn to_debug_string(&self, sim: &Simulgine) -> String {
        match self {
            TypeIdentifier::UserClass(user_class_indx) => sim
                .get_user_class(*user_class_indx)
                .map_or("<invalid>".to_owned(), |x| x.to_debug_string(sim)),
            _ => self.to_string(sim),
        }
    }

    pub fn to_string(&self, sim: &Simulgine) -> String {
        match self {
            TypeIdentifier::UserClass(user_class_indx) => sim
                .get_user_class(*user_class_indx)
                .map_or("<invalid>".to_owned(), |x| x.to_string(sim)),
            TypeIdentifier::UnlinkedType(_) => "<unlinked>".into(),
            TypeIdentifier::Type(type_identifier) => {
                format!("Type({})", type_identifier.to_string(sim))
            }
            TypeIdentifier::Error => "<error>".to_owned(),
            TypeIdentifier::I64 => "i64".to_owned(),
            TypeIdentifier::I32 => "i32".to_owned(),
            TypeIdentifier::I16 => "i16".to_owned(),
            TypeIdentifier::I8 => "i8".to_owned(),
            TypeIdentifier::U64 => "u64".to_owned(),
            TypeIdentifier::U32 => "u32".to_owned(),
            TypeIdentifier::U16 => "u16".to_owned(),
            TypeIdentifier::U8 => "u8".to_owned(),
            TypeIdentifier::Float => "float".to_owned(),
            TypeIdentifier::Double => "double".to_owned(),
            TypeIdentifier::String => "string".to_owned(),
            TypeIdentifier::Boolean => "boolean".to_owned(),
            TypeIdentifier::None => "<none>".to_owned(),
        }
    }
}

impl Type {
    pub fn is_assignable_from(&self, x: &Type) -> bool {
        match self {
            Type::None => true,
            Type::UserClass => x == &Type::UserClass,
            Type::Type => x == &Type::Type,
            Type::Error => false,
            Type::UnlinkedType => false,
            Type::I64 => [
                &Type::I64,
                &Type::I32,
                &Type::I16,
                &Type::I8,
                &Type::U32,
                &Type::U16,
                &Type::U8,
            ]
            .contains(&x),
            Type::I32 => [&Type::I32, &Type::I16, &Type::I8, &Type::U16, &Type::U8].contains(&x),
            Type::I16 => [&Type::I16, &Type::I8, &Type::U8].contains(&x),
            Type::I8 => x == &Type::I8,
            Type::U64 => [&Type::U64, &Type::U32, &Type::U16, &Type::U8].contains(&x),
            Type::U32 => [&Type::U32, &Type::U16, &Type::U8].contains(&x),
            Type::U16 => [&Type::U16, &Type::U8].contains(&x),
            Type::U8 => [&Type::U8].contains(&x),
            Type::Float => x == &Type::Float,
            Type::Double => [&Type::Double, &Type::Float].contains(&x),
            Type::String => x == &Type::String,
            Type::Boolean => x == &Type::Boolean,
        }
    }
}

impl PartialEq for TypeIdentifier {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::UserClass(l0), Self::UserClass(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
impl Eq for TypeIdentifier {}

#[derive(Clone, Debug)]
pub enum TypeInstance {
    UserClass(UserClassInst),
    Type(TypeInst),
    I64(I64Inst),
    I32(I32Inst),
    I16(I16Inst),
    I8(I8Inst),
    U64(U64Inst),
    U32(U32Inst),
    U16(U16Inst),
    U8(U8Inst),
    Float(FloatInst),
    Double(DoubleInst),
    String(StringInst),
    Boolean(BooleanInst),
    None,
}

#[derive(Clone, Debug)]
pub enum TypeReference<'a> {
    Instance(TypeInstance),
    UserClassRO(UserClassROInst<'a>),
}

#[derive(Clone, Debug)]
pub struct BooleanInst {
    pub val: bool,
}

#[derive(Clone, Debug)]
pub struct StringInst {
    pub val: String,
}

#[derive(Clone, Debug)]
pub struct DoubleInst {
    pub val: f64,
}

impl From<FloatInst> for DoubleInst {
    fn from(value: FloatInst) -> Self {
        DoubleInst {
            val: value.val as f64,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FloatInst {
    pub val: f32,
}

#[derive(Clone, Debug)]
pub struct U8Inst {
    pub val: u8,
}

#[derive(Clone, Debug)]
pub struct U16Inst {
    pub val: u16,
}

impl From<U8Inst> for U16Inst {
    fn from(value: U8Inst) -> Self {
        U16Inst {
            val: value.val as u16,
        }
    }
}

#[derive(Clone, Debug)]
pub struct U32Inst {
    pub val: u32,
}

impl From<U16Inst> for U32Inst {
    fn from(value: U16Inst) -> Self {
        U32Inst {
            val: value.val as u32,
        }
    }
}

#[derive(Clone, Debug)]
pub struct U64Inst {
    pub val: u64,
}

impl From<U32Inst> for U64Inst {
    fn from(value: U32Inst) -> Self {
        U64Inst {
            val: value.val as u64,
        }
    }
}

#[derive(Clone, Debug)]
pub struct I8Inst {
    pub val: i8,
}

#[derive(Clone, Debug)]
pub struct I16Inst {
    pub val: i16,
}

impl From<I8Inst> for I16Inst {
    fn from(value: I8Inst) -> Self {
        I16Inst {
            val: value.val as i16,
        }
    }
}

impl From<U8Inst> for I16Inst {
    fn from(value: U8Inst) -> Self {
        I16Inst {
            val: value.val as i16,
        }
    }
}

#[derive(Clone, Debug)]
pub struct I32Inst {
    pub val: i32,
}

impl From<I16Inst> for I32Inst {
    fn from(value: I16Inst) -> Self {
        I32Inst {
            val: value.val as i32,
        }
    }
}

impl From<U16Inst> for I32Inst {
    fn from(value: U16Inst) -> Self {
        I32Inst {
            val: value.val as i32,
        }
    }
}

#[derive(Clone, Debug)]
pub struct I64Inst {
    pub val: i64,
}

impl From<I32Inst> for I64Inst {
    fn from(value: I32Inst) -> Self {
        I64Inst {
            val: value.val as i64,
        }
    }
}

impl From<U32Inst> for I64Inst {
    fn from(value: U32Inst) -> Self {
        I64Inst {
            val: value.val as i64,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TypeInst {
    pub val: TypeIdentifier,
}

#[derive(Clone, Debug)]
pub struct UserClassInst {
    pub val: UserObject,
}

#[derive(Clone, Debug)]
pub struct UserClassROInst<'a> {
    pub val: &'a UserObject,
}

impl TypeInstance {
    pub fn as_type(value: &Self) -> Type {
        match value {
            TypeInstance::UserClass(_) => Type::UserClass,
            TypeInstance::Type(_) => Type::Type,
            TypeInstance::I64(_) => Type::I64,
            TypeInstance::I32(_) => Type::I32,
            TypeInstance::I16(_) => Type::I16,
            TypeInstance::I8(_) => Type::I8,
            TypeInstance::U64(_) => Type::U64,
            TypeInstance::U32(_) => Type::U32,
            TypeInstance::U16(_) => Type::U16,
            TypeInstance::U8(_) => Type::U8,
            TypeInstance::Float(_) => Type::Float,
            TypeInstance::Double(_) => Type::Double,
            TypeInstance::String(_) => Type::String,
            TypeInstance::Boolean(_) => Type::Boolean,
            TypeInstance::None => Type::None,
        }
    }

    pub fn as_type_identifier(value: &Self) -> TypeIdentifier {
        match value {
            TypeInstance::UserClass(x) => TypeIdentifier::UserClass(x.val.class.clone()),
            TypeInstance::Type(x) => TypeIdentifier::Type(Box::new(x.val.clone())),
            TypeInstance::I64(_) => TypeIdentifier::I64,
            TypeInstance::I32(_) => TypeIdentifier::I32,
            TypeInstance::I16(_) => TypeIdentifier::I16,
            TypeInstance::I8(_) => TypeIdentifier::I8,
            TypeInstance::U64(_) => TypeIdentifier::U64,
            TypeInstance::U32(_) => TypeIdentifier::U32,
            TypeInstance::U16(_) => TypeIdentifier::U16,
            TypeInstance::U8(_) => TypeIdentifier::U8,
            TypeInstance::Float(_) => TypeIdentifier::Float,
            TypeInstance::Double(_) => TypeIdentifier::Double,
            TypeInstance::String(_) => TypeIdentifier::String,
            TypeInstance::Boolean(_) => TypeIdentifier::Boolean,
            TypeInstance::None => TypeIdentifier::None,
        }
    }

    pub fn coerce_number_int(&self) -> Option<i128> {
        match self {
            TypeInstance::UserClass(_) => None,
            TypeInstance::Type(_) => None,
            TypeInstance::I64(x) => Some(x.val as i128),
            TypeInstance::I32(x) => Some(x.val as i128),
            TypeInstance::I16(x) => Some(x.val as i128),
            TypeInstance::I8(x) => Some(x.val as i128),
            TypeInstance::U64(x) => Some(x.val as i128),
            TypeInstance::U32(x) => Some(x.val as i128),
            TypeInstance::U16(x) => Some(x.val as i128),
            TypeInstance::U8(x) => Some(x.val as i128),
            TypeInstance::Float(_) => None,
            TypeInstance::Double(_) => None,
            TypeInstance::String(_) => None,
            TypeInstance::Boolean(_) => None,
            TypeInstance::None => None,
        }
    }

    pub fn coerce_number_float(&self) -> Option<f64> {
        match self {
            TypeInstance::UserClass(_) => None,
            TypeInstance::Type(_) => None,
            TypeInstance::I64(_) => None,
            TypeInstance::I32(_) => None,
            TypeInstance::I16(_) => None,
            TypeInstance::I8(_) => None,
            TypeInstance::U64(_) => None,
            TypeInstance::U32(_) => None,
            TypeInstance::U16(_) => None,
            TypeInstance::U8(_) => None,
            TypeInstance::Float(x) => Some(x.val as f64),
            TypeInstance::Double(x) => Some(x.val as f64),
            TypeInstance::String(_) => None,
            TypeInstance::Boolean(_) => None,
            TypeInstance::None => None,
        }
    }

    pub fn coerce_boolean(&self) -> Option<bool> {
        match self {
            TypeInstance::UserClass(_) => None,
            TypeInstance::Type(_) => None,
            TypeInstance::I64(_) => None,
            TypeInstance::I32(_) => None,
            TypeInstance::I16(_) => None,
            TypeInstance::I8(_) => None,
            TypeInstance::U64(_) => None,
            TypeInstance::U32(_) => None,
            TypeInstance::U16(_) => None,
            TypeInstance::U8(_) => None,
            TypeInstance::Float(_) => None,
            TypeInstance::Double(_) => None,
            TypeInstance::String(_) => None,
            TypeInstance::Boolean(x) => Some(x.val),
            TypeInstance::None => None,
        }
    }

    pub fn to_debug_string(&self, sim: &Simulgine) -> String {
        let inner = match self {
            TypeInstance::UserClass(user_class_inst) => user_class_inst.val.to_debug_string(sim),
            TypeInstance::Type(type_inst) => type_inst.val.to_debug_string(sim),
            _ => self.to_string(sim),
        };

        let type_str = TypeInstance::as_type_identifier(self).to_string(sim);

        format!("[{}] {}", type_str, inner)
    }

    pub fn to_string(&self, sim: &Simulgine) -> String {
        match self {
            TypeInstance::UserClass(user_class_inst) => user_class_inst.val.to_string(sim),
            TypeInstance::Type(type_inst) => type_inst.val.to_string(sim),
            TypeInstance::I64(i64_inst) => i64_inst.val.to_string(),
            TypeInstance::I32(i32_inst) => i32_inst.val.to_string(),
            TypeInstance::I16(i16_inst) => i16_inst.val.to_string(),
            TypeInstance::I8(i8_inst) => i8_inst.val.to_string(),
            TypeInstance::U64(u64_inst) => u64_inst.val.to_string(),
            TypeInstance::U32(u32_inst) => u32_inst.val.to_string(),
            TypeInstance::U16(u16_inst) => u16_inst.val.to_string(),
            TypeInstance::U8(u8_inst) => u8_inst.val.to_string(),
            TypeInstance::Float(float_inst) => float_inst.val.to_string(),
            TypeInstance::Double(double_inst) => double_inst.val.to_string(),
            TypeInstance::String(string_inst) => format!("\"{}\"", string_inst.val),
            TypeInstance::Boolean(boolean_inst) => boolean_inst.val.to_string(),
            TypeInstance::None => "None".to_string(),
        }
    }
}

impl TypeReference<'_> {
    pub fn as_type(value: &Self) -> Type {
        match value {
            TypeReference::Instance(type_instance) => TypeInstance::as_type(type_instance),
            TypeReference::UserClassRO(_user_class_roinst) => Type::UserClass,
        }
    }

    pub fn as_type_identifier(value: &Self) -> TypeIdentifier {
        match value {
            TypeReference::Instance(type_instance) => {
                TypeInstance::as_type_identifier(type_instance)
            }
            TypeReference::UserClassRO(user_class_roinst) => {
                TypeIdentifier::UserClass(user_class_roinst.val.class)
            }
        }
    }

    pub fn coerce_number_int(&self) -> Option<i128> {
        match self {
            TypeReference::Instance(type_instance) => type_instance.coerce_number_int(),
            TypeReference::UserClassRO(_user_class_roinst) => None,
        }
    }

    pub fn coerce_number_float(&self) -> Option<f64> {
        match self {
            TypeReference::Instance(type_instance) => type_instance.coerce_number_float(),
            TypeReference::UserClassRO(_user_class_roinst) => None,
        }
    }

    pub fn coerce_boolean(&self) -> Option<bool> {
        match self {
            TypeReference::Instance(type_instance) => type_instance.coerce_boolean(),
            TypeReference::UserClassRO(_user_class_roinst) => None,
        }
    }

    pub fn to_debug_string(&self, sim: &Simulgine) -> String {
        let inner = match self {
            TypeReference::Instance(type_instance) => return type_instance.to_debug_string(sim),
            TypeReference::UserClassRO(user_class_roinst) => {
                user_class_roinst.val.to_debug_string(sim)
            }
        };

        let type_str = TypeReference::as_type_identifier(self).to_string(sim);

        format!("[{}] {}", type_str, inner)
    }

    pub fn to_string(&self, sim: &Simulgine) -> String {
        match self {
            TypeReference::Instance(type_instance) => type_instance.to_string(sim),
            TypeReference::UserClassRO(user_class_roinst) => user_class_roinst.val.to_string(sim),
        }
    }
}

impl PartialEq for TypeInstance {
    fn eq(&self, other: &Self) -> bool {
        if INTEGER_NUMBERS.contains(&TypeInstance::as_type(self))
            && INTEGER_NUMBERS.contains(&TypeInstance::as_type(other))
        {
            return self.coerce_number_int().unwrap() == other.coerce_number_int().unwrap();
        }
        if FLOATING_NUMBERS.contains(&TypeInstance::as_type(self))
            && FLOATING_NUMBERS.contains(&TypeInstance::as_type(other))
        {
            return self.coerce_number_float().unwrap() == other.coerce_number_float().unwrap();
        }

        match (self, other) {
            (Self::UserClass(_), Self::UserClass(_)) => false,
            (Self::Type(l0), Self::Type(r0)) => l0.val == r0.val,
            (Self::String(l0), Self::String(r0)) => l0.val == r0.val,
            (Self::Boolean(l0), Self::Boolean(r0)) => l0.val == r0.val,
            _ => false,
        }
    }
}

impl PartialEq for TypeReference<'_> {
    fn eq(&self, other: &Self) -> bool {
        match self {
            TypeReference::Instance(type_instance) => match other {
                TypeReference::Instance(type_instance1) => type_instance.eq(type_instance1),
                TypeReference::UserClassRO(_user_class_roinst) => false,
            },
            TypeReference::UserClassRO(user_class_roinst) => match other {
                TypeReference::Instance(_type_instance) => false,
                TypeReference::UserClassRO(user_class_roinst1) => {
                    user_class_roinst.val == user_class_roinst1.val
                }
            },
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct ASTNodeDefinitionClass {
    pub(crate) name: String,
    pub(crate) fields: Vec<ASTNodeDefinitionField>,
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
    ROOT,
    Variable(ASTVarRef),
    Inner(Box<FASTReference>, usize),
}

#[derive(Debug, PartialEq, Eq)]
pub struct FASTNode {
    pub node: FASTContent,
    pub ret_type: TypeIdentifier,
    pub inter_type: TypeIdentifier,
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
pub struct AST {
    pub(crate) root: Vec<ASTNodeDefinitionClass>,
}

#[derive(FromPrimitive, ToPrimitive)]
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
