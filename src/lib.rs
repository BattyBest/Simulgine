pub mod compiler;

use compiler::{
    ast::ASTVariable,
    astbuilder::build_free_expr,
    linker::{link_ast_node, LinkerEnvInfo, LinkerInfo},
    r#type::{TypeReference, TYPES},
    runner::{execute_fast_node, FASTExecContext, SimulgineErrors},
    scanner::FileScanner,
    simulgine_inst::{Simulgine, SimulgineInst, UserObject},
};

/// Run an expression inside a Simulgine instance.
///
/// This function takes in a string and compiles it, then runs the resulting expression, and returns
/// the resulting TypeReference. It will return a SimulgineErrors upon any error in the code in the
/// string provided.
///
/// # Examples
///
/// ```no_run
/// # use simulgine::run_free_expression;
/// # use simulgine::compiler::runner::*;
/// # use simulgine::run_simulgine;
/// # use simulgine::compiler::runner::SimulgineErrors;
/// use std::path::Path;
///
/// # fn try_main() -> Result<(), SimulgineErrors> {
/// let sim = compile_directory(Path::new("project"))?;
/// let sim = run_simulgine(&sim);
/// run_free_expression(&sim, "root.fieldInRoot");
/// # Ok(())
/// # }
/// # fn main() {
/// # try_main().unwrap()
/// # }
/// ```
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

/// Run an expression in a const-context.
///
/// A const-context is where an expression is evaluated without any Simulgine instance. You cannot
/// access any classes that would normally be declared in Simulgine, but everything else works.
///
/// # Examples
///
/// ```
/// # use simulgine::run_free_const_expression;
/// # use simulgine::compiler::runner::*;
/// # use simulgine::run_simulgine;
/// # use simulgine::compiler::runner::SimulgineErrors;
/// use std::path::Path;
///
///
/// # fn try_main() -> Result<(), SimulgineErrors> {
/// let res = run_free_const_expression("2 + 2")?;
/// let res = res.coerce_number_int();
/// assert_eq!(res, Some(4));
/// # Ok(())
/// # }
/// # fn main() {
/// # try_main().unwrap()
/// # }
/// ```
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

/// Spawns a Simulgine instance.
///
/// Turns a Simulgine into a SimulgineInst. Assumes the Simulgine is valid.
///
/// # Panics
///
/// This function panics if the ROOT class does not exist in the Simulgine.
pub fn run_simulgine(sim: &Simulgine) -> SimulgineInst<'_> {
    let root = UserObject::spawn_user_object(sim, *sim.user_class_names.get("ROOT").unwrap());

    SimulgineInst {
        based: sim,
        root: root.unwrap(),
    }
}
