//! FR-009 translation of `analysis_claim` nodes into analysis statements (TC-015 case 19).
//!
//! Every contract package here is built at test time through the public path:
//! a `quire.checked-package/v2` document built in this file, `CheckedPackageV2::read`,
//! `CheckedPackageV2::lower` with `CompleteLoweringProfileV2`, the canonical bytes of the
//! `CompleteContractPackageV2`, and `ContractPackageWireV1::read`.

use ix_trace_rs::trace;
use quire_analyze::{
    lower_analysis_request, translate_analysis_claims, AnalysisTranslation, TranslationError,
    TranslationErrorKind,
};
use quire_contract_model::{
    BooleanOperator, CheckedNodeId, CheckedNodeTag, CheckedPackageEvidence,
    CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult, ComparisonOperator,
    CompleteLoweringProfileV2, CompleteLoweringRecordV2, ContractPackageReadResult,
    ContractPackageV1, ContractPackageWireV1, ExecutionPoint, ExpressionKind, LoweringProfileRef,
    StateObservation, ValueDeclarationKind, ValueType,
};
use serde_json::{json, Value};

const PACKAGE: &str = "test/pkg";
const FEATURE: &str = "quire.value.complete/v1";
const AND: &str = "quire.op.boolean.and";
const OR: &str = "quire.op.boolean.or";
const NOT: &str = "quire.op.boolean.not";
const IMPLIES: &str = "quire.op.boolean.implies";
const EQ: &str = "quire.op.boolean.eq";
const NE: &str = "quire.op.boolean.ne";

fn id(label: &str) -> Value {
    json!({"package": PACKAGE, "path": [label], "ordinal": 0})
}

fn reference(label: &str) -> Value {
    json!({"term": "reference", "target": id(label)})
}

fn literal(ty: &str, kind: &str, value: Value) -> Value {
    json!({"term": "literal", "type": id(ty), "value_kind": kind, "value": value})
}

/// A checked-package document under construction, in node order.
struct Document {
    name: String,
    nodes: Vec<Value>,
    regions: Vec<(u64, u64)>,
    next: usize,
}

impl Document {
    fn new() -> Self {
        let mut document = Self {
            name: PACKAGE.to_owned(),
            nodes: Vec::new(),
            regions: Vec::new(),
            next: 0,
        };
        for form in ["boolean", "integer", "text"] {
            let label = format!("scalar_type/{form}");
            document.node(
                &label,
                "scalar_type",
                "type",
                form,
                &label,
                &[],
                json!({"term": "aggregate", "members": []}),
            );
        }
        document
    }

    fn node(
        &mut self,
        label: &str,
        tag: &str,
        role: &str,
        form: &str,
        semantic_type: &str,
        dependencies: &[&str],
        body: Value,
    ) {
        let mut dependencies = dependencies.to_vec();
        dependencies.sort_unstable();
        dependencies.dedup();
        let position = self.nodes.len() as u64;
        self.nodes.push(json!({
            "node_id": id(label),
            "schema_version": "quire.checked-semantic-graph/v2",
            "node_tag": tag,
            "semantic_form": form,
            "semantic_type": id(semantic_type),
            "dependencies": dependencies.iter().map(|label| id(label)).collect::<Vec<_>>(),
            "occurrences": [{"role": role, "ordinal": 0}],
            "body": body,
        }));
        self.regions.push((position * 10 + 3, position * 10 + 7));
    }

    fn fresh(&mut self, prefix: &str) -> String {
        self.next += 1;
        format!("{prefix}/{}", self.next)
    }

    /// A parameter of the type node `scalar_type/{ty}`.
    fn parameter_of(&mut self, ty: &str) -> String {
        let label = self.fresh("value/parameter");
        let body = json!({"term": "aggregate", "members": [
            {"term": "binding", "name": "name",
             "value": literal("scalar_type/text", "text", json!(format!("p{}", self.next)))},
            {"term": "binding", "name": "level",
             "value": literal("scalar_type/integer", "integer", json!("0"))},
        ]});
        self.node(&label, "value", "expression", "parameter", ty, &[], body);
        label
    }

    /// An `integer_range` type node over `[0, 100]`, created once.
    fn range(&mut self) -> String {
        let label = "bounded_domain/range".to_owned();
        if self.nodes.iter().any(|node| node["node_id"] == id(&label)) {
            return label;
        }
        let bound = |name: &str, value: &str| {
            json!({"term": "binding", "name": name,
                "value": literal("scalar_type/integer", "integer", json!(value))})
        };
        let body =
            json!({"term": "aggregate", "members": [bound("min", "0"), bound("max", "100")]});
        self.node(
            &label,
            "bounded_domain",
            "generated",
            "integer_range",
            "scalar_type/integer",
            &["scalar_type/integer"],
            body,
        );
        label
    }

    fn parameter(&mut self) -> String {
        self.parameter_of("scalar_type/boolean")
    }

    fn literal(&mut self, value: bool) -> String {
        let label = self.fresh("value/literal");
        let body = literal("scalar_type/boolean", "boolean", json!(value));
        self.node(
            &label,
            "value",
            "expression",
            "literal",
            "scalar_type/boolean",
            &[],
            body,
        );
        label
    }

    /// An application of `identity` over references to `arguments`, typed `result`.
    fn application(&mut self, identity: &str, arguments: &[&str], result: &str) -> String {
        let label = self.fresh("expression/application");
        let (form, operator) = if arguments.len() == 1 {
            ("unary", "unary")
        } else {
            ("binary", "binary")
        };
        let body = json!({
            "term": "application",
            "operator": operator,
            "operation": {"identity": identity, "laws": [], "mode": null,
                "member": null, "leaves": []},
            "result_type": id(result),
            "arguments": arguments.iter().map(|argument| reference(argument)).collect::<Vec<_>>(),
        });
        self.node(
            &label,
            "expression",
            "expression",
            form,
            result,
            arguments,
            body,
        );
        label
    }

    fn op(&mut self, identity: &str, arguments: &[&str]) -> String {
        self.application(identity, arguments, "scalar_type/boolean")
    }

    fn reference_node(&mut self, target: &str, ty: &str) -> String {
        let label = self.fresh("expression/reference");
        self.node(
            &label,
            "expression",
            "expression",
            "reference",
            ty,
            &[target],
            reference(target),
        );
        label
    }

    /// An `analysis_claim` whose body is a reference to `target`.
    fn claim(&mut self, target: &str) -> String {
        let label = self.fresh("claim/analysis_claim");
        self.node(
            &label,
            "claim",
            "claim",
            "analysis_claim",
            "scalar_type/boolean",
            &[target],
            reference(target),
        );
        label
    }

    fn region(&self, label: &str) -> (u64, u64) {
        let position = self
            .nodes
            .iter()
            .position(|node| node["node_id"] == id(label))
            .expect("fixture node");
        self.regions[position]
    }

    /// Replaces the body of the node `label` and its dependencies.
    fn set_body(&mut self, label: &str, body: Value, dependencies: &[&str]) {
        let node = self
            .nodes
            .iter_mut()
            .find(|node| node["node_id"] == id(label))
            .expect("fixture node");
        node["body"] = body;
        let mut dependencies = dependencies.to_vec();
        dependencies.sort_unstable();
        node["dependencies"] = json!(dependencies
            .iter()
            .map(|label| id(label))
            .collect::<Vec<_>>());
    }

    fn value(&self) -> Value {
        let source = json!({"authority": "agent-ix", "identity": "quire.fixture.source/v1"});
        let definition = |identity: &str| json!({"authority": "agent-ix", "identity": identity});
        let source_map = self
            .nodes
            .iter()
            .zip(&self.regions)
            .map(|(node, (start, end))| {
                json!({
                    "node_id": node["node_id"],
                    "role": node["occurrences"][0]["role"],
                    "ordinal": 0,
                    "regions": [{"source": source, "start": start, "end": end}],
                })
            })
            .collect::<Vec<_>>();
        json!({
            "contract_version": "quire.checked-package/v2",
            "name": self.name,
            "lock": {
                "sources": [source],
                "edition": {"role": "edition", "definition": definition("quire.fixture.edition/v1")},
                "profile_selections": [],
                "definition_selections": [],
                "required_features": [FEATURE],
                "dependency_selections": [],
            },
            "semantic_graph": {
                "graph_version": "quire.checked-semantic-graph/v2",
                "nodes": self.nodes,
                "operand_rows": self.nodes.iter().map(|_| json!([])).collect::<Vec<_>>(),
            },
            "source_map": source_map,
            "capability_report": [{"feature": FEATURE, "disposition": "available"}],
            "diagnostics": {"catalog": definition("quire.fixture.diagnostics-catalog/v1"), "entries": []},
        })
    }

    /// The document with the fixture package name replaced by this document's name.
    fn document_bytes(&self) -> Vec<u8> {
        let text = serde_json::to_string(&self.value()).expect("document text");
        let named = serde_json::to_string(&self.name).expect("name text");
        text.replace(&format!("\"{PACKAGE}\""), &named).into_bytes()
    }

    fn checked(&self) -> CheckedPackageV2 {
        let mut evidence = CheckedPackageEvidence::new();
        evidence.support_feature(FEATURE);
        let bytes = self.document_bytes();
        match CheckedPackageV2::read(&bytes, CheckedPackageReadLimits::bounded(), &evidence) {
            CheckedPackageV2ReadResult::Admitted(package) => *package,
            other => panic!("checked package refused: {other:?}"),
        }
    }

    /// The canonical bytes of the contract package lowered from `requested`.
    fn bytes(&self, requested: &[&str]) -> Vec<u8> {
        let checked = self.checked();
        let keys: Vec<CheckedNodeId> = requested
            .iter()
            .map(|label| {
                let text = id(label).to_string();
                let named = serde_json::to_string(&self.name).expect("name text");
                let text = text.replace(&format!("\"{PACKAGE}\""), &named);
                serde_json::from_str(&text).expect("node id")
            })
            .collect();
        let profile = CompleteLoweringProfileV2 {
            reference: LoweringProfileRef {
                authority: "quire-analyze".into(),
                identity: "run-package-test".into(),
            },
            required_source_selections: Vec::new(),
            supported_tags: CheckedNodeTag::ALL.iter().copied().collect(),
            require_bounds: true,
            work_limit: u64::MAX,
        };
        let result = checked.lower(&keys, &profile);
        for record in &result.records {
            assert!(
                matches!(record, CompleteLoweringRecordV2::Lowered { .. }),
                "lowering refused: {record:?}"
            );
        }
        result
            .package
            .canonical_bytes()
            .expect("canonical contract package bytes")
            .to_vec()
    }

    /// The admitted contract package over the claims `requested`.
    fn package(&self, requested: &[&str]) -> ContractPackageV1 {
        let bytes = self.bytes(requested);
        match ContractPackageWireV1::read(
            &bytes,
            CheckedPackageReadLimits::bounded(),
            &Default::default(),
        ) {
            ContractPackageReadResult::Admitted(package) => *package,
            other => panic!("contract package refused: {other:?}"),
        }
    }
}

/// The position of the node labelled `label` in the admitted package's graph.
fn index(package: &ContractPackageV1, label: &str) -> usize {
    let key: CheckedNodeId = serde_json::from_value(id(label)).expect("node id");
    package
        .wire()
        .semantic_graph
        .nodes
        .iter()
        .position(|node| node.node_id == key)
        .expect("node in graph")
}

fn translate(document: &Document, claims: &[&str]) -> (ContractPackageV1, AnalysisTranslation) {
    let package = document.package(claims);
    let translation = translate_analysis_claims(&package).expect("translates");
    (package, translation)
}

fn refusal(document: &Document, claims: &[&str]) -> (ContractPackageV1, TranslationError) {
    let package = document.package(claims);
    let error = translate_analysis_claims(&package).expect_err("refused");
    (package, error)
}

/// The expression of the clause at `position` of the one requirement.
fn expression(
    translation: &AnalysisTranslation,
    position: usize,
) -> &quire_contract_model::Expression {
    translation.package().requirements()[0].clauses()[position]
        .body()
        .expression()
}

fn symbol(package: &ContractPackageV1, label: &str) -> String {
    format!("n{}", index(package, label))
}

fn value_reference(package: &ContractPackageV1, label: &str) -> ExpressionKind {
    ExpressionKind::ValueReference {
        name: quire_contract_model::SymbolName::new(symbol(package, label)).expect("symbol"),
        observation: StateObservation::Current,
    }
}

#[trace("FR-009-AC-20")]
#[test]
fn claims_become_clauses_of_one_requirement_in_node_order() {
    let mut document = Document::new();
    let p = document.parameter();
    let q = document.parameter();
    let first = document.op(AND, &[&p, &q]);
    let second = document.op(OR, &[&p, &q]);
    let claim_a = document.claim(&first);
    let claim_b = document.claim(&second);
    let (package, translation) = translate(&document, &[&claim_b, &claim_a]);

    let contract = translation.package();
    assert_eq!(contract.requirements().len(), 1);
    let requirement = &contract.requirements()[0];
    assert_eq!(requirement.id().as_str(), "analysis");
    assert_eq!(contract.id().as_str(), PACKAGE);
    let expected = {
        let mut indexes = [index(&package, &claim_a), index(&package, &claim_b)];
        indexes.sort_unstable();
        indexes
    };
    let ids = requirement
        .clauses()
        .iter()
        .map(|clause| clause.id().to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        expected.map(|node| format!("claim-{node}")).to_vec(),
        "clauses follow graph order, not request order"
    );
    let reference = contract.requirement_ref(requirement);
    assert_eq!(reference.revision().get(), 1);
    for clause in requirement.clauses() {
        assert!(matches!(
            clause.anchor(),
            Some(ExecutionPoint::Handler { name }) if name.as_str() == "analysis"
        ));
    }
    assert_eq!(translation.environment().owner(), &reference);
    assert_eq!(translation.request().statement_count(), 2);
}

#[trace("FR-009-AC-20")]
#[test]
fn a_package_name_the_analysis_api_refuses_is_named() {
    let mut document = Document::new();
    document.name = "bad name".to_owned();
    let p = document.parameter();
    let claim = document.claim(&p);
    let package = document.package(&[&claim]);
    let error = translate_analysis_claims(&package).expect_err("package name refused");
    assert_eq!(error.kind(), TranslationErrorKind::PackageName);
    assert!(error.message().contains("bad name"), "{error}");
}

#[trace("FR-009-AC-21")]
#[test]
fn each_boolean_operator_translates_to_its_expression_kind() {
    let binary = [
        (AND, Some(BooleanOperator::TotalAnd), None),
        (OR, Some(BooleanOperator::TotalOr), None),
        (IMPLIES, Some(BooleanOperator::Implication), None),
        (EQ, None, Some(ComparisonOperator::Equal)),
        (NE, None, Some(ComparisonOperator::NotEqual)),
    ];
    for (identity, boolean, compare) in binary {
        let mut document = Document::new();
        let p = document.parameter();
        let q = document.parameter();
        let application = document.op(identity, &[&p, &q]);
        let claim = document.claim(&application);
        let (package, translation) = translate(&document, &[&claim]);
        let (left, right) = (value_reference(&package, &p), value_reference(&package, &q));
        match expression(&translation, 0).kind() {
            ExpressionKind::Boolean {
                operator,
                left: l,
                right: r,
            } => {
                assert_eq!(Some(*operator), boolean, "{identity}");
                assert_eq!((l.kind(), r.kind()), (&left, &right), "{identity}");
            }
            ExpressionKind::Compare {
                operator,
                left: l,
                right: r,
            } => {
                assert_eq!(Some(*operator), compare, "{identity}");
                assert_eq!((l.kind(), r.kind()), (&left, &right), "{identity}");
            }
            other => panic!("{identity} translated to {other:?}"),
        }
    }

    let mut document = Document::new();
    let p = document.parameter();
    let not = document.op(NOT, &[&p]);
    let claim = document.claim(&not);
    let (package, translation) = translate(&document, &[&claim]);
    match expression(&translation, 0).kind() {
        ExpressionKind::BooleanNot { operand } => {
            assert_eq!(operand.kind(), &value_reference(&package, &p));
        }
        other => panic!("not translated to {other:?}"),
    }
}

#[trace("FR-009-AC-21")]
#[test]
fn a_boolean_literal_and_a_reference_chain_translate_to_the_target_body() {
    let mut document = Document::new();
    let truth = document.literal(true);
    let claim = document.claim(&truth);
    let (_, translation) = translate(&document, &[&claim]);
    assert_eq!(
        expression(&translation, 0).kind(),
        &ExpressionKind::BooleanLiteral { value: true }
    );

    let mut document = Document::new();
    let p = document.parameter();
    let not = document.op(NOT, &[&p]);
    let hop = document.reference_node(&not, "scalar_type/boolean");
    let hop_again = document.reference_node(&hop, "scalar_type/boolean");
    let claim = document.claim(&hop_again);
    let (package, translation) = translate(&document, &[&claim]);
    match expression(&translation, 0).kind() {
        ExpressionKind::BooleanNot { operand } => {
            assert_eq!(operand.kind(), &value_reference(&package, &p));
        }
        other => panic!("reference chain translated to {other:?}"),
    }
}

#[trace("FR-009-AC-22")]
#[test]
fn a_parameter_reached_twice_is_one_declaration_and_one_symbol() {
    let mut document = Document::new();
    let p = document.parameter();
    let q = document.parameter();
    let first = document.op(NOT, &[&p]);
    let second = document.op(AND, &[&p, &q]);
    let claim_a = document.claim(&first);
    let claim_b = document.claim(&second);
    let _unused = document.parameter();
    let (package, translation) = translate(&document, &[&claim_a, &claim_b]);

    let declarations = translation.environment().values();
    let mut expected = [index(&package, &p), index(&package, &q)];
    expected.sort_unstable();
    let names = declarations
        .iter()
        .map(|declaration| declaration.name().as_str().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(names, expected.map(|node| format!("n{node}")).to_vec());
    for declaration in declarations {
        assert_eq!(declaration.kind(), ValueDeclarationKind::Input);
        assert_eq!(declaration.value_type(), &ValueType::Boolean);
    }
    let query = lower_analysis_request(translation.request()).expect("lowers");
    assert_eq!(
        query.variables().len(),
        2,
        "one symbol per distinct parameter"
    );
}

#[trace("FR-009-AC-24")]
#[test]
fn spans_come_from_the_first_region_of_the_first_occurrence() {
    let mut document = Document::new();
    let p = document.parameter();
    let not = document.op(NOT, &[&p]);
    let claim = document.claim(&not);
    let (package, translation) = translate(&document, &[&claim]);

    let check = |span: &quire_contract_model::SourceSpan, label: &str| {
        let (start, end) = document.region(label);
        assert_eq!((span.start().line(), span.start().column()), (1, 1));
        assert_eq!((span.end().line(), span.end().column()), (1, 1));
        assert_eq!(span.start().byte_offset(), start);
        assert_eq!(span.end().byte_offset(), end);
        assert_eq!(span.source().document().as_str(), "contract-package");
        assert_eq!(span.source().revision().get(), 1);
    };
    let requirement = &translation.package().requirements()[0];
    let clause = &requirement.clauses()[0];
    check(clause.source(), &claim);
    let root = expression(&translation, 0);
    check(root.source(), &claim);
    match root.kind() {
        ExpressionKind::BooleanNot { operand } => check(operand.source(), &claim),
        other => panic!("{other:?}"),
    }
    check(translation.environment().values()[0].source(), &p);
    assert_eq!(
        translation.environment().values()[0].name().as_str(),
        symbol(&package, &p)
    );
    let start = requirement.source().start();
    assert_eq!(
        (
            start.byte_offset(),
            requirement.source().end().byte_offset()
        ),
        (0, 0)
    );
}

#[trace("FR-009-AC-26")]
#[test]
fn two_translations_of_the_same_bytes_agree() {
    let mut document = Document::new();
    let p = document.parameter();
    let q = document.parameter();
    let and = document.op(AND, &[&p, &q]);
    let claim_a = document.claim(&and);
    let claim_b = document.claim(&p);
    let bytes = document.bytes(&[&claim_a, &claim_b]);
    let read = || match ContractPackageWireV1::read(
        &bytes,
        CheckedPackageReadLimits::bounded(),
        &Default::default(),
    ) {
        ContractPackageReadResult::Admitted(package) => *package,
        other => panic!("{other:?}"),
    };
    let first = translate_analysis_claims(&read()).expect("first");
    let second = translate_analysis_claims(&read()).expect("second");
    assert_eq!(first.package(), second.package());
    assert_eq!(first.environment(), second.environment());
}

#[trace("FR-009-AC-6", "FR-009-AC-23")]
#[test]
fn a_package_without_a_claim_or_with_a_non_boolean_claim_is_distinguished() {
    let mut document = Document::new();
    let p = document.parameter();
    let package = document.package(&[&p]);
    let error = translate_analysis_claims(&package).expect_err("no claim");
    assert_eq!(error.kind(), TranslationErrorKind::NoAnalysableClause);

    let mut document = Document::new();
    let range = document.range();
    let number = document.parameter_of(&range);
    let claim = document.claim(&number);
    let (package, error) = refusal(&document, &[&claim]);
    assert_eq!(error.kind(), TranslationErrorKind::NonBooleanClaim);
    assert_eq!(error.node(), Some(index(&package, &claim)));
}

fn assert_refused_at(
    package: &ContractPackageV1,
    error: &TranslationError,
    kind: TranslationErrorKind,
    label: &str,
) {
    let node = index(package, label);
    assert_eq!(error.kind(), kind, "{error}");
    assert_eq!(error.node(), Some(node), "{error}");
    assert_eq!(error.path(), format!("semantic_graph.nodes[{node}]"));
    assert!(error
        .to_string()
        .contains(&format!("semantic_graph.nodes[{node}]")));
}

#[trace("FR-009-AC-19", "FR-009-AC-23")]
#[test]
fn an_integer_operation_is_refused_at_its_node() {
    let mut document = Document::new();
    let range = document.range();
    let a = document.parameter_of(&range);
    let b = document.parameter_of(&range);
    let sum = document.application("quire.op.integer.add", &[&a, &b], &range);
    let less = document.application("quire.op.integer.lt", &[&sum, &b], "scalar_type/boolean");
    let claim = document.claim(&less);
    let (package, error) = refusal(&document, &[&claim]);
    assert_refused_at(&package, &error, TranslationErrorKind::CannotLower, &less);
}

/// `not` applied `depth` times over a Boolean parameter; returns the claim and the
/// innermost `not`, whose `ValueReference` operand sits at tree depth `depth + 1`.
fn not_chain(document: &mut Document, depth: usize) -> (String, String) {
    let p = document.parameter();
    let mut innermost = document.op(NOT, &[&p]);
    let mut top = innermost.clone();
    for _ in 1..depth {
        top = document.op(NOT, &[&top]);
    }
    if depth == 0 {
        innermost = p.clone();
        top = p;
    }
    (document.claim(&top), innermost)
}

#[trace("FR-009-AC-23")]
#[test]
fn the_depth_bound_is_128_against_129() {
    let mut document = Document::new();
    let (claim, _) = not_chain(&mut document, 127);
    translate(&document, &[&claim]);

    let mut document = Document::new();
    let (claim, innermost) = not_chain(&mut document, 128);
    let (package, error) = refusal(&document, &[&claim]);
    assert_refused_at(
        &package,
        &error,
        TranslationErrorKind::ExpressionDepthBound,
        &innermost,
    );
}

/// Shared `and` chains: `chains[k]` expands to `2^(k+1) - 1` `Expression` nodes.
struct Sized {
    parameter: String,
    chains: Vec<String>,
}

impl Sized {
    fn new(document: &mut Document) -> Self {
        let parameter = document.parameter();
        Self {
            chains: vec![parameter.clone()],
            parameter,
        }
    }

    /// A node whose translation has exactly `size` `Expression` nodes.
    fn node(&mut self, document: &mut Document, size: usize) -> String {
        if size == 1 {
            return self.parameter.clone();
        }
        if size.is_multiple_of(2) {
            let inner = self.node(document, size - 1);
            return document.op(NOT, &[&inner]);
        }
        let mut level = 0;
        while (1_usize << (level + 2)) - 1 < size {
            level += 1;
        }
        while self.chains.len() <= level {
            let last = self.chains[self.chains.len() - 1].clone();
            self.chains.push(document.op(AND, &[&last, &last]));
        }
        let largest = (1_usize << (level + 1)) - 1;
        let rest = self.node(document, size - 1 - largest);
        let chain = self.chains[level].clone();
        document.op(AND, &[&chain, &rest])
    }
}

#[trace("FR-009-AC-23")]
#[test]
fn the_node_bound_is_10000_against_10001() {
    let mut document = Document::new();
    let mut sized = Sized::new(&mut document);
    let body = sized.node(&mut document, 9_998);
    let top = document.op(AND, &[&body, &sized.parameter]);
    let claim = document.claim(&top);
    translate(&document, &[&claim]);

    let mut document = Document::new();
    let mut sized = Sized::new(&mut document);
    let body = sized.node(&mut document, 9_998);
    let last = document.op(NOT, &[&sized.parameter]);
    let top = document.op(AND, &[&body, &last]);
    let claim = document.claim(&top);
    let (package, error) = refusal(&document, &[&claim]);
    assert_refused_at(
        &package,
        &error,
        TranslationErrorKind::ExpressionNodeBound,
        &last,
    );
}

#[trace("FR-009-AC-23")]
#[test]
fn a_shared_subterm_chain_of_61_nodes_is_refused_without_expanding_it() {
    let mut document = Document::new();
    let p = document.parameter();
    let mut top = document.op(AND, &[&p, &p]);
    for _ in 1..61 {
        top = document.op(AND, &[&top, &top]);
    }
    let claim = document.claim(&top);
    let package = document.package(&[&claim]);
    let started = std::time::Instant::now();
    let error = translate_analysis_claims(&package).expect_err("2^61 expansion refused");
    assert_eq!(
        error.kind(),
        TranslationErrorKind::ExpressionNodeBound,
        "{error}"
    );
    assert!(error.node().is_some());
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
}

#[trace("FR-009-AC-23")]
#[test]
fn a_reference_to_another_claim_is_refused_at_the_claim() {
    let mut document = Document::new();
    let p = document.parameter();
    let inner = document.claim(&p);
    let outer = document.claim(&inner);
    let (package, error) = refusal(&document, &[&outer]);
    assert_refused_at(&package, &error, TranslationErrorKind::CannotLower, &inner);
}

#[trace("FR-009-AC-23")]
#[test]
fn a_reference_cycle_is_refused() {
    let mut document = Document::new();
    let p = document.parameter();
    let first = document.op(AND, &[&p, &p]);
    let second = document.op(AND, &[&first, &p]);
    // `first = and(second, p)` closes the loop `first -> second -> first`.
    document.set_body(
        &first,
        json!({
            "term": "application",
            "operator": "binary",
            "operation": {"identity": AND, "laws": [], "mode": null, "member": null, "leaves": []},
            "result_type": id("scalar_type/boolean"),
            "arguments": [reference(&second), reference(&p)],
        }),
        &[&second, &p],
    );
    for label in [&first, &second] {
        let node = document
            .nodes
            .iter_mut()
            .find(|node| node["node_id"] == id(label))
            .expect("node");
        node["recursion_group"] = json!("loop");
    }
    let claim = document.claim(&second);
    let (package, error) = refusal(&document, &[&claim]);
    assert_eq!(error.kind(), TranslationErrorKind::CannotLower, "{error}");
    assert!(error.message().contains("cycle"), "{error}");
    let named = error.node().expect("a node");
    assert!([index(&package, &first), index(&package, &second)].contains(&named));
}
