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
