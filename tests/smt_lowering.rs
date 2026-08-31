//! Requirement-tagged tests for deterministic SMT-LIB2 lowering (issue #7).

use quire_analyze::{
    lower_boolean_statements, type_shape_digest, BindingGroup, BindingMember, LoweringErrorCode,
    StatementInput, CAPABILITY_CONTRACT, CONTRACT_IR_REVISION, MAX_QUERY_STATEMENTS,
    SMTLIB2_PROFILE,
};
use quire_contract_ir::{
    AnchorName, BooleanOperator, Clause, ClauseId, ClauseKind, CollectionType, ComparisonOperator,
    ContractPackage, DeclarationEnvironment, EnumDeclaration, EnumVariantDeclaration,
    ExecutionPoint, Expression, ExpressionKind, IntegerDomain, IntegerType, OverflowPolicy,
    PackageId, QuantifierDomain, QuantifierKind, RationalType, RecordDeclaration,
    RecordFieldDeclaration, Requirement, RequirementId, RequirementRef, RequirementRevision,
    SchemaVersion, SourceDocumentId, SourceIdentity, SourceLocation, SourceRevision, SourceSpan,
    StateObservation, SymbolName, TypeDeclaration, TypedExpression, ValueDeclaration,
    ValueDeclarationKind, ValueType,
};

fn source_identity() -> SourceIdentity {
    SourceIdentity::new(
        SourceDocumentId::new("spec").expect("source id"),
        SourceRevision::new(1).expect("source revision"),
    )
}

fn span_at(offset: u64) -> SourceSpan {
    let source = source_identity();
    SourceSpan::new(
        SourceLocation::new(
            source.clone(),
            1,
            u32::try_from(offset + 1).expect("column"),
            offset,
        )
        .expect("start"),
        SourceLocation::new(
            source,
            1,
            u32::try_from(offset + 2).expect("column"),
            offset + 1,
        )
        .expect("end"),
    )
    .expect("span")
}

fn span() -> SourceSpan {
    span_at(0)
}

fn owner(requirement: &str, revision: u64) -> RequirementRef {
    RequirementRef::new(
        PackageId::new("agent-ix/demo").expect("package"),
        RequirementId::new(requirement).expect("requirement"),
        RequirementRevision::new(revision).expect("revision"),
    )
}

fn point() -> ExecutionPoint {
    ExecutionPoint::Pre {
        operation: AnchorName::new("check").expect("anchor"),
    }
}

fn boolean_environment(requirement: &str, revision: u64, names: &[&str]) -> DeclarationEnvironment {
    let values = names
        .iter()
        .map(|name| {
            ValueDeclaration::new(
                SymbolName::new(*name).expect("symbol"),
                ValueDeclarationKind::Input,
                ValueType::Boolean,
                span(),
            )
        })
        .collect();
    DeclarationEnvironment::new(owner(requirement, revision), vec![], values, vec![])
        .expect("environment")
}

fn boolean_state_environment(
    requirement: &str,
    revision: u64,
    names: &[&str],
) -> DeclarationEnvironment {
    let values = names
        .iter()
        .map(|name| {
            ValueDeclaration::new(
                SymbolName::new(*name).expect("symbol"),
                ValueDeclarationKind::State,
                ValueType::Boolean,
                span(),
            )
        })
        .collect();
    DeclarationEnvironment::new(owner(requirement, revision), vec![], values, vec![])
        .expect("environment")
}

fn value(name: &str) -> Expression {
    observed_value(name, StateObservation::Current)
}

fn observed_value(name: &str, observation: StateObservation) -> Expression {
    Expression::new(
        ExpressionKind::ValueReference {
            name: SymbolName::new(name).expect("symbol"),
            observation,
        },
        span(),
    )
}

fn bool_statement(
    requirement: &str,
    revision: u64,
    clause: &str,
    names: &[&str],
    expression: Expression,
    digest_byte: u8,
) -> StatementInput {
    statement_from_parts(
        clause,
        boolean_environment(requirement, revision, names),
        expression,
        digest_byte,
    )
}

fn statement_from_parts(
    clause_id: &str,
    environment: DeclarationEnvironment,
    expression: Expression,
    source_offset: u8,
) -> StatementInput {
    let package = package_from_parts(
        clause_id,
        &environment,
        expression,
        ClauseKind::Assertion,
        Some(point()),
        source_offset,
    );
    StatementInput::from_clause(
        &package,
        &package.requirements()[0],
        &package.requirements()[0].clauses()[0],
        environment,
    )
    .expect("statement")
}

fn package_from_parts(
    clause_id: &str,
    environment: &DeclarationEnvironment,
    expression: Expression,
    kind: ClauseKind,
    execution_point: Option<ExecutionPoint>,
    source_offset: u8,
) -> ContractPackage<TypedExpression> {
    let validation_point = execution_point.clone().unwrap_or_else(point);
    let checked = environment
        .check_expression(&expression, &ValueType::Boolean, &validation_point, true)
        .expect("typed expression");
    let clause = Clause::new(
        ClauseId::new(clause_id).expect("clause id"),
        kind,
        execution_point,
        span_at(u64::from(source_offset)),
        checked,
    )
    .expect("clause");
    let requirement = Requirement::new(
        environment.owner().package(),
        environment.owner().requirement().clone(),
        environment.owner().revision(),
        span(),
        vec![clause],
    )
    .expect("requirement");
    ContractPackage::new(
        environment.owner().package().clone(),
        SchemaVersion::V1_0,
        source_identity(),
        vec![requirement],
    )
    .expect("package")
}

fn implication(left: &str, right: &str) -> Expression {
    Expression::new(
        ExpressionKind::Boolean {
            operator: BooleanOperator::Implication,
            left: Box::new(value(left)),
            right: Box::new(Expression::new(
                ExpressionKind::BooleanNot {
                    operand: Box::new(value(right)),
                },
                span(),
            )),
        },
        span(),
    )
}

/// FR-002-AC-1/3 and TC-002/TC-004: lowering is byte deterministic and names complete identities.
/// Trace: TC-002, TC-004, TC-010, FR-002-AC-1, FR-002-AC-3, FR-002-AC-5, NFR-001-AC-1
#[test]
fn golden_query_is_order_independent_and_source_mapped() {
    let first = bool_statement(
        "REQ-alpha",
        1,
        "C-one",
        &["ready", "blocked"],
        implication("ready", "blocked"),
        0x11,
    );
    let second = bool_statement("REQ-beta", 2, "C-two", &["enabled"], value("enabled"), 0x22);

    let forward = lower_boolean_statements(&[first.clone(), second.clone()], &[]).expect("lower");
    let reverse = lower_boolean_statements(&[second, first.clone()], &[]).expect("lower reversed");

    assert_eq!(forward.query, reverse.query);
    assert_eq!(
        forward.analysis_request_digest,
        reverse.analysis_request_digest
    );
    assert_eq!(forward.query_digest, reverse.query_digest);
    assert_eq!(forward.assertions, reverse.assertions);
    assert_eq!(forward.query, include_str!("golden/boolean-v1.smt2"));
    assert_eq!(forward.profile, SMTLIB2_PROFILE);
    assert_eq!(forward.logic, "QF_UF");
    assert_eq!(forward.assertions.len(), 2);
    assert!(forward
        .assertions
        .iter()
        .all(|item| item.name.contains("_c")));
    assert_eq!(first.clause(), &forward.assertions[0].clause);
    assert_eq!(first.clause_digest(), forward.assertions[0].clause_digest);
    assert_eq!(first.execution_point(), &point());
    assert_eq!(first.expression(), &implication("ready", "blocked"));
}

/// FR-001-AC-4 and NFR-002-AC-1: unverified clause identities cannot enter a query bundle.
/// Trace: TC-003, TC-010, FR-001-AC-4, NFR-002-AC-1
#[test]
fn statement_input_recomputes_identity_and_rejects_provenance_mismatch() {
    let valid_environment = boolean_environment("REQ-valid", 1, &["ready"]);
    let valid_package = package_from_parts(
        "C-valid",
        &valid_environment,
        value("ready"),
        ClauseKind::Assertion,
        Some(point()),
        1,
    );
    let wrong_owner = boolean_environment("REQ-wrong", 1, &["ready"]);
    let owner_errors = StatementInput::from_clause(
        &valid_package,
        &valid_package.requirements()[0],
        &valid_package.requirements()[0].clauses()[0],
        wrong_owner,
    )
    .expect_err("owner mismatch");
    assert!(owner_errors
        .iter()
        .any(|error| error.path() == "statement.environment.owner"));
    assert!(owner_errors[0].to_string().contains("InvalidStatement"));

    let unrelated_environment = boolean_environment("REQ-unrelated", 1, &["ready"]);
    let unrelated_package = package_from_parts(
        "C-unrelated",
        &unrelated_environment,
        value("ready"),
        ClauseKind::Assertion,
        Some(point()),
        2,
    );
    let membership_errors = StatementInput::from_clause(
        &valid_package,
        &unrelated_package.requirements()[0],
        &unrelated_package.requirements()[0].clauses()[0],
        unrelated_environment,
    )
    .expect_err("package membership");
    assert!(membership_errors
        .iter()
        .any(|error| error.path() == "statement.clause"));

    let information_environment = boolean_environment("REQ-information", 1, &[]);
    let information_package = package_from_parts(
        "C-information",
        &information_environment,
        Expression::new(ExpressionKind::BooleanLiteral { value: true }, span()),
        ClauseKind::Information,
        None,
        3,
    );
    let anchor_errors = StatementInput::from_clause(
        &information_package,
        &information_package.requirements()[0],
        &information_package.requirements()[0].clauses()[0],
        information_environment,
    )
    .expect_err("missing anchor");
    assert!(anchor_errors
        .iter()
        .any(|error| error.path() == "statement.clause.anchor"));
}

/// FR-002-AC-1/2: every exact Boolean v1 operator has a stable concrete encoding.
/// Trace: TC-010, FR-002-AC-1, FR-002-AC-2
#[test]
fn every_supported_boolean_operator_has_an_exact_encoding() {
    let operators = [
        (BooleanOperator::ShortCircuitAnd, "(and"),
        (BooleanOperator::ShortCircuitOr, "(or"),
        (BooleanOperator::TotalAnd, "(and"),
        (BooleanOperator::TotalOr, "(or"),
        (BooleanOperator::Implication, "(=>"),
    ];
    for (index, (operator, expected)) in operators.into_iter().enumerate() {
        let expression = Expression::new(
            ExpressionKind::Boolean {
                operator,
                left: Box::new(value("left")),
                right: Box::new(value("right")),
            },
            span(),
        );
        let statement = bool_statement(
            "REQ-operators",
            1,
            &format!("C-{index}"),
            &["left", "right"],
            expression,
            u8::try_from(index + 1).expect("small fixture"),
        );
        let bundle = lower_boolean_statements(&[statement], &[]).expect("lower operator");
        assert!(
            bundle.query.contains(expected),
            "missing encoding for {operator:?}"
        );
    }

    for (index, (operator, expected)) in [
        (ComparisonOperator::Equal, "(= "),
        (ComparisonOperator::NotEqual, "(not (= "),
    ]
    .into_iter()
    .enumerate()
    {
        let expression = Expression::new(
            ExpressionKind::Compare {
                operator,
                left: Box::new(value("left")),
                right: Box::new(value("right")),
            },
            span(),
        );
        let statement = bool_statement(
            "REQ-comparisons",
            1,
            &format!("C-{index}"),
            &["left", "right"],
            expression,
            u8::try_from(index + 20).expect("small fixture"),
        );
        let bundle = lower_boolean_statements(&[statement], &[]).expect("lower comparison");
        assert!(bundle.query.contains(expected));
    }
}

/// FR-002-AC-2/4 and TC-003: unsupported arithmetic fails before solver execution.
/// Trace: TC-003, TC-010, FR-002-AC-2, FR-002-AC-4
#[test]
fn arithmetic_is_explicitly_unsupported() {
    let integer = IntegerType::new(IntegerDomain::Signed, 0, 10, OverflowPolicy::Reject)
        .expect("integer type");
    let environment = DeclarationEnvironment::new(
        owner("REQ-int", 1),
        vec![],
        vec![ValueDeclaration::new(
            SymbolName::new("count").expect("symbol"),
            ValueDeclarationKind::Input,
            ValueType::integer(integer.clone()),
            span(),
        )],
        vec![],
    )
    .expect("environment");
    let expression = Expression::new(
        ExpressionKind::Compare {
            operator: ComparisonOperator::Greater,
            left: Box::new(value("count")),
            right: Box::new(Expression::new(
                ExpressionKind::IntegerLiteral {
                    value: 0,
                    value_type: integer,
                },
                span(),
            )),
        },
        span(),
    );
    let statement = statement_from_parts("C-int", environment, expression, 0x33);

    let errors = lower_boolean_statements(&[statement], &[]).expect_err("unsupported");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), LoweringErrorCode::UnsupportedConstruct);
    assert!(errors[0].message().contains("comparison"));
}

/// FR-002-AC-2/4 and TC-003: quantification and non-Boolean data fail explicitly.
/// Trace: TC-003, TC-010, FR-002-AC-2, FR-002-AC-4
#[test]
fn quantification_and_data_types_are_explicitly_unsupported() {
    let collection_type = CollectionType::new(ValueType::Boolean, 4).expect("collection type");
    let environment = DeclarationEnvironment::new(
        owner("REQ-quantifier", 1),
        vec![],
        vec![ValueDeclaration::new(
            SymbolName::new("items").expect("symbol"),
            ValueDeclarationKind::Input,
            ValueType::collection(collection_type),
            span(),
        )],
        vec![],
    )
    .expect("environment");
    let quantifier = Expression::new(
        ExpressionKind::Quantifier {
            quantifier: QuantifierKind::ForAll,
            domain: QuantifierDomain::Elements,
            collection: Box::new(value("items")),
            local: SymbolName::new("item").expect("local"),
            local_source: span(),
            predicate: Box::new(Expression::new(
                ExpressionKind::BooleanLiteral { value: true },
                span(),
            )),
        },
        span(),
    );
    let quantified = statement_from_parts("C-quantifier", environment, quantifier, 0x34);

    let text_equality = Expression::new(
        ExpressionKind::Compare {
            operator: ComparisonOperator::Equal,
            left: Box::new(Expression::new(
                ExpressionKind::TextLiteral {
                    value: "left".to_owned(),
                },
                span(),
            )),
            right: Box::new(Expression::new(
                ExpressionKind::TextLiteral {
                    value: "right".to_owned(),
                },
                span(),
            )),
        },
        span(),
    );
    let text = bool_statement("REQ-text", 1, "C-text", &[], text_equality, 0x35);

    let errors = lower_boolean_statements(&[quantified, text], &[]).expect_err("unsupported");
    assert_eq!(errors.len(), 2);
    assert!(errors
        .iter()
        .all(|error| error.code() == LoweringErrorCode::UnsupportedConstruct));
    assert!(errors
        .iter()
        .any(|error| error.message().contains("quantification")));
    assert!(errors.iter().any(|error| error.message().contains("text")));
}

/// FR-001-AC-2 and FR-002-AC-3: reviewed bindings merge exact compatible roots only.
/// Trace: TC-004, TC-010, FR-001-AC-2, FR-002-AC-3
#[test]
fn explicit_binding_merges_two_complete_origins() {
    let first_environment = boolean_environment("REQ-left", 1, &["active"]);
    let second_environment = boolean_environment("REQ-right", 4, &["active"]);
    let name = SymbolName::new("active").expect("symbol");
    let group = BindingGroup::new(
        "system-active",
        vec![
            BindingMember::from_declaration(
                &first_environment,
                &name,
                StateObservation::Current,
                &point(),
            )
            .expect("first member"),
            BindingMember::from_declaration(
                &second_environment,
                &name,
                StateObservation::Current,
                &point(),
            )
            .expect("second member"),
        ],
    )
    .expect("binding");
    let first = statement_from_parts("C-left", first_environment, value("active"), 0x44);
    let second = statement_from_parts("C-right", second_environment, value("active"), 0x55);

    let unbound = lower_boolean_statements(&[first.clone(), second.clone()], &[]).expect("unbound");
    let bound = lower_boolean_statements(&[first, second], &[group]).expect("bound");
    assert_eq!(unbound.variables.len(), 2);
    assert_eq!(bound.variables.len(), 1);
    assert_eq!(bound.variables[0].origins.len(), 2);
    assert_eq!(
        bound.variables[0].binding_group.as_deref(),
        Some("system-active")
    );
}

/// FR-002-AC-3: state observations remain distinct in symbols and binding compatibility.
/// Trace: TC-004, TC-010, FR-001-AC-2, FR-002-AC-3
#[test]
fn state_observations_are_identity_bearing() {
    let environment = boolean_state_environment("REQ-state", 1, &["ready"]);
    let expression = Expression::new(
        ExpressionKind::Boolean {
            operator: BooleanOperator::TotalAnd,
            left: Box::new(observed_value("ready", StateObservation::Current)),
            right: Box::new(observed_value("ready", StateObservation::Pre)),
        },
        span(),
    );
    let statement = statement_from_parts("C-state", environment.clone(), expression, 0x56);
    let bundle = lower_boolean_statements(&[statement], &[]).expect("lower state observations");

    assert_eq!(bundle.variables.len(), 2);
    let origins: Vec<_> = bundle
        .variables
        .iter()
        .flat_map(|variable| variable.origins.iter())
        .collect();
    assert!(origins
        .iter()
        .any(|origin| origin.contains("|state|current|")));
    assert!(origins.iter().any(|origin| origin.contains("|state|pre|")));

    let name = SymbolName::new("ready").expect("symbol");
    let current =
        BindingMember::from_declaration(&environment, &name, StateObservation::Current, &point())
            .expect("current member");
    let pre = BindingMember::from_declaration(&environment, &name, StateObservation::Pre, &point())
        .expect("pre member");
    assert!(BindingGroup::new("mixed-observations", vec![current, pre]).is_err());
}

/// FR-001-AC-2/4: duplicate group identities and unused members fail closed.
/// Trace: TC-003, TC-010, FR-001-AC-2, FR-001-AC-4
#[test]
fn malformed_binding_sets_are_rejected_before_query_generation() {
    let first_environment = boolean_environment("REQ-left", 1, &["active"]);
    let second_environment = boolean_environment("REQ-right", 1, &["active"]);
    let name = SymbolName::new("active").expect("symbol");
    let first_member = BindingMember::from_declaration(
        &first_environment,
        &name,
        StateObservation::Current,
        &point(),
    )
    .expect("first");
    let second_member = BindingMember::from_declaration(
        &second_environment,
        &name,
        StateObservation::Current,
        &point(),
    )
    .expect("second");
    let group = BindingGroup::new(
        "system-active",
        vec![first_member.clone(), second_member.clone()],
    )
    .expect("group");
    assert_eq!(group.id(), "system-active");
    assert_eq!(group.members().len(), 2);
    assert!(!first_member.key().is_empty());
    assert_eq!(
        first_member.type_shape_digest(),
        second_member.type_shape_digest()
    );

    assert!(BindingMember::from_declaration(
        &first_environment,
        &SymbolName::new("missing").expect("symbol"),
        StateObservation::Current,
        &point(),
    )
    .is_err());
    assert!(BindingMember::from_declaration(
        &first_environment,
        &name,
        StateObservation::Pre,
        &point(),
    )
    .is_err());
    assert!(BindingGroup::new(
        "not valid!",
        vec![first_member.clone(), second_member.clone()]
    )
    .is_err());
    assert!(BindingGroup::new("too-small", vec![first_member.clone()]).is_err());
    assert!(BindingGroup::new(
        "duplicate-member",
        vec![first_member.clone(), first_member.clone()]
    )
    .is_err());

    let integer_type =
        IntegerType::new(IntegerDomain::Signed, -1, 1, OverflowPolicy::Reject).expect("integer");
    let integer_environment = DeclarationEnvironment::new(
        owner("REQ-integer", 1),
        vec![],
        vec![ValueDeclaration::new(
            name.clone(),
            ValueDeclarationKind::Input,
            ValueType::integer(integer_type),
            span(),
        )],
        vec![],
    )
    .expect("integer environment");
    let integer_member = BindingMember::from_declaration(
        &integer_environment,
        &name,
        StateObservation::Current,
        &point(),
    )
    .expect("integer member");
    assert!(BindingGroup::new(
        "different-types",
        vec![first_member.clone(), integer_member]
    )
    .is_err());
    let statement = statement_from_parts("C-left", first_environment, value("active"), 0x66);

    let duplicate = lower_boolean_statements(
        std::slice::from_ref(&statement),
        &[group.clone(), group.clone()],
    )
    .expect_err("duplicate group id");
    assert!(duplicate.iter().any(|error| error.path() == "binding.id"));

    let repeated_member = lower_boolean_statements(
        std::slice::from_ref(&statement),
        &[
            group.clone(),
            BindingGroup::new("other-group", vec![first_member, second_member])
                .expect("other group"),
        ],
    )
    .expect_err("member in multiple groups");
    assert!(repeated_member
        .iter()
        .any(|error| error.message().contains("multiple binding groups")));

    let unused = lower_boolean_statements(&[statement], &[group]).expect_err("unused member");
    assert!(unused
        .iter()
        .any(|error| error.message().contains("is not referenced")));
}

/// ADR-0010 and TC-009: type shape ignores owner but changes with complete structure.
/// Trace: TC-002, TC-009, TC-010, FR-001-AC-3
#[test]
fn type_shape_is_owner_independent_and_structural() {
    let left = boolean_environment("REQ-left", 1, &["active"]);
    let right = boolean_environment("REQ-right", 9, &["active"]);
    assert_eq!(
        type_shape_digest(&left, &ValueType::Boolean).expect("left"),
        type_shape_digest(&right, &ValueType::Boolean).expect("right")
    );
    let integer =
        IntegerType::new(IntegerDomain::Unsigned, 0, 1, OverflowPolicy::Reject).expect("integer");
    assert_ne!(
        type_shape_digest(&left, &ValueType::Boolean).expect("bool"),
        type_shape_digest(&left, &ValueType::integer(integer)).expect("integer")
    );

    let saturating = IntegerType::new(IntegerDomain::Signed, -4, 9, OverflowPolicy::Saturate)
        .expect("saturating integer");
    let rejecting = IntegerType::new(IntegerDomain::Signed, -4, 9, OverflowPolicy::Reject)
        .expect("rejecting integer");
    assert_ne!(
        type_shape_digest(&left, &ValueType::integer(saturating)).expect("saturating"),
        type_shape_digest(&left, &ValueType::integer(rejecting)).expect("rejecting")
    );
    let rational = RationalType::new(-3, 7, 5).expect("rational");
    let option = ValueType::option(ValueType::rational(rational));
    let collection =
        ValueType::collection(CollectionType::new(option.clone(), 8).expect("bounded collection"));
    assert_ne!(
        type_shape_digest(&left, &option).expect("option"),
        type_shape_digest(&left, &collection).expect("collection")
    );
}

/// ADR-0010 and TC-002: structural enum and record identities ignore authored declaration order.
/// Trace: TC-002, TC-010, FR-001-AC-3
#[test]
fn named_type_shape_is_declaration_order_independent() {
    fn environment(reverse: bool) -> DeclarationEnvironment {
        let mut variants = vec![
            EnumVariantDeclaration::new(SymbolName::new("red").expect("name"), span()),
            EnumVariantDeclaration::new(SymbolName::new("blue").expect("name"), span()),
        ];
        let mut fields = vec![
            RecordFieldDeclaration::new(
                SymbolName::new("enabled").expect("name"),
                ValueType::Boolean,
                span(),
            ),
            RecordFieldDeclaration::new(
                SymbolName::new("label").expect("name"),
                ValueType::Text,
                span(),
            ),
        ];
        if reverse {
            variants.reverse();
            fields.reverse();
        }
        let enumeration =
            EnumDeclaration::new(SymbolName::new("Color").expect("name"), span(), variants)
                .expect("enum");
        let record = RecordDeclaration::new(SymbolName::new("Item").expect("name"), span(), fields)
            .expect("record");
        DeclarationEnvironment::new(
            owner("REQ-types", 1),
            vec![
                TypeDeclaration::Enum {
                    declaration: enumeration,
                },
                TypeDeclaration::Record {
                    declaration: record,
                },
            ],
            vec![],
            vec![],
        )
        .expect("environment")
    }

    let forward = environment(false);
    let reverse = environment(true);
    for value_type in [
        ValueType::Enum {
            name: SymbolName::new("Color").expect("name"),
        },
        ValueType::Record {
            name: SymbolName::new("Item").expect("name"),
        },
    ] {
        assert_eq!(
            type_shape_digest(&forward, &value_type).expect("forward"),
            type_shape_digest(&reverse, &value_type).expect("reverse")
        );
    }
}

/// FR-001-AC-3/5: clause and request mutations invalidate downstream identities.
/// Trace: TC-002, TC-010, FR-001-AC-3, FR-001-AC-5
#[test]
fn material_input_changes_invalidate_request_and_query_digests() {
    let first = bool_statement("REQ-digest", 1, "C-one", &["ready"], value("ready"), 0x71);
    let changed = bool_statement("REQ-digest", 1, "C-two", &["ready"], value("ready"), 0x72);
    let first = lower_boolean_statements(&[first], &[]).expect("first");
    let changed = lower_boolean_statements(&[changed], &[]).expect("changed");
    assert_ne!(
        first.analysis_request_digest,
        changed.analysis_request_digest
    );
    assert_ne!(first.query_digest, changed.query_digest);
}

/// NFR-001-AC-2/3: the public statement bound rejects work before lowering.
/// Trace: TC-003, TC-010, NFR-001-AC-2, NFR-001-AC-3
#[test]
fn statement_resource_bound_is_enforced() {
    let statement = bool_statement(
        "REQ-bound",
        1,
        "C-one",
        &[],
        Expression::new(ExpressionKind::BooleanLiteral { value: true }, span()),
        0x73,
    );
    let statements = vec![statement; MAX_QUERY_STATEMENTS + 1];
    let errors = lower_boolean_statements(&statements, &[]).expect_err("resource bound");
    assert_eq!(errors[0].code(), LoweringErrorCode::ResourceLimit);

    let empty = lower_boolean_statements(&[], &[]).expect_err("empty request");
    assert_eq!(empty[0].code(), LoweringErrorCode::InvalidStatement);

    let statement = bool_statement(
        "REQ-duplicate",
        1,
        "C-duplicate",
        &[],
        Expression::new(ExpressionKind::BooleanLiteral { value: true }, span()),
        0x74,
    );
    let duplicate = lower_boolean_statements(&[statement.clone(), statement], &[])
        .expect_err("duplicate statement");
    assert_eq!(duplicate[0].code(), LoweringErrorCode::DuplicateStatement);
}

/// FR-002 capability contract and exact dependency pin are public and closed.
/// Trace: TC-010, FR-002-AC-2, FR-002-AC-4, FR-002-AC-5
#[test]
fn capability_contract_is_explicit() {
    assert_eq!(
        CONTRACT_IR_REVISION,
        "bb5d30cbb1519b7ac286250114c96ba967661cba"
    );
    assert_eq!(CAPABILITY_CONTRACT.profile, "quire.smtlib2/v1");
    assert!(CAPABILITY_CONTRACT
        .exact_constructs
        .contains(&"boolean_value_reference"));
    for unsupported in [
        "arithmetic",
        "quantification",
        "record",
        "collection",
        "pure_function_call",
    ] {
        assert!(CAPABILITY_CONTRACT
            .unsupported_constructs
            .contains(&unsupported));
    }

    let manifest = include_str!("../Cargo.toml");
    let lock = include_str!("../Cargo.lock");
    assert!(manifest.contains(&format!("rev = \"{CONTRACT_IR_REVISION}\"")));
    assert!(lock.contains(&format!("#{CONTRACT_IR_REVISION}")));
    assert!(!lock.contains("name = \"z3\""));
    assert!(!lock.contains("name = \"cvc5\""));
}
