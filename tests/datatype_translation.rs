//! FR-022 translation of enum, record and option `analysis_claim` terms (TC-020).
//!
//! Every contract package is built at test time through the public path, as in
//! `package_translation`. Enum claims also lower, so they run end to end. Record and option
//! claims translate and then need the record and option lowering, so their success-path tests
//! are ignored until that lowering lands.

use ix_trace_rs::trace;
use quire_analyze::{
    execute_differential, lower_analysis_request, translate_analysis_claims, AdapterLimits,
    AnalysisStatus, CancellationToken, SolverConfig, SolverEngine, TranslationErrorKind,
};
use quire_contract_model::{
    BooleanOperator, ComparisonOperator, ContractPackageV1, Expression, ExpressionKind,
    TypeDeclaration, ValueDeclarationKind, ValueType,
};

mod common;

use common::*;

const ENUM_EQ: &str = "quire.op.enum.eq";
const ENUM_NE: &str = "quire.op.enum.ne";
const STRUCTURAL_EQ: &str = "quire.op.structural.eq";

fn kind_of(expression: &Expression) -> &ExpressionKind {
    expression.kind()
}

fn compare(expression: &Expression) -> (ComparisonOperator, &Expression, &Expression) {
    match kind_of(expression) {
        ExpressionKind::Compare {
            operator,
            left,
            right,
        } => (*operator, left, right),
        other => panic!("not a comparison: {other:?}"),
    }
}

fn enum_literal(expression: &Expression) -> (&str, &str) {
    match kind_of(expression) {
        ExpressionKind::EnumLiteral {
            enumeration,
            variant,
        } => (enumeration.as_str(), variant.as_str()),
        other => panic!("not an enum literal: {other:?}"),
    }
}

/// `role = Admin or role = Auditor`, then `second`: `role != Admin`, or `role = Guest`.
fn membership(guest: bool) -> (Document, Vec<String>) {
    let mut document = Document::new();
    let role = document.enum_type("Role", &["Admin", "Auditor", "Guest"]);
    let r = document.parameter_of(&role);
    let admin = document.enum_literal(&role, "Admin");
    let auditor = document.enum_literal(&role, "Auditor");
    let is_admin = document.op(ENUM_EQ, &[&r, &admin]);
    let is_auditor = document.op(ENUM_EQ, &[&r, &auditor]);
    let member = document.op(OR, &[&is_admin, &is_auditor]);
    let second = if guest {
        let guest = document.enum_literal(&role, "Guest");
        document.op(ENUM_EQ, &[&r, &guest])
    } else {
        document.op(ENUM_NE, &[&r, &admin])
    };
    let first = document.claim(&member);
    let second = document.claim(&second);
    (document, vec![first, second])
}

/// `user.role = Admin`, and with `guest` also `user.role = Guest`.
fn record_field(guest: bool) -> (Document, Vec<String>) {
    let mut document = Document::new();
    let role = document.enum_type("Role", &["Admin", "Auditor", "Guest"]);
    let user = document.record_type("User", &[("role", &role)]);
    let u = document.parameter_of(&user);
    let projected = document.project(&user, "role", &u, &role);
    let admin = document.enum_literal(&role, "Admin");
    let is_admin = document.op(ENUM_EQ, &[&projected, &admin]);
    let mut claims = vec![document.claim(&is_admin)];
    if guest {
        let guest = document.enum_literal(&role, "Guest");
        let is_guest = document.op(ENUM_EQ, &[&projected, &guest]);
        claims.push(document.claim(&is_guest));
    }
    (document, claims)
}

/// `present(o) => value(o) = Admin`, or with `guarded` false only `value(o) = Admin`.
fn option_value(guarded: bool) -> (Document, Vec<String>, String) {
    let mut document = Document::new();
    let role = document.enum_type("Role", &["Admin", "Auditor", "Guest"]);
    let maybe = document.option_type("MaybeRole", &role);
    let o = document.parameter_of(&maybe);
    let value = document.option_value(&o, &role);
    let admin = document.enum_literal(&role, "Admin");
    let is_admin = document.op(ENUM_EQ, &[&value, &admin]);
    let body = if guarded {
        let present = document.present(&o);
        document.op(IMPLIES, &[&present, &is_admin])
    } else {
        is_admin
    };
    let claim = document.claim(&body);
    (document, vec![claim], value)
}

fn labels(claims: &[String]) -> Vec<&str> {
    claims.iter().map(String::as_str).collect()
}

#[trace("FR-022-AC-1", "FR-022-AC-2", "TC-020")]
#[test]
fn an_enum_membership_disjunction_translates_to_enum_comparisons() {
    let (document, claims) = membership(false);
    let (package, translation) = translate(&document, &labels(&claims));

    let ExpressionKind::Boolean {
        operator: BooleanOperator::TotalOr,
        left,
        right,
    } = kind_of(expression(&translation, 0))
    else {
        panic!("not a total or");
    };
    let r = value_reference(&package, "value/parameter/1");
    for (side, variant) in [(left, "Admin"), (right, "Auditor")] {
        let (operator, operand, literal) = compare(side);
        assert_eq!(operator, ComparisonOperator::Equal);
        assert_eq!(kind_of(operand), &r);
        let (enumeration, found) = enum_literal(literal);
        assert_eq!(found, variant);
        assert_eq!(enumeration, format!("t{}", index(&package, "Role")));
    }
    let (operator, operand, literal) = compare(expression(&translation, 1));
    assert_eq!(operator, ComparisonOperator::NotEqual);
    assert_eq!(kind_of(operand), &r);
    assert_eq!(enum_literal(literal).1, "Admin");

    // The type and the value come from the package's semantic_type nodes, and nothing else.
    let types = translation.environment().types();
    assert_eq!(types.len(), 1);
    let TypeDeclaration::Enum { declaration } = &types[0] else {
        panic!("not an enum");
    };
    let variants = declaration
        .variants()
        .iter()
        .map(|variant| variant.name().as_str())
        .collect::<Vec<_>>();
    assert_eq!(variants, ["Admin", "Auditor", "Guest"]);
    let values = translation.environment().values();
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].kind(), ValueDeclarationKind::Input);
    assert_eq!(
        values[0].value_type(),
        &ValueType::Enum {
            name: declaration.name().clone()
        }
    );
}

#[trace("FR-022-AC-1", "TC-020")]
#[test]
fn an_enum_member_node_reference_translates_to_the_enum_literal() {
    let mut document = Document::new();
    let role = document.enum_type("Role", &["Admin", "Guest"]);
    let r = document.parameter_of(&role);
    let is_guest = document.op(ENUM_EQ, &[&r, "Role/Guest"]);
    let claim = document.claim(&is_guest);
    let (_, translation) = translate(&document, &[&claim]);
    let (_, _, literal) = compare(expression(&translation, 0));
    assert_eq!(enum_literal(literal).1, "Guest");
}

#[trace("FR-022-AC-1", "TC-020")]
#[test]
fn the_enum_fixtures_lower_to_a_query() {
    for guest in [false, true] {
        let (document, claims) = membership(guest);
        let (_, translation) = translate(&document, &labels(&claims));
        lower_analysis_request(translation.request()).expect("the enum claims lower");
    }
}

#[trace("FR-022-AC-4", "TC-020")]
#[test]
fn the_same_package_translates_to_the_same_clauses_declarations_and_query() {
    let (document, claims) = membership(false);
    let bytes = document.bytes(&labels(&claims));
    let first = translate_analysis_claims(&read_package(&bytes)).expect("first");
    let second = translate_analysis_claims(&read_package(&bytes)).expect("second");
    assert_eq!(first.package(), second.package());
    assert_eq!(first.environment(), second.environment());
    let query = |translation: &quire_analyze::AnalysisTranslation| {
        format!(
            "{:?}",
            lower_analysis_request(translation.request()).expect("query")
        )
    };
    assert_eq!(query(&first), query(&second));
}

#[trace("FR-022-AC-1", "FR-022-AC-2", "TC-020")]
#[test]
#[ignore = "needs VER-86 record/option lowering"]
fn a_record_field_claim_translates_to_a_field_access() {
    let (document, claims) = record_field(true);
    let (package, translation) = translate(&document, &labels(&claims));
    let (operator, projected, literal) = compare(expression(&translation, 0));
    assert_eq!(operator, ComparisonOperator::Equal);
    let ExpressionKind::FieldAccess { base, field } = kind_of(projected) else {
        panic!("not a field access: {projected:?}");
    };
    assert_eq!(field.as_str(), "role");
    assert_eq!(
        kind_of(base),
        &value_reference(&package, "value/parameter/1")
    );
    assert_eq!(enum_literal(literal).1, "Admin");
    assert_eq!(translation.environment().types().len(), 2);
    let TypeDeclaration::Record { declaration } = &translation.environment().types()[1] else {
        panic!("not a record");
    };
    assert_eq!(declaration.fields()[0].name().as_str(), "role");
}

#[trace("FR-022-AC-1", "TC-020")]
#[test]
#[ignore = "needs VER-86 record/option lowering"]
fn a_guarded_option_value_translates_to_an_unwrap_under_the_presence_guard() {
    let (document, claims, _) = option_value(true);
    let (package, translation) = translate(&document, &labels(&claims));
    let ExpressionKind::Boolean {
        operator: BooleanOperator::Implication,
        left,
        right,
    } = kind_of(expression(&translation, 0))
    else {
        panic!("not an implication");
    };
    let o = value_reference(&package, "value/parameter/2");
    let ExpressionKind::IsPresent { option } = kind_of(left) else {
        panic!("not a presence test");
    };
    assert_eq!(kind_of(option), &o);
    let (_, unwrapped, _) = compare(right);
    let ExpressionKind::Unwrap { option } = kind_of(unwrapped) else {
        panic!("not an unwrap");
    };
    assert_eq!(kind_of(option), &o);
}

#[trace("FR-022-AC-1", "TC-020")]
#[test]
#[ignore = "needs VER-86 record/option lowering"]
fn a_none_literal_translates_to_option_none() {
    let mut document = Document::new();
    let role = document.enum_type("Role", &["Admin", "Guest"]);
    let maybe = document.option_type("MaybeRole", &role);
    let o = document.parameter_of(&maybe);
    let none = document.none_literal(&maybe);
    let same = document.op(STRUCTURAL_EQ, &[&o, &none]);
    let claim = document.claim(&same);
    let (_, translation) = translate(&document, &[&claim]);
    let (operator, _, none) = compare(expression(&translation, 0));
    assert_eq!(operator, ComparisonOperator::Equal);
    assert!(matches!(kind_of(none), ExpressionKind::OptionNone { .. }));
}

#[trace("FR-022-AC-1", "TC-020")]
#[test]
fn an_unguarded_option_value_is_refused_by_the_validator_naming_the_claim() {
    let (document, claims, value) = option_value(false);
    let (package, error) = refusal(&document, &labels(&claims));
    let claim = index(&package, &claims[0]);
    assert_eq!(
        error.kind(),
        TranslationErrorKind::ClauseRejected,
        "{error}"
    );
    assert_eq!(error.clause(), Some(format!("claim-{claim}").as_str()));
    assert!(
        !error.path().starts_with("semantic_graph.nodes["),
        "{error}"
    );
    assert_ne!(error.node(), Some(index(&package, &value)));
}

fn assert_refused_at(package: &ContractPackageV1, label: &str) {
    let error = translate_analysis_claims(package).expect_err("refused");
    let node = index(package, label);
    assert_eq!(error.kind(), TranslationErrorKind::CannotLower, "{error}");
    assert_eq!(error.node(), Some(node), "{error}");
    assert_eq!(error.path(), format!("semantic_graph.nodes[{node}]"));
}

#[trace("FR-022-AC-2", "TC-020")]
#[test]
fn a_parameter_of_an_unsupported_type_is_refused_at_the_type_node() {
    // An option over text is not a supported payload.
    let mut document = Document::new();
    let text = document.unsupported_type("text");
    let maybe = document.option_type("MaybeText", &text);
    let o = document.parameter_of(&maybe);
    let present = document.present(&o);
    let claim = document.claim(&present);
    let package = document.package(&[&claim]);
    assert_refused_at(&package, &text);
}

#[trace("FR-022-AC-3", "TC-020")]
#[test]
fn a_record_that_nests_text_is_refused_at_the_text_type() {
    let mut document = Document::new();
    let role = document.enum_type("Role", &["Admin", "Guest"]);
    let text = document.unsupported_type("text");
    let user = document.record_type("User", &[("role", &role), ("name", &text)]);
    let u = document.parameter_of(&user);
    let projected = document.project(&user, "role", &u, &role);
    let admin = document.enum_literal(&role, "Admin");
    let is_admin = document.op(ENUM_EQ, &[&projected, &admin]);
    let claim = document.claim(&is_admin);
    let package = document.package(&[&claim]);
    assert_refused_at(&package, &text);
}

#[trace("FR-022-AC-3", "TC-020")]
#[test]
fn a_record_that_nests_a_collection_or_rational_is_refused_at_that_type() {
    for form in ["sequence", "rational"] {
        let mut document = Document::new();
        let role = document.enum_type("Role", &["Admin", "Guest"]);
        let nested = document.unsupported_type(form);
        let user = document.record_type("User", &[("role", &role), ("extra", &nested)]);
        let u = document.parameter_of(&user);
        let projected = document.project(&user, "role", &u, &role);
        let admin = document.enum_literal(&role, "Admin");
        let is_admin = document.op(ENUM_EQ, &[&projected, &admin]);
        let claim = document.claim(&is_admin);
        let package = document.package(&[&claim]);
        assert_refused_at(&package, &nested);
    }
}

#[trace("FR-022-AC-3", "TC-020")]
#[test]
fn an_identity_outside_the_table_is_refused_at_the_application() {
    for identity in [
        "quire.op.collection.contains",
        "quire.op.enum.lt",
        "quire.op.text.eq",
    ] {
        let mut document = Document::new();
        let role = document.enum_type("Role", &["Admin", "Guest"]);
        let r = document.parameter_of(&role);
        let admin = document.enum_literal(&role, "Admin");
        let is_admin = document.op(ENUM_EQ, &[&r, &admin]);
        let claim = document.claim(&is_admin);
        let edited = with_identity(&document.bytes(&[&claim]), &is_admin, identity);
        let package = read_package(&edited);
        let error = translate_analysis_claims(&package).expect_err(identity);
        let node = index(&package, &is_admin);
        assert_eq!(error.kind(), TranslationErrorKind::CannotLower, "{error}");
        assert_eq!(error.node(), Some(node), "{identity}: {error}");
        assert!(error.message().contains(identity), "{error}");
    }
}

#[trace("FR-022-AC-5", "TC-020")]
#[test]
fn a_shared_enum_chain_over_the_node_bound_is_refused_without_expanding_it() {
    let mut document = Document::new();
    let role = document.enum_type("Role", &["Admin", "Guest"]);
    let r = document.parameter_of(&role);
    let admin = document.enum_literal(&role, "Admin");
    let mut top = document.op(ENUM_EQ, &[&r, &admin]);
    // 3 nodes doubled and one more each level: 2^(k+2) - 1, over 10,000 at k = 12.
    for _ in 0..12 {
        top = document.op(OR, &[&top, &top]);
    }
    let claim = document.claim(&top);
    let (_, error) = refusal(&document, &[&claim]);
    assert_eq!(
        error.kind(),
        TranslationErrorKind::ExpressionNodeBound,
        "{error}"
    );
}

#[trace("FR-022-AC-5", "TC-020")]
#[test]
fn an_enum_comparison_under_a_deep_not_chain_is_refused_at_the_depth_bound() {
    let mut document = Document::new();
    let role = document.enum_type("Role", &["Admin", "Guest"]);
    let r = document.parameter_of(&role);
    let admin = document.enum_literal(&role, "Admin");
    let mut top = document.op(ENUM_EQ, &[&r, &admin]);
    for _ in 0..128 {
        top = document.op(NOT, &[&top]);
    }
    let claim = document.claim(&top);
    let (_, error) = refusal(&document, &[&claim]);
    assert_eq!(
        error.kind(),
        TranslationErrorKind::ExpressionDepthBound,
        "{error}"
    );
}

/// FR-022-AC-1 on the official engines: the contradictory membership is refuted, and the
/// satisfiable one is never refuted. Run with `QUIRE_Z3` and `QUIRE_CVC5` set to the executables.
#[trace("FR-022-AC-1", "TC-020")]
#[test]
#[ignore = "requires official Z3 and cvc5 executables at QUIRE_Z3 and QUIRE_CVC5"]
fn the_official_engines_decide_the_enum_membership_fixtures() {
    let path = |name: &str| std::path::PathBuf::from(std::env::var_os(name).expect(name));
    let z3 = SolverConfig::new(SolverEngine::Z3, path("QUIRE_Z3"), AdapterLimits::default())
        .expect("Z3 configuration");
    let cvc5 = SolverConfig::new(
        SolverEngine::Cvc5,
        path("QUIRE_CVC5"),
        AdapterLimits::default(),
    )
    .expect("cvc5 configuration");
    for (guest, refuted) in [(false, false), (true, true)] {
        let (document, claims) = membership(guest);
        let (_, translation) = translate(&document, &labels(&claims));
        let query = lower_analysis_request(translation.request()).expect("query");
        let run = execute_differential(&query, &z3, &cvc5, &CancellationToken::default())
            .expect("differential run");
        assert_eq!(
            run.agreed_status() == Some(AnalysisStatus::Refuted),
            refuted,
            "guest {guest}: {run:#?}"
        );
    }
}
