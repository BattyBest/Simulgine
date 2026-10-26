//! AI-GENERATED TESTS. Written by Claude, not by a human maintainer.
//! See tests/ai_generated/README.md.
//!
//! Programs that compile and run but produce the wrong value. `#N` refers to the finding number
//! in the 2026-10-26 code review.

use crate::{compile, eval_const, eval_in, run};

/// Strips the `[type] ` prefix from a debug string, leaving only the value.
fn value(debug: &str) -> &str {
    debug.split_once("] ").map_or(debug, |x| x.1)
}

fn const_value(expr: &str) -> String {
    let res = eval_const(expr)
        .finished()
        .unwrap_or_else(|e| panic!("`{expr}` should evaluate, got error {e}"));
    value(&res).to_string()
}

// #1: binary operators must follow normal precedence and be left-associative.

#[test]
fn multiplication_binds_tighter_than_addition() {
    assert_eq!(const_value("2*3+4"), "10");
    assert_eq!(const_value("2+3*4"), "14");
}

#[test]
fn subtraction_is_left_associative() {
    assert_eq!(const_value("10-2-3"), "5");
}

#[test]
fn division_is_left_associative() {
    assert_eq!(const_value("100/10/5"), "2");
}

#[test]
fn comparison_binds_looser_than_arithmetic() {
    assert_eq!(const_value("1+2<4"), "true");
}

#[test]
fn precedence_inside_field_body() {
    let res = run("class ROOT { u64 x 2*3+4; }", 1, &["x"]).finished();
    assert_eq!(res, ["[u64] 10"]);
}

// #2: literal arithmetic must not be done in the smallest type that fits each literal.

#[test]
fn literal_sum_in_u64_field_does_not_wrap_at_u8() {
    let res = run("class ROOT { u64 x 255 + 1; }", 1, &["x"]).finished();
    assert_eq!(res, ["[u64] 256"]);
}

#[test]
fn const_literal_sum_does_not_wrap_at_u8() {
    assert_eq!(const_value("200+100"), "300");
}

// #3 and #4: unsigned operands assigned to a signed field must not wrap, and the stored value
// must have the field's declared type.

#[test]
fn unsigned_subtraction_into_signed_field() {
    let src = "class ROOT { u32 a 3; u32 b 5; i64 p::<2> parent.a - parent.b; }";
    let res = run(src, 1, &["p"]).finished();
    assert_eq!(res, ["[i64] -2"]);
}

#[test]
fn widened_field_value_has_declared_type() {
    let src = "class ROOT { u8 a 5; u64 b::<2> parent.a; }";
    let res = run(src, 1, &["b"]).finished();
    assert_eq!(res, ["[u64] 5"]);
}

// #5: an if/else must take the wider of its two branch types.

#[test]
fn if_else_keeps_wider_branch_type() {
    assert_eq!(const_value("(if true 300 else 1) + 1"), "301");
}

// #6: `^` must work with literal exponents, and `%` must bind like `*` and `/`.

#[test]
fn pow_with_literal_exponent() {
    assert_eq!(const_value("2^3"), "8");
}

#[test]
fn modulus_has_multiplicative_precedence() {
    // (2*7)%4 = 2; if % binds tighter, 2*(7%4) = 6.
    assert_eq!(const_value("2*7%4"), "2");
}

// #7: `-` must not be part of an identifier, so `a-1` is a subtraction.

#[test]
fn minus_without_spaces_is_subtraction() {
    let src = "class ROOT { u8 a 5; u8 b::<2> parent.a-1; }";
    let res = run(src, 1, &["b"]).finished();
    assert_eq!(res, ["[u8] 4"]);
}

// #8: `instanceof` must work outside the REPL, where the expected type is not "anything".

#[test]
fn instanceof_primitive_in_field_body() {
    let res = compile("class ROOT { u8 x instanceof u8; }").finished();
    assert!(res.is_ok(), "should compile, got {res:?}");
}

#[test]
fn instanceof_user_class_in_field_body() {
    let src = "class A { u8 v 1; }\nclass ROOT { A a instanceof A; }";
    let res = compile(src).finished();
    assert!(res.is_ok(), "should compile, got {res:?}");
}

// #9: REPL expressions must reject trailing tokens instead of ignoring them.

#[test]
fn trailing_tokens_are_an_error() {
    let res = eval_const("1 2").finished();
    assert!(res.is_err(), "`1 2` should be an error, got {res:?}");
}

// #10: an unterminated string must be an error, not a silently truncated string.

#[test]
fn unterminated_string_is_an_error() {
    let res = eval_const("\"abc").finished();
    assert!(res.is_err(), "`\"abc` should be an error, got {res:?}");
}

// Regression guard: protected fields stay readable from the REPL.

#[test]
fn repl_reads_protected_field() {
    let res = eval_in("class ROOT { u8 a 7; }", 1, "root.a").finished();
    assert_eq!(res.map(|x| value(&x).to_string()), Ok("7".to_string()));
}
