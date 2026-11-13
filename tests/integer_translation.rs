//! FR-012 translation of integer `analysis_claim` terms (TC-018).
//!
//! The packages are built at test time through the public path, as in `package_translation`.
//! Integer operands must share one integer type, so every literal here is typed by the same
//! `integer_range` as the parameters.

use ix_trace_rs::trace;
use quire_analyze::{translate_analysis_claims, TranslationErrorKind};
use quire_contract_model::{
    ComparisonOperator, ContractPackageV1, Expression, ExpressionKind, IntegerDomain,
    NumericOperator, OverflowPolicy, ValueDeclarationKind, ValueType,
};

mod common;

use common::*;

const BOOLEAN: &str = "scalar_type/boolean";
const NEGATE: &str = "quire.op.integer.negate";
const ADD: &str = "quire.op.integer.add";
const SUB: &str = "quire.op.integer.sub";
const MUL: &str = "quire.op.integer.mul";

fn assert_refused_at(
    package: &ContractPackageV1,
    error: &quire_analyze::TranslationError,
    label: &str,
) {
    let node = index(package, label);
    assert_eq!(error.kind(), TranslationErrorKind::CannotLower, "{error}");
    assert_eq!(error.node(), Some(node), "{error}");
    assert!(
        error
            .to_string()
            .contains(&format!("semantic_graph.nodes[{node}]")),
        "{error}"
    );
}

/// The translation does not stop at a size or depth bound. Lowering of the integer terms
/// belongs to the analysis lowering, so another refusal from it is not a bound refusal.
fn assert_within_bounds(document: &Document, claim: &str) {
    let package = document.package(&[claim]);
    if let Err(error) = translate_analysis_claims(&package) {
        assert!(
            !matches!(
                error.kind(),
                TranslationErrorKind::ExpressionNodeBound
                    | TranslationErrorKind::ExpressionTotalBound
                    | TranslationErrorKind::ExpressionDepthBound
            ),
            "{error}"
        );
    }
}

fn operands(expression: &Expression) -> (&Expression, &Expression) {
    match expression.kind() {
        ExpressionKind::Numeric { left, right, .. }
        | ExpressionKind::Compare { left, right, .. }
        | ExpressionKind::Boolean { left, right, .. } => (left, right),
        other => panic!("not a binary expression: {other:?}"),
    }
}

#[trace("FR-012-AC-4")]
#[ignore = "needs VER-71 lowering"]
#[test]
fn each_integer_operation_translates_to_its_expression_kind() {
    let numeric = [
        (ADD, NumericOperator::Add),
        (SUB, NumericOperator::Subtract),
        (MUL, NumericOperator::Multiply),
    ];
    for (identity, expected) in numeric {
        let mut document = Document::new();
        let range = document.range();
        let a = document.parameter_of(&range);
        let unit = document.int_literal(&range, 1);
        let application = document.int_op(identity, &[&a, &unit], &range);
        let b = document.parameter_of(&range);
        let compare = document.int_op("quire.op.integer.le", &[&application, &b], BOOLEAN);
        let claim = document.claim(&compare);
        let (package, translation) = translate(&document, &[&claim]);
        let (sum, _) = operands(expression(&translation, 0));
        let ExpressionKind::Numeric {
            operator,
            left,
            right,
        } = sum.kind()
        else {
            panic!("{identity} translated to {:?}", sum.kind());
        };
        assert_eq!(*operator, expected, "{identity}");
        assert_eq!(left.kind(), &value_reference(&package, &a));
        let ExpressionKind::IntegerLiteral { value, value_type } = right.kind() else {
            panic!("literal translated to {:?}", right.kind());
        };
        assert_eq!(*value, 1);
        assert_eq!(
            (
                value_type.domain(),
                value_type.minimum(),
                value_type.maximum(),
                value_type.overflow()
            ),
            (IntegerDomain::Signed, 0, 100, OverflowPolicy::Reject)
        );
    }

    let mut document = Document::new();
    let range = document.range_between("signed", -100, 100);
    let a = document.parameter_of(&range);
    let negated = document.int_op(NEGATE, &[&a], &range);
    let b = document.parameter_of(&range);
    let compare = document.int_op("quire.op.integer.eq", &[&negated, &b], BOOLEAN);
    let claim = document.claim(&compare);
    let (package, translation) = translate(&document, &[&claim]);
    let (negate, _) = operands(expression(&translation, 0));
    match negate.kind() {
        ExpressionKind::NumericNegate { operand } => {
            assert_eq!(operand.kind(), &value_reference(&package, &a));
        }
        other => panic!("negate translated to {other:?}"),
    }
}

#[trace("FR-012-AC-4")]
#[ignore = "needs VER-71 lowering"]
#[test]
fn each_integer_comparison_translates_to_its_operator() {
    let comparisons = [
        ("eq", ComparisonOperator::Equal),
        ("ne", ComparisonOperator::NotEqual),
        ("lt", ComparisonOperator::Less),
        ("le", ComparisonOperator::LessEqual),
        ("gt", ComparisonOperator::Greater),
        ("ge", ComparisonOperator::GreaterEqual),
    ];
    for (name, expected) in comparisons {
        let mut document = Document::new();
        let range = document.range();
        let a = document.parameter_of(&range);
        let b = document.parameter_of(&range);
        let identity = format!("quire.op.integer.{name}");
        let compare = document.int_op(&identity, &[&a, &b], BOOLEAN);
        let claim = document.claim(&compare);
        let (package, translation) = translate(&document, &[&claim]);
        let ExpressionKind::Compare {
            operator,
            left,
            right,
        } = expression(&translation, 0).kind()
        else {
            panic!("{name} did not translate to a comparison");
        };
        assert_eq!(*operator, expected, "{name}");
        assert_eq!(left.kind(), &value_reference(&package, &a));
        assert_eq!(right.kind(), &value_reference(&package, &b));
    }
}

#[trace("FR-012-AC-4")]
#[ignore = "needs VER-71 lowering"]
#[test]
fn a_comparison_nests_under_a_boolean_operation() {
    let mut document = Document::new();
    let range = document.range();
    let a = document.parameter_of(&range);
    let b = document.parameter_of(&range);
    let p = document.parameter();
    let less = document.int_op("quire.op.integer.lt", &[&a, &b], BOOLEAN);
    let and = document.op(AND, &[&less, &p]);
    let claim = document.claim(&and);
    let (_, translation) = translate(&document, &[&claim]);
    let (left, _) = operands(expression(&translation, 0));
    assert!(matches!(
        left.kind(),
        ExpressionKind::Compare {
            operator: ComparisonOperator::Less,
            ..
        }
    ));
}

#[trace("FR-012-AC-4")]
#[test]
fn an_integer_operand_of_a_boolean_operation_is_refused_at_the_node() {
    // The checked-package reader refuses ill-typed operands, so the types are edited afterwards.
    let mut document = Document::new();
    let p = document.parameter();
    let q = document.parameter();
    let and = document.op(AND, &[&p, &q]);
    let claim = document.claim(&and);
    let bytes = with_node(&document.bytes(&[&claim]), &q, |node| {
        node["semantic_type"] = id("scalar_type/integer");
    });
    let package = read_package(&bytes);
    let error = translate_analysis_claims(&package).expect_err("refused");
    assert_refused_at(&package, &error, &q);

    let mut document = Document::new();
    let p = document.parameter();
    let q = document.parameter();
    let inner = document.op(EQ, &[&p, &q]);
    let and = document.op(AND, &[&inner, &p]);
    let claim = document.claim(&and);
    let bytes = with_identity(&document.bytes(&[&claim]), &inner, ADD);
    let package = read_package(&bytes);
    let error = translate_analysis_claims(&package).expect_err("refused");
    assert_refused_at(&package, &error, &inner);
}

#[trace("FR-012-AC-4")]
#[test]
fn a_boolean_operand_of_an_integer_operation_is_refused_at_the_node() {
    let mut document = Document::new();
    let range = document.range();
    let a = document.parameter_of(&range);
    let b = document.parameter_of(&range);
    let less = document.int_op("quire.op.integer.lt", &[&a, &b], BOOLEAN);
    let claim = document.claim(&less);
    let bytes = with_node(&document.bytes(&[&claim]), &b, |node| {
        node["semantic_type"] = id(BOOLEAN);
    });
    let package = read_package(&bytes);
    let error = translate_analysis_claims(&package).expect_err("refused");
    assert_refused_at(&package, &error, &b);
}

#[trace("FR-012-AC-3")]
#[test]
fn a_product_of_two_parameters_is_refused_at_the_product() {
    let mut document = Document::new();
    let range = document.range();
    let a = document.parameter_of(&range);
    let b = document.parameter_of(&range);
    let product = document.int_op(MUL, &[&a, &b], &range);
    let c = document.parameter_of(&range);
    let compare = document.int_op("quire.op.integer.le", &[&product, &c], BOOLEAN);
    let claim = document.claim(&compare);
    let (package, error) = refusal(&document, &[&claim]);
    assert_refused_at(&package, &error, &product);

    // Parameters nested under the operands count as well.
    let mut document = Document::new();
    let range = document.range();
    let a = document.parameter_of(&range);
    let b = document.parameter_of(&range);
    let zero = document.int_literal(&range, 0);
    let left = document.int_op(ADD, &[&a, &zero], &range);
    let right = document.int_op(SUB, &[&b, &zero], &range);
    let product = document.int_op(MUL, &[&left, &right], &range);
    let compare = document.int_op("quire.op.integer.le", &[&product, &a], BOOLEAN);
    let claim = document.claim(&compare);
    let (package, error) = refusal(&document, &[&claim]);
    assert_refused_at(&package, &error, &product);
}

#[trace("FR-012-AC-3")]
#[ignore = "needs VER-71 lowering"]
#[test]
fn a_product_with_a_constant_operand_is_linear() {
    let mut document = Document::new();
    let range = document.range();
    let a = document.parameter_of(&range);
    let one = document.int_literal(&range, 1);
    let product = document.int_op(MUL, &[&one, &a], &range);
    let compare = document.int_op("quire.op.integer.le", &[&product, &a], BOOLEAN);
    let claim = document.claim(&compare);
    translate(&document, &[&claim]);
}

#[trace("FR-012-AC-3")]
#[test]
fn division_remainder_and_modulus_are_refused_at_the_application() {
    for name in ["div", "rem", "mod"] {
        let mut document = Document::new();
        let range = document.range();
        let a = document.parameter_of(&range);
        let b = document.parameter_of(&range);
        let identity = format!("quire.op.integer.{name}");
        let quotient = document.int_op(ADD, &[&a, &b], &range);
        let compare = document.int_op("quire.op.integer.le", &[&quotient, &a], BOOLEAN);
        let claim = document.claim(&compare);
        let bytes = with_identity(&document.bytes(&[&claim]), &quotient, &identity);
        let package = read_package(&bytes);
        let error = translate_analysis_claims(&package).expect_err("refused");
        assert_refused_at(&package, &error, &quotient);
        assert!(error.message().contains(&identity), "{error}");
    }
}

#[trace("FR-012-AC-3")]
#[test]
fn an_integer_reference_cycle_is_refused() {
    let mut document = Document::new();
    let range = document.range();
    let a = document.parameter_of(&range);
    let zero = document.int_literal(&range, 0);
    let first = document.int_op(ADD, &[&a, &zero], &range);
    let second = document.int_op(ADD, &[&first, &zero], &range);
    document.set_body(
        &first,
        serde_json::json!({
            "term": "application",
            "operator": "binary",
            "operation": {"identity": ADD, "laws": [], "mode": null, "member": null, "leaves": []},
            "result_type": id(&range),
            "arguments": [reference(&second), reference(&zero)],
        }),
        &[&second, &zero],
    );
    for label in [&first, &second] {
        let node = document
            .nodes
            .iter_mut()
            .find(|node| node["node_id"] == id(label))
            .expect("node");
        node["recursion_group"] = serde_json::json!("loop");
    }
    let compare = document.int_op("quire.op.integer.le", &[&second, &a], BOOLEAN);
    let claim = document.claim(&compare);
    let (package, error) = refusal(&document, &[&claim]);
    assert_eq!(error.kind(), TranslationErrorKind::CannotLower, "{error}");
    assert!(error.message().contains("cycle"), "{error}");
    let named = error.node().expect("a node");
    assert!([index(&package, &first), index(&package, &second)].contains(&named));
}

#[trace("FR-012-AC-5")]
#[ignore = "needs VER-71 lowering"]
#[test]
fn integer_parameters_are_declared_once_with_their_bounds() {
    let mut document = Document::new();
    let range = document.range();
    let a = document.parameter_of(&range);
    let b = document.parameter_of(&range);
    let first = document.int_op("quire.op.integer.le", &[&a, &b], BOOLEAN);
    let second = document.int_op("quire.op.integer.ge", &[&a, &b], BOOLEAN);
    let claim_a = document.claim(&first);
    let claim_b = document.claim(&second);
    let (package, translation) = translate(&document, &[&claim_a, &claim_b]);

    let declarations = translation.environment().values();
    let mut expected = [index(&package, &a), index(&package, &b)];
    expected.sort_unstable();
    let names = declarations
        .iter()
        .map(|declaration| declaration.name().as_str().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(names, expected.map(|node| format!("n{node}")).to_vec());
    for declaration in declarations {
        assert_eq!(declaration.kind(), ValueDeclarationKind::Input);
        let ValueType::Integer { value } = declaration.value_type() else {
            panic!("not an integer declaration: {:?}", declaration.value_type());
        };
        assert_eq!(
            (
                value.domain(),
                value.minimum(),
                value.maximum(),
                value.overflow()
            ),
            (IntegerDomain::Signed, 0, 100, OverflowPolicy::Reject)
        );
    }
}

#[trace("FR-012-AC-6")]
#[ignore = "needs VER-71 lowering"]
#[test]
fn two_translations_of_the_same_integer_bytes_agree() {
    let mut document = Document::new();
    let range = document.range();
    let dose = document.parameter_of(&range);
    let weight = document.parameter_of(&range);
    let one = document.int_literal(&range, 1);
    let scaled = document.int_op(MUL, &[&weight, &one], &range);
    let bounded = document.int_op("quire.op.integer.le", &[&dose, &scaled], BOOLEAN);
    let claim = document.claim(&bounded);
    let package = document.package(&[&claim]);
    let first = translate_analysis_claims(&package).expect("first");
    let second = translate_analysis_claims(&package).expect("second");
    assert_eq!(first.package(), second.package());
    assert_eq!(first.environment(), second.environment());
}

#[trace("FR-012-AC-7")]
#[test]
fn the_depth_bound_over_integer_sums_is_128_against_129() {
    let build = |additions: usize| {
        let mut document = Document::new();
        let range = document.range();
        let a = document.parameter_of(&range);
        let zero = document.int_literal(&range, 0);
        let innermost = document.int_op(ADD, &[&a, &zero], &range);
        let mut top = innermost.clone();
        for _ in 1..additions {
            top = document.int_op(ADD, &[&top, &zero], &range);
        }
        let compare = document.int_op("quire.op.integer.le", &[&top, &a], BOOLEAN);
        let claim = document.claim(&compare);
        (document, claim, innermost)
    };
    let (document, claim, _) = build(126);
    assert_within_bounds(&document, &claim);

    let (document, claim, innermost) = build(127);
    let (package, error) = refusal(&document, &[&claim]);
    assert_eq!(
        error.kind(),
        TranslationErrorKind::ExpressionDepthBound,
        "{error}"
    );
    assert_eq!(error.node(), Some(index(&package, &innermost)));
}

#[trace("FR-012-AC-7")]
#[test]
fn the_node_bound_over_integer_sums_is_10000_against_10001() {
    // Over the one-point range `[0, 0]` every sum stays inside its named bounds.
    let build = |size: usize| {
        let mut document = Document::new();
        let range = document.range_between("point", 0, 0);
        let mut sized = Sized::sums(&mut document, &range);
        let sum = sized.node(&mut document, size);
        let compare = document.int_op("quire.op.integer.le", &[&sum, &sized.parameter], BOOLEAN);
        let claim = document.claim(&compare);
        (document, claim)
    };
    // The comparison and its right operand add two nodes to the sum.
    let (document, claim) = build(9_998);
    assert_within_bounds(&document, &claim);

    let (document, claim) = build(9_999);
    let (_, error) = refusal(&document, &[&claim]);
    assert_eq!(
        error.kind(),
        TranslationErrorKind::ExpressionNodeBound,
        "{error}"
    );
}

#[trace("FR-012-AC-7")]
#[test]
fn a_shared_integer_sum_chain_of_61_nodes_is_refused_without_expanding_it() {
    let mut document = Document::new();
    let range = document.range_between("point", 0, 0);
    let a = document.parameter_of(&range);
    let mut top = document.int_op(ADD, &[&a, &a], &range);
    for _ in 1..61 {
        top = document.int_op(ADD, &[&top, &top], &range);
    }
    let compare = document.int_op("quire.op.integer.le", &[&top, &a], BOOLEAN);
    let claim = document.claim(&compare);
    let package = document.package(&[&claim]);
    let started = std::time::Instant::now();
    let error = translate_analysis_claims(&package).expect_err("2^61 expansion refused");
    assert_eq!(
        error.kind(),
        TranslationErrorKind::ExpressionNodeBound,
        "{error}"
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
}
