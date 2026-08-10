use std::{collections::HashMap, fmt::write, format, sync::Mutex, write};

use rayon::iter::{
    FromParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};

use crate::compiler::scanner::FileScanner;

use super::{
    ast::*,
    runner::SimulgineErrors,
    scanner::Token,
    simulgine_inst::{Simulgine, UserClass, UserClassIndx, UserClassMember},
};

fn find_type(str: &str, user_classes: &HashMap<String, UserClassIndx>) -> Option<TypeIdentifier> {
    match str {
        "i64" => Some(TypeIdentifier::I64),
        "i32" => Some(TypeIdentifier::I32),
        "i16" => Some(TypeIdentifier::I16),
        "i8" => Some(TypeIdentifier::I8),
        "u64" => Some(TypeIdentifier::U64),
        "u32" => Some(TypeIdentifier::U32),
        "u16" => Some(TypeIdentifier::U16),
        "u8" => Some(TypeIdentifier::U8),
        "float" => Some(TypeIdentifier::Float),
        "double" => Some(TypeIdentifier::Double),
        "string" => Some(TypeIdentifier::String),
        "bool" => Some(TypeIdentifier::Boolean),
        _ => Some(TypeIdentifier::UserClass(*user_classes.get(str)?)),
    }
}

fn link_field(
    f: &ASTNodeDefinitionField,
    class_name: &str,
    user_classes: &HashMap<String, UserClassIndx>,
) -> Result<UserClassMember> {
    let t = find_type(&f.type_identifier, user_classes);

    if let Some(t) = t {
        Ok(UserClassMember {
            t: t.clone(),
            class: user_classes.get(&class_name.to_string()).unwrap().clone(),
            body: None,
            name: f.name.clone(),
            stage: f.turbofish,
            access_level: f.access_level,
            change_level: f.change_level,
        })
    } else {
        Err(ASTLinkError {
            token: f.token.clone(),
            err: ASTLinkErrorInner::InvalidFieldType(
                f.name.clone(),
                class_name.to_string(),
                f.type_identifier.clone(),
            ),
        })
    }
}

fn link_field_body(
    f: &UserClassMember,
    parent: UserClassIndx,
    user_classes_map: &HashMap<String, UserClassIndx>,
    user_classes: &[UserClass],
    node: Option<ASTNode>,
) -> Option<std::result::Result<FASTNode, ASTLinkError>> {
    return if let Some(x) = node {
        assert_eq!(f.change_level, FieldChangeLevel::Volatile);
        match link_ast_node(
            x,
            LinkerInfo {
                classes_map: user_classes_map,
                cur_class: Some(&parent),
                classes: user_classes,
            },
            &[&[
                &ASTVariable {
                    t: f.t.clone(),
                    name: "this".into(),
                },
                &ASTVariable {
                    t: TypeIdentifier::UserClass(parent.clone()),
                    name: "parent".into(),
                },
            ]],
            &[f.t.clone().into()],
        ) {
            Ok(x) => {
                if f.t.is_assignable_from(&x.ret_type) {
                    Some(Ok(x))
                } else {
                    let err = format!("Type mismatch, field {} has body that returns type {:#?} but has type {:#?}", f.name, x.ret_type, f.t).leak();
                    Some(Err(ASTLinkError {
                        token: FileScanner::empty_error_token(err),
                        err: ASTLinkErrorInner::Err,
                    }))
                }
            }
            Err(x) => Some(Err(x)),
        }
    } else {
        assert!(
            f.change_level == FieldChangeLevel::Const || f.change_level == FieldChangeLevel::Level
        );
        None
    };
}
pub fn link_ast(ast: AST) -> std::result::Result<Simulgine, SimulgineErrors> {
    let error: Mutex<Vec<ASTLinkError>> = Mutex::new(vec![]);

    let class_defs = ast.root;
    // Dont say I didnt tell you, let the jugglingationing begin!
    let classes_map =
        HashMap::<String, UserClassIndx>::from_par_iter(class_defs.par_iter().map(|x| {
            (
                x.name.clone(),
                UserClassIndx {
                    0: class_defs.iter().position(|y| y == x).unwrap(),
                },
            )
        }));
    if !classes_map.contains_key("ROOT") {
        (*error.lock().unwrap()).push(ASTLinkError {
            token: FileScanner::empty_error_token(
                "Class ROOT is not defined. Don't confuse me, what am I supposed to start on?!?!?",
            ),
            err: ASTLinkErrorInner::Err,
        });
        return Err(SimulgineErrors::LinkError(error.into_inner().unwrap()));
    }
    let fields: Vec<(String, Vec<(UserClassMember, Option<ASTNode>)>)> = class_defs
        .into_iter()
        .map(|x| {
            let fields = x
                .fields
                .into_par_iter()
                .map(|f| (link_field(&f, &x.name, &classes_map), f.body))
                .filter_map(|x| {
                    if x.0.is_ok() {
                        Some(x)
                    } else {
                        (*error.lock().unwrap()).push(x.0.unwrap_err());
                        None
                    }
                })
                .map(|x| (x.0.unwrap(), x.1))
                .collect();
            (x.name, fields)
        })
        .collect();

    if (*error.lock().unwrap()).len() != 0 {
        return Err(SimulgineErrors::LinkError(error.into_inner().unwrap()));
    }

    let mut baked_classes: Vec<UserClass> = Vec::new();

    let mut nodes: HashMap<String, HashMap<String, Option<ASTNode>>> = HashMap::new();

    for class_def in fields.into_iter() {
        // Check we don't have this class already
        if baked_classes
            .iter()
            .find(|x| x.name == class_def.0)
            .is_some()
        {
            (*error.lock().unwrap()).push(ASTLinkError {
                token: FileScanner::empty_error_token(
                    format!(
                        "Class {} is defined twice (or thrice) (or more-ice).",
                        &class_def.0
                    )
                    .leak(), // We can't proceed after an error anyway, so just leak it.
                ),
                err: ASTLinkErrorInner::Err,
            });
            continue;
        }
        // Retrieve class
        let mut class = UserClass {
            name: class_def.0.clone(),
            field_names: HashMap::with_capacity(class_def.1.len()),
            field_stages: Vec::new(),
            fields: Vec::with_capacity(class_def.1.len()),
        };
        let mut i: usize = 0;

        // Retrieve its fields
        let mut fs = class_def
            .1
            .into_iter()
            .map(|x| {
                let a = x.0.name.clone();
                (x.0, (a, x.1))
            })
            .collect::<Vec<_>>();

        fs.sort_by(|a, b| a.0.stage.total_cmp(&b.0.stage));

        let staged = (0..fs.len())
            .collect::<Vec<_>>()
            .chunk_by(|a, b| fs.get(*a).unwrap().0.stage == fs.get(*b).unwrap().0.stage)
            .map(|x| x.iter().map(|y| y.clone()).collect::<Vec<_>>())
            .collect::<Vec<_>>();

        class.field_stages = staged;

        let (fields, node): (Vec<UserClassMember>, HashMap<String, Option<ASTNode>>) =
            fs.into_iter().unzip();

        for a in fields {
            class.field_names.insert(a.name.clone(), i);
            class.fields.push(a);
            i += 1;
        }

        nodes.insert(class.name.clone(), node);
        baked_classes.push(class);

        if (*error.lock().unwrap()).len() != 0 {
            return Err(SimulgineErrors::LinkError(error.into_inner().unwrap()));
        }
    }

    let mut baked_nodes: HashMap<String, HashMap<String, Option<FASTNode>>> = HashMap::new();

    for class in baked_classes.iter() {
        let mut f_nodes = HashMap::new();
        let mut nodes = nodes.remove(&class.name).unwrap();
        for a in class.fields.iter() {
            let node = nodes.remove(&a.name).unwrap();
            let f_node = link_field_body(
                a,
                classes_map.get(&class.name).unwrap().clone(),
                &classes_map,
                &baked_classes,
                node,
            );
            if f_node.as_ref().is_some_and(|x| x.is_err()) {
                (*error.lock().unwrap()).push(f_node.unwrap().unwrap_err());
                continue;
            }
            f_nodes.insert(a.name.clone(), f_node.map(|x| x.unwrap()));
        }
        baked_nodes.insert(class.name.clone(), f_nodes);
    }

    if (*error.lock().unwrap()).len() != 0 {
        return Err(SimulgineErrors::LinkError(error.into_inner().unwrap()));
    }

    for class in baked_classes.iter_mut() {
        let mut fs = baked_nodes.remove(&class.name).unwrap();
        for a in class.fields.iter_mut() {
            a.body = fs.remove(&a.name).unwrap();
        }
    }

    Ok(Simulgine {
        user_classes: baked_classes,
        user_class_names: classes_map,
    })
}

type Result<T> = std::result::Result<T, ASTLinkError>;
type VariableII<'a, 'b> = [&'a [&'b ASTVariable]];

#[derive(Debug, Clone, Copy)]
pub(crate) struct LinkerInfo<'a> {
    pub(crate) classes_map: &'a HashMap<String, UserClassIndx>,
    pub(crate) cur_class: Option<&'a UserClassIndx>,
    pub(crate) classes: &'a [UserClass],
}

fn ast_linking_need_x_unary(
    info: LinkerInfo,
    astnode_expr: Box<ASTNode>,
    c: fn(Box<FASTNode>) -> FASTContent,
    x: &[Type],
    variables: &VariableII,
) -> Result<FASTNode> {
    let token = astnode_expr.token.clone();
    let inner = link_ast_node(*astnode_expr, info, variables, x)?;
    if x.contains(&(&(&inner).ret_type).into()) {
        Ok(FASTNode {
            ret_type: inner.ret_type.clone(),
            inter_type: inner.ret_type.clone(),
            node: c(Box::new(inner)),
        })
    } else {
        Err(ASTLinkError {
            token,
            err: ASTLinkErrorInner::MismatchedType(inner.ret_type.into(), x.into()),
        })
    }
}

fn ast_linking_need_x_number(
    info: LinkerInfo,
    astnode_expr: Box<ASTNode>,
    astnode_expr1: Box<ASTNode>,
    c: fn(Box<FASTNode>, Box<FASTNode>) -> FASTContent,
    x: &[Type],
    variables: &VariableII,
) -> Result<FASTNode> {
    let t = astnode_expr.token.clone();
    let t1 = astnode_expr1.token.clone();
    let inner = link_ast_node(*astnode_expr, info, variables, x)?;
    let inner1 = link_ast_node(*astnode_expr1, info, variables, x)?;
    let t = if inner.ret_type.is_assignable_from(&inner1.ret_type) {
        inner.ret_type.clone()
    } else if inner1.ret_type.is_assignable_from(&inner.ret_type) {
        inner1.ret_type.clone()
    } else {
        return Err(ASTLinkError {
            token: t,
            err: ASTLinkErrorInner::UncovertibleTypes(
                inner.ret_type.into(),
                inner1.ret_type.into(),
            ),
        });
    };
    if x.contains(&t.clone().into()) {
        Ok(FASTNode {
            node: c(Box::new(inner), Box::new(inner1)),
            inter_type: t.clone(),
            ret_type: t,
        })
    } else {
        Err(ASTLinkError {
            token: t1,
            err: ASTLinkErrorInner::MismatchedType(t.into(), x.into()),
        })
    }
}

fn ast_linking_need_x_compare(
    info: LinkerInfo,
    astnode_expr: Box<ASTNode>,
    astnode_expr1: Box<ASTNode>,
    c: fn(Box<FASTNode>, Box<FASTNode>) -> FASTContent,
    x: &[Type],
    variables: &VariableII,
) -> Result<FASTNode> {
    let t = astnode_expr.token.clone();
    let t1 = astnode_expr1.token.clone();
    let inner = link_ast_node(*astnode_expr, info, variables, x)?;
    let inner1 = link_ast_node(*astnode_expr1, info, variables, x)?;
    let t = if inner.ret_type.is_assignable_from(&inner1.ret_type) {
        inner.ret_type.clone()
    } else if inner1.ret_type.is_assignable_from(&inner.ret_type) {
        inner1.ret_type.clone()
    } else {
        return Err(ASTLinkError {
            token: t,
            err: ASTLinkErrorInner::UncovertibleTypes(
                inner.ret_type.into(),
                inner1.ret_type.into(),
            ),
        });
    };
    if x.contains(&t.clone().into()) {
        Ok(FASTNode {
            node: c(Box::new(inner), Box::new(inner1)),
            inter_type: t,
            ret_type: TypeIdentifier::Boolean,
        })
    } else {
        Err(ASTLinkError {
            token: t1,
            err: ASTLinkErrorInner::MismatchedType(t.into(), x.into()),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ASTLinkError {
    token: Token,
    err: ASTLinkErrorInner,
}

#[derive(Debug, Clone)]
pub enum ASTLinkErrorInner {
    MismatchedType(Type, Box<[Type]>),
    MismatchedTypes(Box<[Type]>, Box<[Type]>),
    UncovertibleTypes(Type, Type),
    NonexistentVT(String),
    NonexistentField(String, String),
    SubError(Vec<ASTLinkError>),
    InvalidFieldType(String, String, String),
    AccessDenied(String, String),
    NonexistentVariable,
    Err,
    EmptyBrace,
    ErrorToken,
}

impl std::fmt::Display for ASTLinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = write!(f, "In Token: {:#?}", &self.token);
        if res.is_err() {
            return res;
        }
        match &self.err {
            ASTLinkErrorInner::MismatchedType(t, items) => {
                write!(f, "Expected one of types {:#?}, but got {:#?}", items, t)
            }
            ASTLinkErrorInner::MismatchedTypes(t, items) => {
                write!(f, "Expected one of types {:#?}, but got {:#?}", items, t)
            }
            ASTLinkErrorInner::UncovertibleTypes(t1, t2) => write!(
                f,
                "The types {:#?} and {:#?} can not be converted to a common type",
                t1, t2
            ),
            ASTLinkErrorInner::NonexistentVT(type_identifier) => {
                write!(f, "Type {} does not exist.", type_identifier)
            }
            ASTLinkErrorInner::NonexistentField(class_name, type_identifier) => {
                write!(
                    f,
                    "Class {} has no field named {}.",
                    class_name, type_identifier
                )
            }
            ASTLinkErrorInner::InvalidFieldType(field_name, class_name, t) => {
                write!(
                    f,
                    "Field '{}' of class '{}' has an invalid type of '{}'.",
                    field_name, class_name, t
                )
            }
            ASTLinkErrorInner::SubError(astlink_errors) => write!(
                f,
                "{}",
                astlink_errors
                    .iter()
                    .map(|x| format!("{}", x))
                    .collect::<Vec<String>>()
                    .join("\n")
            ),
            ASTLinkErrorInner::AccessDenied(class, field) => write!(
                f,
                "Can not access field {} of class {} from outside the class due to its access level.",
                field,
                class
                ),
            ASTLinkErrorInner::NonexistentVariable => write!(f, "Tried to access a non-existent variable. You most likely tried using parent or this in the REPL."),
            ASTLinkErrorInner::Err => write!(f, "An error."),
            ASTLinkErrorInner::EmptyBrace => write!(f, "Braces were empty."),
            ASTLinkErrorInner::ErrorToken => write!(f, "Encountered an error taken."),
        }
    }
}

impl std::error::Error for ASTLinkError {}

pub fn link_ast_node(
    node: ASTNode,
    info: LinkerInfo,
    variables: &VariableII,
    exp_type: &[Type],
) -> Result<FASTNode> {
    let token = node.token.clone();
    let ret = match node.inner {
        ASTNodeInner::Integer(x) => {
            #[rustfmt::skip]
            let t = exp_type
                .iter()
                .filter(|a| match a {
                    Type::I64 =>    i64::MIN        <= x        &&  x        <= i64::MAX,
                    Type::I32 =>    i32::MIN as i64 <= x        &&  x        <= i32::MAX as i64,
                    Type::I16 =>    i16::MIN as i64 <= x        &&  x        <= i16::MAX as i64,
                    Type::I8  =>    i8::MIN  as i64 <= x        &&  x        <= i8::MAX  as i64,
                    Type::U64 =>    u64::MIN        <= x as u64 &&  x as u64 <= u64::MAX,
                    Type::U32 =>    u32::MIN as u64 <= x as u64 &&  x as u64 <= u32::MAX as u64,
                    Type::U16 =>    u16::MIN as u64 <= x as u64 &&  x as u64 <= u16::MAX as u64,
                    Type::U8  =>    u8::MIN  as u64 <= x as u64 &&  x as u64 <= u8::MAX  as u64,
                    _ => false,
                })
                .next()
                .ok_or(ASTLinkError{
                    token: node.token,
                    err: ASTLinkErrorInner::MismatchedTypes(exp_type.into(), INTEGER_NUMBERS.clone().into())
                })?;

            Ok(FASTNode {
                node: FASTContent::Integer(x),
                inter_type: t.try_into().unwrap(),
                ret_type: t.try_into().unwrap(),
            })
        }
        ASTNodeInner::Decimal(x) => {
            #[rustfmt::skip]
            let t = exp_type
                .iter()
                .filter(|a| match a {
                    Type::Double => f64::MIN        <= Into::<f64>::into(x)        &&  Into::<f64>::into(x)        <= f64::MAX,
                    Type::Float  => f32::MIN as f32 <= Into::<f64>::into(x) as f32 &&  Into::<f64>::into(x) as f32 <= f32::MAX as f32,
                    _ => false,
                })
                .next()
                .ok_or(ASTLinkError{
                    token: node.token,
                    err: ASTLinkErrorInner::MismatchedTypes(exp_type.into(), FLOATING_NUMBERS.clone().into())
                })?;

            Ok(FASTNode {
                node: FASTContent::Decimal(x),
                inter_type: t.try_into().unwrap(),
                ret_type: t.try_into().unwrap(),
            })
        }
        ASTNodeInner::String(x) => Ok(FASTNode {
            node: FASTContent::String(x),
            inter_type: TypeIdentifier::String,
            ret_type: TypeIdentifier::String,
        }),
        ASTNodeInner::True => Ok(FASTNode {
            node: FASTContent::True,
            ret_type: TypeIdentifier::Boolean,
            inter_type: TypeIdentifier::Boolean,
        }),
        ASTNodeInner::False => Ok(FASTNode {
            node: FASTContent::False,
            ret_type: TypeIdentifier::Boolean,
            inter_type: TypeIdentifier::Boolean,
        }),
        ASTNodeInner::Negate(astnode_expr) => ast_linking_need_x_unary(
            info,
            astnode_expr,
            FASTContent::Negate,
            &SIGNED_NUMBERS,
            variables,
        ),
        ASTNodeInner::Not(astnode_expr) => ast_linking_need_x_unary(
            info,
            astnode_expr,
            FASTContent::Not,
            &[Type::Boolean],
            variables,
        ),
        ASTNodeInner::Variable(astvar_ref) => {
            if (variables.len() as u8 - astvar_ref.indx_up) < 1 {
                return Err(ASTLinkError {
                    token,
                    err: ASTLinkErrorInner::NonexistentVariable,
                });
            }
            let var = variables.get((variables.len() as u8 - astvar_ref.indx_up) as usize - 1);
            if let None = var {
                return Err(ASTLinkError {
                    token,
                    err: ASTLinkErrorInner::NonexistentVariable,
                });
            }
            let var = var.unwrap().get(astvar_ref.indx as usize);
            if let None = var {
                return Err(ASTLinkError {
                    token,
                    err: ASTLinkErrorInner::NonexistentVariable,
                });
            }
            let var = var.unwrap();

            let t = var.t.clone();

            Ok(FASTNode {
                node: FASTContent::Variable(astvar_ref),
                inter_type: t.clone(),
                ret_type: t,
            })
        }
        ASTNodeInner::Typeof(astnode_expr) => {
            let inner = link_ast_node(*astnode_expr, info, variables, exp_type)?;
            let t = TypeIdentifier::Type(Box::new(inner.ret_type.clone()));

            Ok(FASTNode {
                node: FASTContent::Typeof(Box::new(inner)),
                ret_type: t.clone(),
                inter_type: t,
            })
        }
        ASTNodeInner::Instanceof(astnode_expr) => {
            let inner = link_ast_node(*astnode_expr, info, variables, exp_type)?;
            if let TypeIdentifier::Type(x) = inner.ret_type.clone() {
                Ok(FASTNode {
                    node: FASTContent::Instanceof(Box::new(inner)),
                    ret_type: *x.clone(),
                    inter_type: *x,
                })
            } else {
                Err(ASTLinkError {
                    token: node.token,
                    err: ASTLinkErrorInner::MismatchedType(
                        inner.ret_type.into(),
                        Box::new([Type::Type]),
                    ),
                })
            }
        }
        ASTNodeInner::Type(type_identifier) => {
            let x = match &type_identifier {
                TypeIdentifier::UnlinkedType(x) => x,
                _ => {
                    unreachable!(
                        "ASTNodeInner::Type is supposed to have unlinked type during linking."
                    )
                }
            };

            if x.to_lowercase().as_str() == "root" {
                let root = find_type("ROOT", &info.classes_map).unwrap();
                return Ok(FASTNode {
                    node: FASTContent::Reference(FASTReference::ROOT),
                    ret_type: root.clone(),
                    inter_type: root,
                });
            }

            let t = find_type(x, &info.classes_map);

            match t {
                Some(x) => Ok(FASTNode {
                    node: FASTContent::Type(x.clone()),
                    inter_type: TypeIdentifier::Type(Box::new(x.clone())),
                    ret_type: TypeIdentifier::Type(Box::new(x)),
                }),
                None => Err(ASTLinkError {
                    token: node.token,
                    err: ASTLinkErrorInner::NonexistentVT(x.clone()),
                }),
            }
        }
        ASTNodeInner::Set(astvar_ref, astnode_expr1) => {
            let var = variables
                .get((variables.len() as u8 - astvar_ref.indx_up) as usize - 1)
                .unwrap()
                .get(astvar_ref.indx as usize)
                .unwrap();

            let expr = link_ast_node(*astnode_expr1, info, variables, &[var.t.clone().into()])?;

            if var.t.is_assignable_from(&expr.ret_type) {
                Ok(FASTNode {
                    inter_type: var.t.clone(),
                    node: FASTContent::Set(astvar_ref, Box::new(expr)),
                    ret_type: var.t.clone(),
                })
            } else {
                Err(ASTLinkError {
                    token: node.token,
                    err: ASTLinkErrorInner::UncovertibleTypes(
                        var.t.clone().into(),
                        expr.ret_type.into(),
                    ),
                })
            }
        }
        ASTNodeInner::Add(astnode_expr, astnode_expr1) => ast_linking_need_x_number(
            info,
            astnode_expr,
            astnode_expr1,
            FASTContent::Add,
            &NUMBERS,
            variables,
        ),
        ASTNodeInner::Subtract(astnode_expr, astnode_expr1) => ast_linking_need_x_number(
            info,
            astnode_expr,
            astnode_expr1,
            FASTContent::Subtract,
            &NUMBERS,
            variables,
        ),
        ASTNodeInner::Multiply(astnode_expr, astnode_expr1) => ast_linking_need_x_number(
            info,
            astnode_expr,
            astnode_expr1,
            FASTContent::Multiply,
            &NUMBERS,
            variables,
        ),
        ASTNodeInner::Divide(astnode_expr, astnode_expr1) => ast_linking_need_x_number(
            info,
            astnode_expr,
            astnode_expr1,
            FASTContent::Divide,
            &NUMBERS,
            variables,
        ),
        ASTNodeInner::Pow(astnode_expr, astnode_expr1) => {
            let expr1 = link_ast_node(*astnode_expr, info, variables, &NUMBERS)?;
            let expr2 = link_ast_node(*astnode_expr1, info, variables, &NUMBERS)?;

            if !NUMBERS.contains(&expr1.ret_type.clone().into()) {
                return Err(ASTLinkError {
                    token: node.token,
                    err: ASTLinkErrorInner::MismatchedType(
                        expr1.ret_type.clone().into(),
                        Box::new(NUMBERS.clone()),
                    ),
                });
            }

            if <TypeIdentifier as Into<Type>>::into(expr2.ret_type.clone()) != Type::U32 {
                return Err(ASTLinkError {
                    token: node.token,
                    err: ASTLinkErrorInner::MismatchedType(
                        expr2.ret_type.clone().into(),
                        Box::new([Type::U32]),
                    ),
                });
            }

            Ok(FASTNode {
                ret_type: expr1.ret_type.clone().into(),
                inter_type: expr1.ret_type.clone().into(),
                node: FASTContent::Pow(Box::new(expr1), Box::new(expr2)),
            })
        }
        ASTNodeInner::Modulus(astnode_expr, astnode_expr1) => ast_linking_need_x_number(
            info,
            astnode_expr,
            astnode_expr1,
            FASTContent::Modulus,
            &NUMBERS,
            variables,
        ),
        ASTNodeInner::Equal(astnode_expr, astnode_expr1) => ast_linking_need_x_compare(
            info,
            astnode_expr,
            astnode_expr1,
            FASTContent::Equal,
            &EQUALLABLETYPES,
            variables,
        ),
        ASTNodeInner::NotEqual(astnode_expr, astnode_expr1) => ast_linking_need_x_compare(
            info,
            astnode_expr,
            astnode_expr1,
            FASTContent::NotEqual,
            &EQUALLABLETYPES,
            variables,
        ),
        ASTNodeInner::Greater(astnode_expr, astnode_expr1) => ast_linking_need_x_compare(
            info,
            astnode_expr,
            astnode_expr1,
            FASTContent::Greater,
            &NUMBERS,
            variables,
        ),
        ASTNodeInner::Lesser(astnode_expr, astnode_expr1) => ast_linking_need_x_compare(
            info,
            astnode_expr,
            astnode_expr1,
            FASTContent::Lesser,
            &NUMBERS,
            variables,
        ),
        ASTNodeInner::GreaterE(astnode_expr, astnode_expr1) => ast_linking_need_x_compare(
            info,
            astnode_expr,
            astnode_expr1,
            FASTContent::GreaterE,
            &NUMBERS,
            variables,
        ),
        ASTNodeInner::LesserE(astnode_expr, astnode_expr1) => ast_linking_need_x_compare(
            info,
            astnode_expr,
            astnode_expr1,
            FASTContent::LesserE,
            &NUMBERS,
            variables,
        ),
        ASTNodeInner::And(astnode_expr, astnode_expr1) => ast_linking_need_x_compare(
            info,
            astnode_expr,
            astnode_expr1,
            FASTContent::And,
            &[Type::Boolean],
            variables,
        ),
        ASTNodeInner::Or(astnode_expr, astnode_expr1) => ast_linking_need_x_compare(
            info,
            astnode_expr,
            astnode_expr1,
            FASTContent::Or,
            &[Type::Boolean],
            variables,
        ),
        ASTNodeInner::Inner(node, str) => {
            let token = node.token.clone();
            let mut fnode = link_ast_node(*node, info, variables, &[Type::UserClass])?;
            if let FASTContent::Variable(x) = fnode.node {
                fnode = FASTNode {
                    node: FASTContent::Reference(FASTReference::Variable(x)),
                    ret_type: fnode.ret_type,
                    inter_type: fnode.inter_type,
                }
            }
            if let FASTContent::Reference(x) = fnode.node {
                if let TypeIdentifier::UserClass(class) = fnode.ret_type {
                    let cl = info.classes.get(class.0).unwrap();
                    let f = cl.field_names.get(&str);
                    if let Some(y) = f {
                        let f = &cl.fields[*y];
                        if f.access_level == FieldAccessLevel::Private {
                            return Err(ASTLinkError {
                                token,
                                err: ASTLinkErrorInner::AccessDenied(
                                    cl.name.clone(),
                                    f.name.clone(),
                                ),
                            });
                        }
                        if f.access_level == FieldAccessLevel::Protected
                            && info.cur_class.map_or(false, |x| *x != f.class)
                        {
                            return Err(ASTLinkError {
                                token,
                                err: ASTLinkErrorInner::AccessDenied(
                                    cl.name.clone(),
                                    f.name.clone(),
                                ),
                            });
                        }
                        let f_t = &f.t;
                        Ok(FASTNode {
                            node: FASTContent::Reference(FASTReference::Inner(Box::new(x), *y)),
                            ret_type: f_t.clone(),
                            inter_type: f_t.clone(),
                        })
                    } else {
                        Err(ASTLinkError {
                            token,
                            err: ASTLinkErrorInner::NonexistentField(
                                info.classes[class.0].name.clone(),
                                str,
                            ),
                        })
                    }
                } else {
                    Err(ASTLinkError {
                        token,
                        err: ASTLinkErrorInner::MismatchedType(
                            fnode.ret_type.into(),
                            Box::new([Type::UserClass]),
                        ),
                    })
                }
            } else {
                Err(ASTLinkError {
                    token,
                    err: ASTLinkErrorInner::MismatchedType(
                        fnode.ret_type.into(),
                        Box::new([Type::UserClass]),
                    ),
                })
            }
        }
        ASTNodeInner::Brace(mut astblock) => {
            let token = node.token.clone();
            let mut n_vars = variables.to_vec();
            let block_vars = astblock
                .variables
                .iter()
                .map(|x| {
                    let ts = match &x.t {
                        TypeIdentifier::UnlinkedType(ts) => ts,
                        _ => {
                            unreachable!(
                                "Variables are supposed to have unlinked type during linking."
                            )
                        }
                    };
                    let t = find_type(ts, &info.classes_map);

                    match t {
                        Some(t) => Ok(ASTVariable {
                            t,
                            name: x.name.clone(),
                        }),
                        None => Err(ASTLinkError {
                            token: node.token.clone(),
                            err: ASTLinkErrorInner::NonexistentVT(ts.clone()),
                        }),
                    }
                })
                .collect::<Vec<_>>();

            let mut block_ok = vec![];
            let mut block_err = vec![];

            for v in block_vars.into_iter() {
                match v {
                    Ok(x) => block_ok.push(x),
                    Err(x) => block_err.push(x),
                }
            }

            if block_err.len() > 0 {
                return Err(ASTLinkError {
                    token,
                    err: ASTLinkErrorInner::SubError(block_err),
                });
            }

            let block_okref = &block_ok.iter().collect::<Vec<_>>().into_boxed_slice();

            n_vars.push(block_okref);

            let exprlen = astblock.exprs.len();
            let mut exprs = Vec::with_capacity(exprlen);
            let last = astblock.exprs.pop().ok_or(ASTLinkError {
                token: node.token,
                err: ASTLinkErrorInner::EmptyBrace,
            })?;
            let expiter = astblock.exprs.into_iter();
            for x in expiter {
                let n_node = link_ast_node(x, info, &n_vars, &TYPES);
                exprs.push(n_node);
            }

            let n_node = link_ast_node(last, info, &n_vars, exp_type);
            exprs.push(n_node);

            let mut errors = vec![];
            let mut n_exprs = vec![];
            for x in exprs {
                match x {
                    Err(x) => {
                        errors.push(x);
                    }
                    Ok(x) => {
                        n_exprs.push(x);
                    }
                }
            }

            if errors.len() > 0 {
                return Err(ASTLinkError {
                    token,
                    err: ASTLinkErrorInner::SubError(errors),
                });
            }

            let t = n_exprs.last().map(|x| x.ret_type.clone());
            match t {
                None => Err(ASTLinkError {
                    token,
                    err: ASTLinkErrorInner::EmptyBrace,
                }),
                Some(x) => Ok(FASTNode {
                    node: FASTContent::Brace(FASTBlock {
                        variables: block_ok,
                        exprs: n_exprs.into_iter().collect(),
                    }),
                    inter_type: x.clone(),
                    ret_type: x,
                }),
            }
        }
        ASTNodeInner::If(astnode_expr, astnode_expr1, astnode_expr2) => {
            let expr = link_ast_node(*astnode_expr, info, variables, &[Type::Boolean])?;
            let t = link_ast_node(
                *astnode_expr1,
                info,
                variables,
                if astnode_expr2.is_none() {
                    &TYPES
                } else {
                    exp_type
                },
            )?;
            let f = astnode_expr2.map(|x| link_ast_node(*x, info, variables, exp_type));

            if !TypeIdentifier::Boolean.is_assignable_from(&expr.ret_type) {
                return Err(ASTLinkError {
                    token: node.token,
                    err: ASTLinkErrorInner::MismatchedType(
                        expr.ret_type.into(),
                        Box::new([Type::Boolean]),
                    ),
                });
            }

            match f {
                None => Ok(FASTNode {
                    node: FASTContent::If(Box::new(expr), Box::new(t), None),
                    inter_type: TypeIdentifier::None,
                    ret_type: TypeIdentifier::None,
                }),
                Some(x) => {
                    let f = x?;

                    if t.ret_type.is_assignable_from(&f.ret_type) {
                        Ok(FASTNode {
                            inter_type: f.ret_type.clone(),
                            ret_type: f.ret_type.clone(),
                            node: FASTContent::If(Box::new(expr), Box::new(t), Some(Box::new(f))),
                        })
                    } else {
                        Err(ASTLinkError {
                            token: node.token,
                            err: ASTLinkErrorInner::UncovertibleTypes(
                                t.ret_type.into(),
                                f.ret_type.into(),
                            ),
                        })
                    }
                }
            }
        }
        ASTNodeInner::Error => Err(ASTLinkError {
            token: node.token,
            err: ASTLinkErrorInner::ErrorToken,
        }),
    }?;

    if exp_type
        .iter()
        .map(|x| x.is_assignable_from(&ret.ret_type.clone().into()))
        .reduce(|a, b| a || b)
        .unwrap()
    {
        Ok(ret)
    } else {
        Err(ASTLinkError {
            token,
            err: ASTLinkErrorInner::MismatchedType(ret.ret_type.into(), exp_type.into()),
        })
    }
}

// #[cfg(test)]
// mod tests {
//     use ordered_float::NotNan;
//
//     use super::*;
//
//     fn test_link_field() {
//         let mut user_classes = HashMap::new();
//
//         user_classes.insert("oink".to_string(), UserClassIndx { 0: 0 });
//     }
//
//     #[test]
//     fn test_find_type() {
//         let mut user_classes = HashMap::new();
//         user_classes.insert("oink".to_owned(), UserClassIndx { 0: 0 });
//         user_classes.insert("oink".to_owned(), UserClassIndx { 0: 0 });
//         assert_eq!(
//             find_type("i64", &user_classes).unwrap(),
//             TypeIdentifier::I64
//         );
//         assert_eq!(
//             find_type("i32", &user_classes).unwrap(),
//             TypeIdentifier::I32
//         );
//         assert_eq!(
//             find_type("i16", &user_classes).unwrap(),
//             TypeIdentifier::I16
//         );
//         assert_eq!(find_type("i8", &user_classes).unwrap(), TypeIdentifier::I8);
//         assert_eq!(
//             find_type("u64", &user_classes).unwrap(),
//             TypeIdentifier::U64
//         );
//         assert_eq!(
//             find_type("u32", &user_classes).unwrap(),
//             TypeIdentifier::U32
//         );
//         assert_eq!(
//             find_type("u16", &user_classes).unwrap(),
//             TypeIdentifier::U16
//         );
//         assert_eq!(find_type("u8", &user_classes).unwrap(), TypeIdentifier::U8);
//         assert_eq!(
//             find_type("float", &user_classes).unwrap(),
//             TypeIdentifier::Float
//         );
//         assert_eq!(
//             find_type("double", &user_classes).unwrap(),
//             TypeIdentifier::Double
//         );
//         assert_eq!(
//             find_type("string", &user_classes).unwrap(),
//             TypeIdentifier::String
//         );
//         assert_eq!(
//             find_type("bool", &user_classes).unwrap(),
//             TypeIdentifier::Boolean
//         );
//         assert_eq!(
//             find_type("oink", &user_classes).unwrap(),
//             TypeIdentifier::UserClass(UserClassIndx(0))
//         );
//         assert_eq!(
//             find_type("oink2", &user_classes).unwrap(),
//             TypeIdentifier::UserClass(UserClassIndx(1))
//         );
//         assert_eq!(find_type("NonexistentType", &user_classes), None);
//     }
//
//     fn setup_classes() -> (HashMap<String, UserClassIndx>, Vec<UserClass>) {
//         (HashMap::new(), vec![])
//     }
//
//     fn setup_variables<'a, 'b, 'c>() -> &'a [&'b [&'c ASTVariable]] {
//         &[]
//     }
//
//     #[test]
//     fn test_link_ast_node_integer() {
//         let (classes_map, classes) = setup_classes();
//         let variables = setup_variables();
//         let node = ASTNode::Integer(42);
//         let result = link_ast_node(node, &classes_map, &classes, &variables, &[Type::I64]);
//         assert!(result.is_ok());
//         let FASTNode {
//             node: linked_node,
//             inter_type: ty,
//             ret_type: _ty,
//         } = result.unwrap();
//         assert!(matches!(linked_node, FASTContent::Integer(42)));
//         assert_eq!(ty, TypeIdentifier::I64);
//     }
//
//     #[test]
//     fn test_link_ast_node_decimal() {
//         let (classes_map, classes) = setup_classes();
//         let variables = setup_variables();
//         let node = ASTNode::Decimal(NotNan::new(3.15).unwrap());
//         let result = link_ast_node(node, &classes_map, &classes, &variables, &[Type::Double]);
//         assert!(result.is_ok());
//         let FASTNode {
//             node: linked_node,
//             inter_type: ty,
//             ret_type: _ty,
//         } = result.unwrap();
//         assert!(matches!(linked_node, FASTContent::Decimal(_)));
//         assert_eq!(ty, TypeIdentifier::Double);
//     }
//
//     #[test]
//     fn test_link_ast_node_string() {
//         let (classes_map, classes) = setup_classes();
//         let variables = setup_variables();
//         let node = ASTNode::String(String::from("hello"));
//         let result = link_ast_node(node, &classes_map, &classes, &variables, &[Type::String]);
//         assert!(result.is_ok());
//         let FASTNode {
//             node: linked_node,
//             inter_type: ty,
//             ret_type: _ty,
//         } = result.unwrap();
//         assert!(matches!(linked_node, FASTContent::String(_)));
//         assert_eq!(ty, TypeIdentifier::String);
//     }
//
//     #[test]
//     fn test_link_ast_node_negate() {
//         let (classes_map, classes) = setup_classes();
//         let variables = setup_variables();
//         let node = ASTNode::Negate(Box::new(ASTNode::Integer(42)));
//         let result = link_ast_node(node, &classes_map, &classes, &variables, &[Type::I64]);
//         assert!(result.is_ok());
//         let FASTNode {
//             node: linked_node,
//             inter_type: ty,
//             ret_type: _ty,
//         } = result.unwrap();
//         assert!(matches!(linked_node, FASTContent::Negate(_)));
//         assert_eq!(ty, TypeIdentifier::I64);
//     }
//
//     #[test]
//     fn test_link_ast_node_not_invalid_type() {
//         let (classes_map, classes) = setup_classes();
//         let variables = setup_variables();
//         let node = ASTNode::Not(Box::new(ASTNode::Integer(0)));
//         let result = link_ast_node(node, &classes_map, &classes, &variables, &[Type::Boolean]);
//         assert!(matches!(result, Err(ASTLinkError::MismatchedType(_, _))));
//     }
//
//     #[test]
//     fn test_link_ast_node_variable() {
//         let (classes_map, classes) = setup_classes();
//         let variables: &[&[&ASTVariable]] = &[&[&ASTVariable {
//             t: TypeIdentifier::I64,
//             name: "a".to_owned(),
//         }]];
//         let node = ASTNode::Variable(ASTVarRef {
//             indx_up: 0,
//             indx: 0,
//         });
//         let result = link_ast_node(node, &classes_map, &classes, variables, &[Type::I64]);
//         assert!(result.is_ok());
//         let FASTNode {
//             node: linked_node,
//             inter_type: ty,
//             ret_type: _ty,
//         } = result.unwrap();
//         assert!(matches!(linked_node, FASTContent::Variable(_)));
//         assert_eq!(ty, TypeIdentifier::I64);
//     }
//
//     #[test]
//     fn test_link_ast_node_type_nonexistent() {
//         let (classes_map, classes) = setup_classes();
//         let variables = setup_variables();
//         let node = ASTNode::Type(TypeIdentifier::UnlinkedType(String::from("unknown")));
//         let result = link_ast_node(node, &classes_map, &classes, &variables, &[Type::UserClass]);
//         assert!(matches!(result, Err(ASTLinkError::NonexistentVT(_))));
//     }
//
//     #[test]
//     fn test_link_ast_node_set_mismatch() {
//         let (classes_map, classes) = setup_classes();
//         let variables: &[&[&ASTVariable]] = &[&[&ASTVariable {
//             t: TypeIdentifier::I64,
//             name: "a".to_owned(),
//         }]];
//         let node = ASTNode::Set(
//             ASTVarRef {
//                 indx_up: 0,
//                 indx: 0,
//             },
//             Box::new(ASTNode::Decimal(NotNan::new(3.15).unwrap())),
//         );
//         let result = link_ast_node(node, &classes_map, &classes, variables, &[Type::I64]);
//         assert!(matches!(result, Err(ASTLinkError::UncovertibleTypes(_, _))));
//     }
//
//     #[test]
//     fn test_link_ast_node_add() {
//         let (classes_map, classes) = setup_classes();
//         let variables = setup_variables();
//         let node = ASTNode::Add(
//             Box::new(ASTNode::Integer(42)),
//             Box::new(ASTNode::Integer(21)),
//         );
//         let result = link_ast_node(node, &classes_map, &classes, &variables, &[Type::I64]);
//         assert!(result.is_ok());
//         let FASTNode {
//             node: linked_node,
//             inter_type: ty,
//             ret_type: _ty,
//         } = result.unwrap();
//         assert!(matches!(linked_node, FASTContent::Add(_, _)));
//         assert_eq!(ty, TypeIdentifier::I64);
//     }
//
//     #[test]
//     fn test_link_ast_node_subtract() {
//         let (classes_map, classes) = setup_classes();
//         let variables = setup_variables();
//         let node = ASTNode::Subtract(
//             Box::new(ASTNode::Integer(42)),
//             Box::new(ASTNode::Integer(21)),
//         );
//         let result = link_ast_node(node, &classes_map, &classes, &variables, &[Type::I64]);
//         assert!(result.is_ok());
//         let FASTNode {
//             node: linked_node,
//             inter_type: ty,
//             ret_type: _ty,
//         } = result.unwrap();
//         assert!(matches!(linked_node, FASTContent::Subtract(_, _)));
//         assert_eq!(ty, TypeIdentifier::I64);
//     }
//
//     #[test]
//     fn test_link_ast_node_empty_brace() {
//         let (classes_map, classes) = setup_classes();
//         let variables = setup_variables();
//         let node = ASTNode::Brace(ASTBlock {
//             variables: vec![],
//             exprs: vec![],
//         });
//         let result = link_ast_node(node, &classes_map, &classes, &variables, &TYPES);
//         assert!(matches!(result, Err(ASTLinkError::EmptyBrace)));
//     }
//
//     #[test]
//     fn test_link_ast_node_error() {
//         let (classes_map, classes) = setup_classes();
//         let variables = setup_variables();
//         let node = ASTNode::Error;
//         let result = link_ast_node(node, &classes_map, &classes, &variables, &TYPES);
//         assert!(matches!(result, Err(ASTLinkError::ErrorToken)));
//     }
//
//     #[test]
//     fn test_ast_linking_need_x_unary_valid() {
//         let (classes_map, classes) = setup_classes();
//         let variables = setup_variables();
//         let astnode_expr = Box::new(ASTNode::Integer(42));
//         let result = ast_linking_need_x_unary(
//             &classes_map,
//             &classes,
//             astnode_expr,
//             FASTContent::Negate,
//             &NUMBERS,
//             &variables,
//         );
//         assert!(result.is_ok());
//         let FASTNode {
//             node: linked_node,
//             inter_type: ty,
//             ret_type: _ty,
//         } = result.unwrap();
//         assert!(matches!(linked_node, FASTContent::Negate(_)));
//         assert_eq!(ty, TypeIdentifier::I64);
//     }
//
//     #[test]
//     fn test_ast_linking_need_x_unary_invalid_type() {
//         let (classes_map, classes) = setup_classes();
//         let variables = setup_variables();
//         let astnode_expr = Box::new(ASTNode::String(String::from("hello")));
//         let result = ast_linking_need_x_unary(
//             &classes_map,
//             &classes,
//             astnode_expr,
//             FASTContent::Negate,
//             &NUMBERS,
//             &variables,
//         );
//         assert!(matches!(result, Err(ASTLinkError::MismatchedType(_, _))));
//     }
//
//     #[test]
//     fn test_ast_linking_need_x_binary_valid() {
//         let (classes_map, classes) = setup_classes();
//         let variables = setup_variables();
//         let astnode_expr1 = Box::new(ASTNode::Integer(42));
//         let astnode_expr2 = Box::new(ASTNode::Integer(21));
//         let result = ast_linking_need_x_number(
//             &classes_map,
//             &classes,
//             astnode_expr1,
//             astnode_expr2,
//             FASTContent::Add,
//             &NUMBERS,
//             &variables,
//         );
//         assert!(result.is_ok());
//         let FASTNode {
//             node: linked_node,
//             inter_type: ty,
//             ret_type: _ty,
//         } = result.unwrap();
//         assert!(matches!(linked_node, FASTContent::Add(_, _)));
//         assert_eq!(ty, TypeIdentifier::I64);
//     }
//
//     #[test]
//     fn test_ast_linking_need_x_binary_mismatched_type() {
//         let (classes_map, classes) = setup_classes();
//         let variables = setup_variables();
//         let astnode_expr1 = Box::new(ASTNode::String(String::from("hello")));
//         let astnode_expr2 = Box::new(ASTNode::String(String::from("hello")));
//         let result = ast_linking_need_x_number(
//             &classes_map,
//             &classes,
//             astnode_expr1,
//             astnode_expr2,
//             FASTContent::Add,
//             &NUMBERS,
//             &variables,
//         );
//         assert!(matches!(result, Err(ASTLinkError::MismatchedType(_, _))));
//     }
//
//     #[test]
//     fn test_ast_linking_need_x_binary_uncovertible_types() {
//         let (classes_map, classes) = setup_classes();
//         let variables = setup_variables();
//         let astnode_expr1 = Box::new(ASTNode::Integer(42));
//         let astnode_expr2 = Box::new(ASTNode::String(String::from("hello")));
//         let result = ast_linking_need_x_number(
//             &classes_map,
//             &classes,
//             astnode_expr1,
//             astnode_expr2,
//             FASTContent::Add,
//             &NUMBERS,
//             &variables,
//         );
//         assert!(matches!(result, Err(ASTLinkError::UncovertibleTypes(_, _))));
//     }
// }
