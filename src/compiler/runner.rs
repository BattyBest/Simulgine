use core::fmt;
use std::{
    io::Read,
    ops::Index,
    path::{Path, PathBuf},
    unreachable,
};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::compiler::{ast::*, simulgine_inst::spawn_type_instance_const};

use super::{
    ast::TypeReference,
    astbuilder::{build_ast, build_free_expr},
    linker::{link_ast, link_ast_node, ASTLinkError, LinkerEnvInfo, LinkerInfo},
    scanner::FileScanner,
    simulgine_inst::{
        spawn_type_instance, Simulgine, SimulgineInst, UserClass, UserClassMember, UserObject,
    },
};

#[derive(Debug)]
pub enum SimulgineErrors {
    NoSmlFile,
    ASTErrors,
    LinkError(Vec<ASTLinkError>),
    IOError(std::io::Error),
}

impl fmt::Display for SimulgineErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimulgineErrors::NoSmlFile => {
                f.write_str("No .sml files in the given directory could be found.")
            }
            SimulgineErrors::ASTErrors => {
                f.write_str("The code could not be parsed due to grammatical errors.")
            }
            SimulgineErrors::LinkError(e) => {
                e.iter().for_each(|x| {
                    let _ = x.fmt(f);
                });
                // if the formatting fails we uhhhh
                Ok(())
            }
            SimulgineErrors::IOError(error) => error.fmt(f),
        }
    }
}

impl From<std::io::Error> for SimulgineErrors {
    fn from(value: std::io::Error) -> Self {
        SimulgineErrors::IOError(value)
    }
}

impl From<ASTLinkError> for SimulgineErrors {
    fn from(value: ASTLinkError) -> Self {
        SimulgineErrors::LinkError(vec![value])
    }
}

pub struct FASTExecContext<'a> {
    vars: Vec<Vec<TypeReference<'a>>>,
}

pub fn tick_field<'a, 'c, 'd, 'e, 'g>(
    prev: &'a TypeInstance,
    parent: &'g UserObject,
    field: &'c UserClassMember,
    sim: &'d SimulgineInst<'e>,
) -> TypeInstance {
    let mut context = FASTExecContext { vars: vec![] };
    context.vars.push(vec![
        TypeReference::Instance(prev.clone()),
        TypeReference::UserClassRO(UserClassROInst { val: parent }),
    ]);
    assert_eq!(field.change_level, FieldChangeLevel::Volatile);

    match execute_fast_node(field.body.as_ref().unwrap(), &mut context, Some(sim)) {
        TypeReference::Instance(type_instance) => type_instance,
        TypeReference::UserClassRO(_user_class_roinst) => {
            unreachable!("Field body returned reference")
        }
    }
}

pub fn calc_field(
    i: usize,
    f: &TypeInstance,
    t: &UserObject,
    class: &UserClass,
    sim: &SimulgineInst,
) -> TypeInstance {
    if class.fields[i].change_level == FieldChangeLevel::Const {
        f.clone()
    } else if class.fields[i].change_level == FieldChangeLevel::Level {
        if let TypeInstance::UserClass(x) = &t.fields[i] {
            let fs = tick_obj(&x.val, sim);
            TypeInstance::UserClass(UserClassInst { val: fs })
        } else {
            f.clone()
        }
    } else {
        tick_field(f, t, &class.fields[i], sim)
    }
}

pub fn tick_obj(t: &UserObject, sim: &SimulgineInst) -> UserObject {
    let class = sim.based.get_user_class(t.class).unwrap();

    let mut obj = t.clone();

    for j in 0..class.field_stages.len() {
        class.field_stages[j]
            .par_iter()
            .clone()
            .map(|x| calc_field(*x, &obj.fields[*x], &obj, class, sim))
            .collect::<Vec<_>>()
            .into_iter()
            .enumerate()
            .for_each(|(i, v)| {
                obj.fields[class.field_stages[j][i]] = v;
            });
    }

    obj
}

pub fn tick_sim(sim: &mut SimulgineInst) {
    let fs = tick_obj(&sim.root, sim);
    sim.root = fs;
}

fn get_var_val<'a, 'b>(context: &'a FASTExecContext<'b>, var: ASTVarRef) -> &'a TypeReference<'b> {
    context
        .vars
        .index(context.vars.len() - var.indx_up as usize - 1)
        .index(var.indx as usize)
}

fn set_var_val<'a>(
    context: &mut FASTExecContext<'a>,
    var: ASTVarRef,
    val: TypeReference<'a>,
) -> TypeReference<'a> {
    let l = context.vars.len();
    context.vars[l - var.indx_up as usize - 1][var.indx as usize] = val.clone();
    val
}

macro_rules! number_worker {
    ($par:expr, $node:expr, $l:expr, $r:expr, $context:expr, $sim:expr) => {{
        let l = execute_fast_node($l, $context, $sim);
        let r = execute_fast_node($r, $context, $sim);

        match $node.ret_type {
            TypeIdentifier::I64 => {
                let l = l.coerce_number_int().unwrap();
                let r = r.coerce_number_int().unwrap();

                TypeInstance::I64(I64Inst {
                    val: $par(l, r) as i64,
                })
            }
            TypeIdentifier::I32 => {
                let l = l.coerce_number_int().unwrap();
                let r = r.coerce_number_int().unwrap();

                TypeInstance::I32(I32Inst {
                    val: $par(l, r) as i32,
                })
            }
            TypeIdentifier::I16 => {
                let l = l.coerce_number_int().unwrap();
                let r = r.coerce_number_int().unwrap();

                TypeInstance::I16(I16Inst {
                    val: $par(l, r) as i16,
                })
            }
            TypeIdentifier::I8 => {
                let l = l.coerce_number_int().unwrap();
                let r = r.coerce_number_int().unwrap();

                TypeInstance::I8(I8Inst {
                    val: $par(l, r) as i8,
                })
            }
            TypeIdentifier::U64 => {
                let l = l.coerce_number_int().unwrap();
                let r = r.coerce_number_int().unwrap();

                TypeInstance::U64(U64Inst {
                    val: $par(l, r) as u64,
                })
            }
            TypeIdentifier::U32 => {
                let l = l.coerce_number_int().unwrap();
                let r = r.coerce_number_int().unwrap();

                TypeInstance::U32(U32Inst {
                    val: $par(l, r) as u32,
                })
            }
            TypeIdentifier::U16 => {
                let l = l.coerce_number_int().unwrap();
                let r = r.coerce_number_int().unwrap();

                TypeInstance::U16(U16Inst {
                    val: $par(l, r) as u16,
                })
            }
            TypeIdentifier::U8 => {
                let l = l.coerce_number_int().unwrap();
                let r = r.coerce_number_int().unwrap();

                TypeInstance::U8(U8Inst {
                    val: $par(l, r) as u8,
                })
            }
            TypeIdentifier::Float => {
                let l = l.coerce_number_float().unwrap();
                let r = r.coerce_number_float().unwrap();

                TypeInstance::Float(FloatInst {
                    val: $par(l, r) as f32,
                })
            }
            TypeIdentifier::Double => {
                let l = l.coerce_number_float().unwrap();
                let r = r.coerce_number_float().unwrap();

                TypeInstance::Double(DoubleInst {
                    val: $par(l, r) as f64,
                })
            }
            _ => unreachable!("You can only conduct number operations on numbers."),
        }
    }};
}

macro_rules! compare_worker {
    ($par:expr, $node:expr, $l:expr, $r:expr, $context:expr, $sim:expr) => {{
        let l = execute_fast_node($l, $context, $sim);
        let r = execute_fast_node($r, $context, $sim);

        if INTEGER_NUMBERS.contains(&$node.inter_type.clone().into()) {
            TypeInstance::Boolean(BooleanInst {
                val: $par(l.coerce_number_int(), r.coerce_number_int()),
            })
        } else if FLOATING_NUMBERS.contains(&$node.inter_type.clone().into()) {
            TypeInstance::Boolean(BooleanInst {
                val: $par(l.coerce_number_float(), r.coerce_number_float()),
            })
        } else {
            unreachable!("Tried to do number comparisons on not numbers");
        }
    }};
}

pub(crate) fn resolve_inner_reference<'a, 'b, 'c, 'd, 'e>(
    node: &'a FASTReference,
    context: &'b mut FASTExecContext<'c>,
    sim: Option<&'d SimulgineInst<'e>>,
) -> TypeReference<'d>
where
    'c: 'd,
{
    match node {
        FASTReference::Root => {
            if let Some(sim) = sim {
                TypeReference::UserClassRO(UserClassROInst { val: &sim.root })
            } else {
                TypeReference::Instance(TypeInstance::None)
            }
        }
        FASTReference::Inner(fastreference, s) => {
            let res = resolve_inner_reference(fastreference, context, sim);
            match res {
                TypeReference::Instance(_type_instance) => unreachable!("Inner ('.') is somehow trying to access a field on something not an UserObject."),
                TypeReference::UserClassRO(user_class_roinst) => {
                    let res = user_class_roinst.val.fields.index(*s);
                    match res {
                        TypeInstance::UserClass(user_class_inst) => {
                            TypeReference::UserClassRO(UserClassROInst {
                                val: &user_class_inst.val,
                            })
                        }
                        _ => TypeReference::Instance(res.clone()),
                    }
                }
            }
        }
        FASTReference::Variable(x) => {
            let val = get_var_val(context, x.clone());
            val.clone()
        }
    }
}

pub(crate) fn execute_fast_node<'a, 'b, 'c, 'f>(
    node: &'a FASTNode,
    context: &'b mut FASTExecContext<'c>,
    sim: Option<&'c SimulgineInst<'f>>, // None == const context
) -> TypeReference<'c> {
    match &node.node {
        FASTContent::Variable(astvar_ref) => get_var_val(&*context, astvar_ref.clone()).clone(),
        FASTContent::Set(astvar_ref, fastnode) => {
            let a = execute_fast_node(fastnode, context, sim);
            set_var_val(&mut *context, astvar_ref.clone(), a)
        }
        FASTContent::Brace(fastblock) => {
            let mut last = TypeReference::Instance(TypeInstance::None);
            let vars = fastblock
                .variables
                .iter()
                .map(|x| {
                    if let Some(sim) = sim {
                        TypeReference::Instance(spawn_type_instance(sim.based, &x.t))
                    } else {
                        TypeReference::Instance(spawn_type_instance_const(&x.t))
                    }
                })
                .collect();

            context.vars.push(vars);

            for a in &fastblock.exprs {
                last = execute_fast_node(a, context, sim);
            }

            context.vars.pop();

            last
        }
        FASTContent::If(fastnode, fastnode1, fastnode2) => {
            let d = execute_fast_node(fastnode, context, sim)
                .coerce_boolean()
                .unwrap();

            if d {
                execute_fast_node(fastnode1, context, sim)
            } else if let Some(x) = fastnode2 {
                execute_fast_node(x, context, sim)
            } else {
                TypeReference::Instance(TypeInstance::None)
            }
        }
        FASTContent::Reference(node) => resolve_inner_reference(node, context, sim),
        _ => TypeReference::Instance(match &node.node {
            FASTContent::Integer(x) => match node.ret_type {
                TypeIdentifier::I64 => TypeInstance::I64(I64Inst { val: *x }),
                TypeIdentifier::I32 => TypeInstance::I32(I32Inst { val: *x as i32 }),
                TypeIdentifier::I16 => TypeInstance::I16(I16Inst { val: *x as i16 }),
                TypeIdentifier::I8 => TypeInstance::I8(I8Inst { val: *x as i8 }),
                TypeIdentifier::U64 => TypeInstance::U64(U64Inst { val: *x as u64 }),
                TypeIdentifier::U32 => TypeInstance::U32(U32Inst { val: *x as u32 }),
                TypeIdentifier::U16 => TypeInstance::U16(U16Inst { val: *x as u16 }),
                TypeIdentifier::U8 => TypeInstance::U8(U8Inst { val: *x as u8 }),
                _ => unreachable!("Integer literal is supposed to return an integer number."),
            },
            FASTContent::Decimal(not_nan) => match node.ret_type {
                TypeIdentifier::Float => TypeInstance::Float(FloatInst {
                    val: *not_nan.as_f32(),
                }),
                TypeIdentifier::Double => TypeInstance::Double(DoubleInst { val: **not_nan }),
                _ => unreachable!("Decimal literal is supposed to return a floating point number."),
            },
            FASTContent::String(x) => TypeInstance::String(StringInst { val: x.clone() }),
            FASTContent::True => TypeInstance::Boolean(BooleanInst { val: true }),
            FASTContent::False => TypeInstance::Boolean(BooleanInst { val: false }),
            FASTContent::Type(type_identifier) => TypeInstance::Type(TypeInst {
                val: type_identifier.clone(),
            }),
            FASTContent::Negate(fastnode) => {
                let res = execute_fast_node(fastnode, context, sim);
                let TypeReference::Instance(res) = res else {
                    unreachable!("Negate can't negate references.")
                };
                let res = match res {
                    TypeInstance::I64(i64_inst) => {
                        TypeInstance::I64(I64Inst { val: -i64_inst.val })
                    }
                    TypeInstance::I32(i32_inst) => {
                        TypeInstance::I32(I32Inst { val: -i32_inst.val })
                    }
                    TypeInstance::I16(i16_inst) => {
                        TypeInstance::I16(I16Inst { val: -i16_inst.val })
                    }
                    TypeInstance::I8(i8_inst) => TypeInstance::I8(I8Inst { val: -i8_inst.val }),
                    TypeInstance::Float(float_inst) => TypeInstance::Float(FloatInst {
                        val: -float_inst.val,
                    }),
                    TypeInstance::Double(double_inst) => TypeInstance::Double(DoubleInst {
                        val: -double_inst.val,
                    }),
                    _ => unreachable!("Can only negate signed numbers."),
                };

                res
            }
            FASTContent::Not(fastnode) => {
                let res = execute_fast_node(fastnode, context, sim);
                let TypeReference::Instance(res) = res else {
                    unreachable!("Not can't negate references.")
                };

                let res = match res {
                    TypeInstance::Boolean(boolean_inst) => TypeInstance::Boolean(BooleanInst {
                        val: !boolean_inst.val,
                    }),
                    _ => unreachable!("Can only not booleans."),
                };

                res
            }
            FASTContent::Typeof(fastnode) => {
                let res = execute_fast_node(fastnode, context, sim);

                TypeInstance::Type(TypeInst {
                    val: TypeReference::as_type_identifier(&res),
                })
            }
            FASTContent::Instanceof(fastnode) => {
                let res = execute_fast_node(fastnode, context, sim);
                let TypeReference::Instance(res) = res else {
                    unreachable!("Not can't instantiate referenced classes.")
                };

                if let TypeInstance::Type(x) = res {
                    if let Some(sim) = sim {
                        spawn_type_instance(sim.based, &x.val)
                    } else {
                        spawn_type_instance_const(&x.val)
                    }
                } else {
                    unreachable!("Can only instantiate types.")
                }
            }
            FASTContent::Add(fastnode, fastnode1) => {
                number_worker!(|l, r| { l + r }, node, fastnode, fastnode1, context, sim)
            }
            FASTContent::Subtract(fastnode, fastnode1) => {
                number_worker!(|l, r| { l - r }, node, fastnode, fastnode1, context, sim)
            }
            FASTContent::Multiply(fastnode, fastnode1) => {
                number_worker!(|l, r| { l * r }, node, fastnode, fastnode1, context, sim)
            }
            FASTContent::Divide(fastnode, fastnode1) => {
                number_worker!(|l, r| { l / r }, node, fastnode, fastnode1, context, sim)
            }
            FASTContent::Pow(fastnode, fastnode1) => {
                let res = execute_fast_node(fastnode, context, sim);
                let TypeReference::Instance(res) = res else {
                    unreachable!("Not can't pow references.")
                };
                let pow = execute_fast_node(fastnode1, context, sim)
                    .coerce_number_int()
                    .unwrap();
                let res = match res {
                    TypeInstance::I64(i64_inst) => TypeInstance::I64(I64Inst {
                        val: i64_inst.val.pow(pow.try_into().unwrap()),
                    }),
                    TypeInstance::I32(i32_inst) => TypeInstance::I32(I32Inst {
                        val: i32_inst.val.pow(pow.try_into().unwrap()),
                    }),
                    TypeInstance::I16(i16_inst) => TypeInstance::I16(I16Inst {
                        val: i16_inst.val.pow(pow.try_into().unwrap()),
                    }),
                    TypeInstance::I8(i8_inst) => TypeInstance::I8(I8Inst {
                        val: i8_inst.val.pow(pow.try_into().unwrap()),
                    }),
                    TypeInstance::U64(u64_inst) => TypeInstance::U64(U64Inst {
                        val: u64_inst.val.pow(pow.try_into().unwrap()),
                    }),
                    TypeInstance::U32(u32_inst) => TypeInstance::U32(U32Inst {
                        val: u32_inst.val.pow(pow.try_into().unwrap()),
                    }),
                    TypeInstance::U16(u16_inst) => TypeInstance::U16(U16Inst {
                        val: u16_inst.val.pow(pow.try_into().unwrap()),
                    }),
                    TypeInstance::U8(u8_inst) => TypeInstance::U8(U8Inst {
                        val: u8_inst.val.pow(pow.try_into().unwrap()),
                    }),
                    TypeInstance::Float(float_inst) => TypeInstance::Float(FloatInst {
                        val: float_inst.val.powi(pow.try_into().unwrap()),
                    }),
                    TypeInstance::Double(double_inst) => TypeInstance::Double(DoubleInst {
                        val: double_inst.val.powi(pow.try_into().unwrap()),
                    }),
                    _ => unreachable!("Can only pow unsigned numbers and floating points."),
                };

                res
            }
            FASTContent::Modulus(fastnode, fastnode1) => {
                number_worker!(|l, r| { l % r }, node, fastnode, fastnode1, context, sim)
            }
            FASTContent::Equal(fastnode, fastnode1) => {
                let res1 = execute_fast_node(fastnode, context, sim);
                let res2 = execute_fast_node(fastnode1, context, sim);

                TypeInstance::Boolean(BooleanInst { val: res1 == res2 })
            }
            FASTContent::NotEqual(fastnode, fastnode1) => {
                let res1 = execute_fast_node(fastnode, context, sim);
                let res2 = execute_fast_node(fastnode1, context, sim);

                TypeInstance::Boolean(BooleanInst { val: res1 != res2 })
            }
            FASTContent::Greater(fastnode, fastnode1) => {
                compare_worker!(|l, r| { l > r }, node, fastnode, fastnode1, context, sim)
            }
            FASTContent::Lesser(fastnode, fastnode1) => {
                compare_worker!(|l, r| { l < r }, node, fastnode, fastnode1, context, sim)
            }
            FASTContent::GreaterE(fastnode, fastnode1) => {
                compare_worker!(|l, r| { l >= r }, node, fastnode, fastnode1, context, sim)
            }
            FASTContent::LesserE(fastnode, fastnode1) => {
                compare_worker!(|l, r| { l <= r }, node, fastnode, fastnode1, context, sim)
            }
            FASTContent::And(fastnode, fastnode1) => {
                let res1 = execute_fast_node(fastnode, context, sim)
                    .coerce_boolean()
                    .unwrap();
                let res2 = execute_fast_node(fastnode1, context, sim)
                    .coerce_boolean()
                    .unwrap();

                TypeInstance::Boolean(BooleanInst { val: res1 && res2 })
            }
            FASTContent::Or(fastnode, fastnode1) => {
                let res1 = execute_fast_node(fastnode, context, sim)
                    .coerce_boolean()
                    .unwrap();
                let res2 = execute_fast_node(fastnode1, context, sim)
                    .coerce_boolean()
                    .unwrap();

                TypeInstance::Boolean(BooleanInst { val: res1 || res2 })
            }
            FASTContent::If(_, _, _)
            | FASTContent::Set(_, _)
            | FASTContent::Brace(_)
            | FASTContent::Variable(_)
            | FASTContent::Reference(_) => {
                unreachable!("Covered in top branch.")
            }
        }),
    }
}

fn parse_all_sml_in_dir(path: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut ret: Vec<PathBuf> = Vec::new();
    for a in path.read_dir()? {
        let a = a?.path();
        if a.is_dir() {
            ret.extend(parse_all_sml_in_dir(&a)?);
        }
        if a.is_file()
            && a.extension()
                .map(|x| x.to_str().unwrap_or(""))
                .unwrap_or("")
                == "sml"
        {
            ret.push(a);
        }
    }

    Ok(ret)
}

pub fn compile_directory(path: &Path) -> Result<Simulgine, SimulgineErrors> {
    let to_compile: Vec<PathBuf> = parse_all_sml_in_dir(path)?;

    if to_compile.is_empty() {
        return Err(SimulgineErrors::NoSmlFile);
    };

    let mut raw_classes: Vec<ASTNodeDefinitionClass> = Vec::new();
    for a in to_compile {
        let mut buf: Vec<u8> = Vec::new();
        std::fs::File::open(a)?.read_to_end(&mut buf)?;

        let content = String::from_utf8_lossy(buf.as_slice()).into_owned();

        let tokens = super::scanner::FileScanner::synthesize_filescanner_str(&content);

        let ast = build_ast(tokens).ok_or(SimulgineErrors::ASTErrors)?;

        raw_classes.extend(ast.root);
    }

    link_ast(AST { root: raw_classes })
}

pub fn run_free_expression<'a>(
    sim: &'a SimulgineInst<'_>,
    expr: &str,
) -> Result<TypeReference<'a>, SimulgineErrors> {
    let fs = FileScanner::synthesize_filescanner_str(expr);
    let parse = build_free_expr(fs).ok_or(SimulgineErrors::ASTErrors)?;

    let variables: &[&[&ASTVariable]] = &[];

    let fast = link_ast_node(
        parse,
        LinkerInfo {
            env: Some(LinkerEnvInfo {
                classes_map: &sim.based.user_class_names,
                cur_class: None,
                classes: &sim.based.user_classes,
            }),
        },
        variables,
        &TYPES,
    )?;

    Ok(execute_fast_node(
        &fast,
        &mut FASTExecContext { vars: vec![] },
        Some(sim),
    ))
}

pub fn run_free_const_expression<'b>(expr: &str) -> Result<TypeReference<'b>, SimulgineErrors> {
    let fs = FileScanner::synthesize_filescanner_str(expr);
    let parse = build_free_expr(fs).ok_or(SimulgineErrors::ASTErrors)?;

    let variables: &[&[&ASTVariable]] = &[];

    let fast = link_ast_node(parse, LinkerInfo { env: None }, variables, &TYPES)?;

    Ok(execute_fast_node(
        &fast,
        &mut FASTExecContext { vars: vec![] },
        None,
    ))
}

pub fn run_simulgine(sim: &Simulgine) -> SimulgineInst<'_> {
    let root = UserObject::spawn_user_object(sim, *sim.user_class_names.get("ROOT").unwrap());

    SimulgineInst {
        based: sim,
        root: root.unwrap(),
    }
}
