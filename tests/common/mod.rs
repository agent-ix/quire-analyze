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
