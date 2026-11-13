//! Test-time contract-package fixtures for the FR-009 tests (FR-009-AC-27).
//!
//! Every document is built here through the public path: a `quire.checked-package/v2` document,
//! `CheckedPackageV2::read`, `CheckedPackageV2::lower` with `CompleteLoweringProfileV2` and the
//! canonical bytes of the resulting `CompleteContractPackageV2`. Nothing is committed.
#![allow(dead_code)]

use quire_analyze::{translate_analysis_claims, AnalysisTranslation, TranslationError};
use quire_contract_model::{
    CheckedNodeId, CheckedNodeTag, CheckedPackageEvidence, CheckedPackageReadLimits,
    CheckedPackageV2, CheckedPackageV2ReadResult, CompleteLoweringProfileV2,
    CompleteLoweringRecordV2, ContractPackageReadResult, ContractPackageV1, ContractPackageWireV1,
    ExpressionKind, LoweringProfileRef, StateObservation,
};
use serde_json::{json, Value};

pub const PACKAGE: &str = "test/pkg";
pub const FEATURE: &str = "quire.value.complete/v1";
pub const AND: &str = "quire.op.boolean.and";
pub const OR: &str = "quire.op.boolean.or";
pub const NOT: &str = "quire.op.boolean.not";
pub const IMPLIES: &str = "quire.op.boolean.implies";
pub const EQ: &str = "quire.op.boolean.eq";
pub const NE: &str = "quire.op.boolean.ne";

pub fn id(label: &str) -> Value {
    json!({"package": PACKAGE, "path": [label], "ordinal": 0})
}

pub fn reference(label: &str) -> Value {
    json!({"term": "reference", "target": id(label)})
}

pub fn literal(ty: &str, kind: &str, value: Value) -> Value {
    json!({"term": "literal", "type": id(ty), "value_kind": kind, "value": value})
}

/// A checked-package document under construction, in node order.
pub struct Document {
    pub name: String,
    pub nodes: Vec<Value>,
    pub regions: Vec<(u64, u64)>,
    pub next: usize,
}

impl Document {
    pub fn new() -> Self {
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

    pub fn node(
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

    pub fn fresh(&mut self, prefix: &str) -> String {
        self.next += 1;
        format!("{prefix}/{}", self.next)
    }

    /// A parameter of the type node `scalar_type/{ty}`.
    pub fn parameter_of(&mut self, ty: &str) -> String {
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
    pub fn range(&mut self) -> String {
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

    /// An `integer_range` type node over `[min, max]`, created once under `label`.
    pub fn range_between(&mut self, label: &str, min: i64, max: i64) -> String {
        let label = format!("bounded_domain/{label}");
        if self.nodes.iter().any(|node| node["node_id"] == id(&label)) {
            return label;
        }
        let bound = |name: &str, value: i64| {
            json!({"term": "binding", "name": name,
                "value": literal("scalar_type/integer", "integer", json!(value.to_string()))})
        };
        let body = json!({"term": "aggregate", "members": [bound("min", min), bound("max", max)]});
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

    /// An integer literal of the type node `ty`.
    pub fn int_literal(&mut self, ty: &str, value: i64) -> String {
        let label = self.fresh("value/literal");
        let body = literal(ty, "integer", json!(value.to_string()));
        self.node(&label, "value", "expression", "literal", ty, &[], body);
        label
    }

    /// An integer application of `identity` over `arguments`, typed `ty`.
    pub fn int_op(&mut self, identity: &str, arguments: &[&str], ty: &str) -> String {
        self.application(identity, arguments, ty)
    }

    pub fn parameter(&mut self) -> String {
        self.parameter_of("scalar_type/boolean")
    }

    pub fn literal(&mut self, value: bool) -> String {
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
    pub fn application(&mut self, identity: &str, arguments: &[&str], result: &str) -> String {
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

    /// An enum `scalar_type` named `name` (also its node label), with its member nodes
    /// `{name}/{member}`. An unordered enum lists its members sorted.
    pub fn enum_type(&mut self, name: &str, members: &[&str]) -> String {
        let mut sorted = members.to_vec();
        sorted.sort_unstable();
        let body = json!({"term": "enum", "ordered": false, "members": sorted});
        self.node(name, "scalar_type", "declaration", "enum", name, &[], body);
        let node = self.nodes.last_mut().expect("enum node");
        node["declaration"] = json!({"qualified_name": [name]});
        node["owner"] = json!({"kind": "source", "authority": "agent-ix",
            "identity": "quire.fixture.source/v1"});
        for member in sorted {
            let label = format!("{name}/{member}");
            let body = literal(name, "enum", json!(member));
            self.node(
                &label,
                "value",
                "expression",
                "enum_value",
                name,
                &[name],
                body,
            );
        }
        name.to_owned()
    }

    /// A `composite_type/record` named `name` whose fields are `(field, type label)` pairs.
    pub fn record_type(&mut self, name: &str, fields: &[(&str, &str)]) -> String {
        let label = format!("composite_type/{name}");
        let members = fields
            .iter()
            .map(|(field, ty)| json!({"term": "binding", "name": field, "value": reference(ty)}))
            .collect::<Vec<_>>();
        let dependencies = fields.iter().map(|(_, ty)| *ty).collect::<Vec<_>>();
        let body = json!({"term": "aggregate", "members": members});
        self.node(
            &label,
            "composite_type",
            "type",
            "record",
            &label,
            &dependencies,
            body,
        );
        label
    }

    /// A `composite_type/option` named `name` over the type `inner`.
    pub fn option_type(&mut self, name: &str, inner: &str) -> String {
        let label = format!("composite_type/{name}");
        let body = json!({"term": "aggregate", "members": [reference(inner)]});
        self.node(
            &label,
            "composite_type",
            "type",
            "option",
            &label,
            &[inner],
            body,
        );
        label
    }

    /// A bounded type the translation does not support, of the `kind` `text`, `rational` or
    /// `sequence`, so that the lowering of the package accepts it.
    pub fn unsupported_type(&mut self, kind: &str) -> String {
        let bounds = json!({"term": "aggregate", "members": [
            {"term": "binding", "name": "min",
                "value": literal("scalar_type/integer", "integer", json!("0"))},
            {"term": "binding", "name": "max",
                "value": literal("scalar_type/integer", "integer", json!("8"))},
        ]});
        let empty = json!({"term": "aggregate", "members": []});
        let profile = json!({"term": "binding", "name": "text_profile",
            "value": literal("scalar_type/text", "text", json!("nfc"))});
        let (base, domain, form, body) = match kind {
            "text" => {
                let base = "scalar_type/text/profiled";
                let profiled = json!({"term": "aggregate", "members": [profile.clone()]});
                self.node(base, "scalar_type", "type", "text", base, &[], profiled);
                let mut body = bounds;
                body["members"]
                    .as_array_mut()
                    .expect("members")
                    .push(profile);
                (base.to_owned(), "bounded_domain/text", "text_bounds", body)
            }
            "rational" => {
                let base = "scalar_type/rational";
                self.node(
                    base,
                    "scalar_type",
                    "type",
                    "rational",
                    base,
                    &[],
                    empty.clone(),
                );
                (
                    base.to_owned(),
                    "bounded_domain/rational",
                    "rational_range",
                    empty,
                )
            }
            _ => {
                let element = self.range_between("element", 0, 3);
                let base = "composite_type/sequence";
                let items = json!({"term": "aggregate", "members": [reference(&element)]});
                self.node(
                    base,
                    "composite_type",
                    "type",
                    "sequence",
                    base,
                    &[&element],
                    items,
                );
                (
                    base.to_owned(),
                    "bounded_domain/sequence",
                    "collection_bounds",
                    bounds,
                )
            }
        };
        self.node(
            domain,
            "bounded_domain",
            "generated",
            form,
            &base,
            &[&base],
            body,
        );
        domain.to_owned()
    }

    /// An enum literal of the enum type `ty`.
    pub fn enum_literal(&mut self, ty: &str, variant: &str) -> String {
        let label = self.fresh("value/literal");
        let body = literal(ty, "enum", json!(variant));
        self.node(&label, "value", "expression", "literal", ty, &[], body);
        label
    }

    /// The `none` literal of the option type `ty`.
    pub fn none_literal(&mut self, ty: &str) -> String {
        let label = self.fresh("value/literal");
        let body = literal(ty, "none", Value::Null);
        self.node(&label, "value", "expression", "literal", ty, &[], body);
        label
    }

    /// An application of `identity` with the given operator, form and member.
    pub fn shaped(
        &mut self,
        identity: &str,
        operator: &str,
        form: &str,
        member: Value,
        arguments: &[&str],
        extra: &[&str],
        result: &str,
    ) -> String {
        let label = self.fresh("expression/application");
        let body = json!({
            "term": "application",
            "operator": operator,
            "operation": {"identity": identity, "laws": [], "mode": null,
                "member": member, "leaves": []},
            "result_type": id(result),
            "arguments": arguments.iter().map(|argument| reference(argument)).collect::<Vec<_>>(),
        });
        let dependencies = [arguments, extra].concat();
        self.node(
            &label,
            "expression",
            "expression",
            form,
            result,
            &dependencies,
            body,
        );
        label
    }

    /// `operand.field`, a `quire.op.record.project` over a record of type `record`.
    pub fn project(&mut self, record: &str, field: &str, operand: &str, result: &str) -> String {
        let member = json!({"kind": "field", "declaration": id(record), "name": field});
        self.shaped(
            "quire.op.record.project",
            "query",
            "query",
            member,
            &[operand],
            &[record],
            result,
        )
    }

    /// `quire.op.option.present` of `option`.
    pub fn present(&mut self, option: &str) -> String {
        let result = "scalar_type/boolean";
        self.shaped(
            "quire.op.option.present",
            "present",
            "presence_read",
            Value::Null,
            &[option],
            &[],
            result,
        )
    }

    /// `quire.op.option.value` of `option`, typed `result`.
    pub fn option_value(&mut self, option: &str, result: &str) -> String {
        self.shaped(
            "quire.op.option.value",
            "value",
            "value_read",
            Value::Null,
            &[option],
            &[],
            result,
        )
    }

    pub fn op(&mut self, identity: &str, arguments: &[&str]) -> String {
        self.application(identity, arguments, "scalar_type/boolean")
    }

    pub fn reference_node(&mut self, target: &str, ty: &str) -> String {
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
    pub fn claim(&mut self, target: &str) -> String {
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

    pub fn region(&self, label: &str) -> (u64, u64) {
        let position = self
            .nodes
            .iter()
            .position(|node| node["node_id"] == id(label))
            .expect("fixture node");
        self.regions[position]
    }

    /// Replaces the body of the node `label` and its dependencies.
    pub fn set_body(&mut self, label: &str, body: Value, dependencies: &[&str]) {
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

    pub fn value(&self) -> Value {
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
    pub fn document_bytes(&self) -> Vec<u8> {
        let text = serde_json::to_string(&self.value()).expect("document text");
        let named = serde_json::to_string(&self.name).expect("name text");
        text.replace(&format!("\"{PACKAGE}\""), &named).into_bytes()
    }

    pub fn checked(&self) -> CheckedPackageV2 {
        let mut evidence = CheckedPackageEvidence::new();
        evidence.support_feature(FEATURE);
        let bytes = self.document_bytes();
        match CheckedPackageV2::read(&bytes, CheckedPackageReadLimits::bounded(), &evidence) {
            CheckedPackageV2ReadResult::Admitted(package) => *package,
            other => panic!("checked package refused: {other:?}"),
        }
    }

    /// The canonical bytes of the contract package lowered from `requested`.
    pub fn bytes(&self, requested: &[&str]) -> Vec<u8> {
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
    pub fn package(&self, requested: &[&str]) -> ContractPackageV1 {
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
pub fn index(package: &ContractPackageV1, label: &str) -> usize {
    let key: CheckedNodeId = serde_json::from_value(id(label)).expect("node id");
    package
        .wire()
        .semantic_graph
        .nodes
        .iter()
        .position(|node| node.node_id == key)
        .expect("node in graph")
}

pub fn translate(document: &Document, claims: &[&str]) -> (ContractPackageV1, AnalysisTranslation) {
    let package = document.package(claims);
    let translation = translate_analysis_claims(&package).expect("translates");
    (package, translation)
}

pub fn refusal(document: &Document, claims: &[&str]) -> (ContractPackageV1, TranslationError) {
    let package = document.package(claims);
    let error = translate_analysis_claims(&package).expect_err("refused");
    (package, error)
}

/// The expression of the clause at `position` of the one requirement.
pub fn expression(
    translation: &AnalysisTranslation,
    position: usize,
) -> &quire_contract_model::Expression {
    translation.package().requirements()[0].clauses()[position]
        .body()
        .expression()
}

pub fn symbol(package: &ContractPackageV1, label: &str) -> String {
    format!("n{}", index(package, label))
}

pub fn value_reference(package: &ContractPackageV1, label: &str) -> ExpressionKind {
    ExpressionKind::ValueReference {
        name: quire_contract_model::SymbolName::new(symbol(package, label)).expect("symbol"),
        observation: StateObservation::Current,
    }
}

/// The admitted package read from `bytes`.
pub fn read_package(bytes: &[u8]) -> ContractPackageV1 {
    match ContractPackageWireV1::read(
        bytes,
        CheckedPackageReadLimits::bounded(),
        &Default::default(),
    ) {
        ContractPackageReadResult::Admitted(package) => *package,
        other => panic!("contract package refused: {other:?}"),
    }
}

/// The package bytes with the operation identity of the application `label` replaced, for an
/// operation the fixture builder cannot lower (its catalog laws are not modelled here).
pub fn with_identity(bytes: &[u8], label: &str, identity: &str) -> Vec<u8> {
    let package = read_package(bytes);
    let node = index(&package, label);
    let mut value: Value = serde_json::from_slice(bytes).expect("package json");
    value["semantic_graph"]["nodes"][node]["body"]["operation"]["identity"] = json!(identity);
    serde_json::to_vec(&value).expect("package bytes")
}

/// The package bytes with the graph node `label` edited.
pub fn with_node(bytes: &[u8], label: &str, edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let package = read_package(bytes);
    let node = index(&package, label);
    let mut value: Value = serde_json::from_slice(bytes).expect("package json");
    edit(&mut value["semantic_graph"]["nodes"][node]);
    serde_json::to_vec(&value).expect("package bytes")
}

static TEMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// A fresh temporary directory, canonical and free of symbolic links, removed on drop.
pub struct TempDir(std::path::PathBuf);

impl TempDir {
    pub fn new(label: &str) -> Self {
        let counter = TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let base = std::fs::canonicalize(std::env::temp_dir()).expect("temporary directory");
        let path = base.join(format!(
            "quire-analyze-{label}-{}-{counter}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create temporary directory");
        Self(path)
    }

    pub fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        // A test may have made a directory unwritable; restore it so removal can succeed.
        if let Ok(entries) = std::fs::read_dir(&self.0) {
            for entry in entries.flatten() {
                let _ =
                    std::fs::set_permissions(entry.path(), std::fs::Permissions::from_mode(0o755));
            }
        }
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// `not` applied `depth` times over a Boolean parameter; returns the claim and the
/// innermost `not`, whose `ValueReference` operand sits at tree depth `depth + 1`.
pub fn not_chain(document: &mut Document, depth: usize) -> (String, String) {
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

/// Shared `and` chains: `chains[k]` expands to `2^(k+1) - 1` `Expression` nodes.
pub struct Sized {
    pub parameter: String,
    chains: Vec<String>,
    combine: &'static str,
    wrap: &'static str,
    ty: String,
}

impl Sized {
    pub fn new(document: &mut Document) -> Self {
        let parameter = document.parameter();
        Self {
            chains: vec![parameter.clone()],
            parameter,
            combine: AND,
            wrap: NOT,
            ty: "scalar_type/boolean".to_owned(),
        }
    }

    /// Integer sums over a parameter of the range `ty`: `add` doubles, `negate` adds one node.
    pub fn sums(document: &mut Document, ty: &str) -> Self {
        let parameter = document.parameter_of(ty);
        Self {
            chains: vec![parameter.clone()],
            parameter,
            combine: "quire.op.integer.add",
            wrap: "quire.op.integer.negate",
            ty: ty.to_owned(),
        }
    }

    /// A node whose translation has exactly `size` `Expression` nodes.
    pub fn node(&mut self, document: &mut Document, size: usize) -> String {
        if size == 1 {
            return self.parameter.clone();
        }
        if size.is_multiple_of(2) {
            let inner = self.node(document, size - 1);
            return document.application(self.wrap, &[&inner], &self.ty);
        }
        let mut level = 0;
        while (1_usize << (level + 2)) - 1 < size {
            level += 1;
        }
        while self.chains.len() <= level {
            let last = self.chains[self.chains.len() - 1].clone();
            self.chains
                .push(document.application(self.combine, &[&last, &last], &self.ty));
        }
        let largest = (1_usize << (level + 1)) - 1;
        let rest = self.node(document, size - 1 - largest);
        let chain = self.chains[level].clone();
        document.application(self.combine, &[&chain, &rest], &self.ty)
    }
}

/// A claim over a two-node reference cycle; returns the document, the claim and the two cycle nodes.
pub fn cycle_document() -> (Document, String, [String; 2]) {
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
    (document, claim, [first, second])
}

/// A claim over a two-node cycle of integer sums; returns the document, the claim and the cycle.
pub fn integer_cycle_document() -> (Document, String, [String; 2]) {
    const ADD: &str = "quire.op.integer.add";
    let mut document = Document::new();
    let range = document.range();
    let a = document.parameter_of(&range);
    let zero = document.int_literal(&range, 0);
    let first = document.int_op(ADD, &[&a, &zero], &range);
    let second = document.int_op(ADD, &[&first, &zero], &range);
    document.set_body(
        &first,
        json!({
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
        node["recursion_group"] = json!("loop");
    }
    let compare = document.int_op("quire.op.integer.le", &[&second, &a], "scalar_type/boolean");
    let claim = document.claim(&compare);
    (document, claim, [first, second])
}

/// A claim bounding the product of two parameters; returns the document, the claim and the product.
pub fn product_document() -> (Document, String, String) {
    let mut document = Document::new();
    let range = document.range();
    let a = document.parameter_of(&range);
    let b = document.parameter_of(&range);
    let product = document.int_op("quire.op.integer.mul", &[&a, &b], &range);
    let c = document.parameter_of(&range);
    let compare = document.int_op(
        "quire.op.integer.le",
        &[&product, &c],
        "scalar_type/boolean",
    );
    let claim = document.claim(&compare);
    (document, claim, product)
}
