//! AI-GENERATED TESTS. Written by Claude, not by a human maintainer.
//! See tests/ai_generated/README.md.
//!
//! Inputs that currently panic, hang, or abort. Bad programs should produce an error, and a bad
//! value at runtime should not crash the simulation. `#N` refers to the finding number in the
//! 2026-10-26 code review.

use crate::{compile, eval_const, guarded, project, repl_exits_cleanly, run, Outcome};

use simulgine::{
    compiler::runner::{compile_directory, tick_sim},
    run_simulgine,
};

#[track_caller]
fn assert_compile_error(src: &str) {
    let res = compile(src).finished();
    assert!(res.is_err(), "should fail to compile, got {res:?}");
}

#[track_caller]
fn assert_const_error(expr: &str) {
    let res = eval_const(expr).finished();
    assert!(res.is_err(), "`{expr}` should be an error, got {res:?}");
}

/// Compiles `src` (which must compile) and ticks it `ticks` times without panicking.
#[track_caller]
fn assert_ticks_without_panic(src: &str, ticks: usize) {
    let dir = project(src);
    let res = guarded(move || {
        let compiled = compile_directory(&dir).expect("program should compile");
        let mut sim = run_simulgine(&compiled);
        for _ in 0..ticks {
            tick_sim(&mut sim);
        }
    });
    if let Outcome::Panicked(msg) = &res {
        panic!("tick panicked: {msg}");
    }
    res.finished();
}

// #11: a `//` comment at end of input with no newline must not hang.

#[test]
fn trailing_comment_without_newline_in_file() {
    let res = compile("class ROOT { u8 a 1; }\n// end").finished();
    assert!(res.is_ok(), "should compile, got {res:?}");
}

#[test]
fn trailing_comment_in_repl_expression() {
    let res = eval_const("1 // note").finished();
    assert_eq!(res, Ok("[u8] 1".to_string()));
}

// #12: classes that contain themselves (directly or through another class) must be rejected,
// not recurse until the stack overflows. A stack overflow aborts the process, so these run the
// REPL binary in a child process.

#[test]
fn self_containing_class_is_rejected() {
    let dir = project("class ROOT { level ROOT child; }");
    repl_exits_cleanly(&dir, "!quit\n").unwrap();
}

#[test]
fn mutually_containing_classes_are_rejected() {
    let dir = project("class A { level ROOT r; }\nclass ROOT { level A a; }");
    repl_exits_cleanly(&dir, "!quit\n").unwrap();
}

// #13 and #14: malformed source must be a parse error, not a panic.

#[test]
fn field_with_non_identifier_type() {
    assert_compile_error("class ROOT { 5 x 1; }");
}

#[test]
fn unterminated_class() {
    assert_compile_error("class ROOT { u8 a 1;");
}

#[test]
fn syntax_error_inside_braces() {
    assert_compile_error("class ROOT { u8 x { 1 +; }; }");
}

// #15: division or modulus by zero at runtime must not crash the simulation.

#[test]
fn division_by_zero_at_runtime() {
    assert_ticks_without_panic("class ROOT { u8 z 0; u8 q::<2> 10 / parent.z; }", 1);
}

#[test]
fn modulus_by_zero_at_runtime() {
    assert_ticks_without_panic("class ROOT { u8 z 0; u8 q::<2> 10 % parent.z; }", 1);
}

#[test]
fn division_by_zero_in_repl() {
    let res = eval_const("1/0");
    assert!(
        !matches!(res, Outcome::Panicked(_)),
        "`1/0` should not panic, got {res:?}"
    );
}

// #16: negating the minimum signed value must not crash the simulation.

#[test]
fn negating_i8_min_at_runtime() {
    // Built from i8 variables only, so it does not depend on literal typing (#2).
    let src = "class ROOT { i8 a { let one: i8 = -1; let x: i8 = -127; x = x + one; -x; }; }";
    assert_ticks_without_panic(src, 1);
}

// #17: two fields with the same name must be a compile error.

#[test]
fn duplicate_field_names() {
    assert_compile_error("class ROOT { u8 a 1; u8 a 2; }");
}

// #18: assigning to `this` in the REPL, where there is no `this`, must be an error.

#[test]
fn assigning_this_in_repl() {
    assert_const_error("this = 1");
}

// #19: a lone quote must be an error.

#[test]
fn lone_quote() {
    assert_const_error("\"");
}

// #20: malformed stage numbers must be errors.

#[test]
fn empty_turbofish() {
    assert_compile_error("class ROOT { u8 a::<> 1; }");
}

#[test]
fn turbofish_with_two_dots() {
    assert_compile_error("class ROOT { u8 a::<1.2.3> 1; }");
}

// #21: a volatile field may copy another class-typed field.

#[test]
fn copying_class_typed_field() {
    let src = "class A { u8 v 1; }\nclass ROOT { level A a; A c parent.a; }";
    // Tick 1 sets a.v to 1 while c copies the old a (v = 0); tick 2 copies the new a.
    let res = run(src, 2, &["c:v"]).finished();
    assert_eq!(res, ["[u8] 1"]);
}

// #22: reading a field through `this` when `this` is a class instance must work. The block
// returns `parent.a`, so this test also needs #21 fixed, but `this.v` runs first and is what
// currently panics.

#[test]
fn reading_field_through_this() {
    let src = "class A { public u8 v 1; }\n\
               class ROOT { level A a; A c { let t: u8 = this.v; parent.a; }; }";
    assert_ticks_without_panic(src, 1);
}
