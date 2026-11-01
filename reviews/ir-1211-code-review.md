---
id: SR-17011
title: "Code review — IR-1211 trace attribute migration"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-analyze; IR-1211; Cargo.toml, Cargo.lock, deny.toml, tests/adr_0010.rs, tests/analysis_semantics.rs, tests/cli_streams.rs, tests/foundation.rs, tests/hybrid_synthesis.rs, tests/smt_lowering.rs, tests/solver_adapters.rs, src/hybrid_synthesis.rs test module"
review_set: subset
---

## Summary

Ticket: IR-1211. Reviewed frozen head b397d7205784e1cf4f6cbddcec048fd3d8a66620 against 962bbbfcc943cb1f1427d443147a9df005ff15f2 using code-review with the Rust review/style lane. Independent extraction from git base and current source confirms all 44 attached test items and 142 ordered normalized IDs are preserved; removing only trace metadata/imports leaves executable tokens identical.

## Verdict

**PASS** — no code defects found. Merge remains dependent on the lead completing pending focused runtime and cargo-deny validation. This review does not claim those pending checks passed.

The dependency is dev-only, imports are integration-test-local or inside cfg(test), and the inspected dependency implementation returns the annotated item unchanged after ID-shape validation. Bare trace attributes match the static parser convention; proptest retains the same test wrapper and arguments. Lock changes add exactly one package and one root dependency; deny changes allow only that first-party dependency. No vendoring, runtime changes, assertion/name changes, CI weakening, new tracking machinery, or unrelated cleanup was introduced. Existing semantic correctness of all inherited bindings is outside this conversion review; correct below means preserved attachment and IDs.

## Assurance Context

AP-001 (spec/assurance/AP-001-analyze-release.md) applies to the source candidate and names material false-conclusion risks. The change preserves test/evidence attachment and does not alter the analysis/solver boundary. No standing exceptions; no new exception requested. Profile-selected release architecture/measurement/producer-reliance and full structural-gap evidence were not refreshed in this metadata-only review. Installed official solver corpus, Kani/proof evidence, and focused execution/policy results were unavailable at review time. Review independence: reviewer independently compared git source; format/clippy and structural-tool logs were produced by the lead using the shared Rust/Quire toolchain.

## Validation

Independent before/after audit passed: 44 items, 142 ordered IDs, unchanged executable tokens. Existing /tmp/ir1211-fmt.status and clippy.status are zero. Structural trace/matrix evidence was inspected as parser evidence only. Focused runtime and deny sources/licenses are pending behind the shared Cargo resource lock; initial status 124/empty logs represent lock-wait timeouts, not executed-test failures. No Cargo invoked by reviewer. Broad gap/spec sweeps skipped: no runtime production behavior or spec changed; AP-001 release-wide evidence is not certified by this scoped pass. Explicit user no-behavior scope supersedes unrelated digest cleanup instructions.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined units

```yaml
scope:
- id: foundation_keeps_license_publication_and_ci_authority_bounded
  path: tests/foundation.rs
  role: examined
  excerpt: '#[trace("NFR-002-AC-4")]'
- id: foundation_defines_closed_requirements_and_non_conclusive_states
  path: tests/foundation.rs
  role: examined
  excerpt: '#[trace("NFR-002-AC-2")]'
- id: foundation_plan_advances_only_first_unblocked_child
  path: tests/foundation.rs
  role: examined
  excerpt: '#[trace("PLAN-001")]'
- id: golden_query_is_order_independent_and_source_mapped
  path: tests/smt_lowering.rs
  role: examined
  excerpt: '#[trace("TC-002", "TC-004", "TC-010", "FR-002-AC-1", "FR-002-AC-3", "FR-002-AC-5",
    "NFR-001-AC-1")]'
- id: statement_input_recomputes_identity_and_rejects_provenance_mismatch
  path: tests/smt_lowering.rs
  role: examined
  excerpt: '#[trace("TC-003", "TC-010", "FR-001-AC-4", "NFR-002-AC-1")]'
- id: every_supported_boolean_operator_has_an_exact_encoding
  path: tests/smt_lowering.rs
  role: examined
  excerpt: '#[trace("TC-010", "FR-002-AC-1", "FR-002-AC-2")]'
- id: arithmetic_is_explicitly_unsupported
  path: tests/smt_lowering.rs
  role: examined
  excerpt: '#[trace("TC-003", "TC-010", "FR-002-AC-2", "FR-002-AC-4")]'
- id: quantification_and_data_types_are_explicitly_unsupported
  path: tests/smt_lowering.rs
  role: examined
  excerpt: '#[trace("TC-003", "TC-010", "FR-002-AC-2", "FR-002-AC-4")]'
- id: explicit_binding_merges_two_complete_origins
  path: tests/smt_lowering.rs
  role: examined
  excerpt: '#[trace("TC-004", "TC-010", "FR-001-AC-2", "FR-002-AC-3")]'
- id: state_observations_are_identity_bearing
  path: tests/smt_lowering.rs
  role: examined
  excerpt: '#[trace("TC-004", "TC-010", "FR-001-AC-2", "FR-002-AC-3")]'
- id: malformed_binding_sets_are_rejected_before_query_generation
  path: tests/smt_lowering.rs
  role: examined
  excerpt: '#[trace("TC-003", "TC-010", "FR-001-AC-2", "FR-001-AC-4")]'
- id: type_shape_is_owner_independent_and_structural
  path: tests/smt_lowering.rs
  role: examined
  excerpt: '#[trace("TC-002", "TC-009", "TC-010", "FR-001-AC-3")]'
- id: named_type_shape_is_declaration_order_independent
  path: tests/smt_lowering.rs
  role: examined
  excerpt: '#[trace("TC-002", "TC-010", "FR-001-AC-3")]'
- id: material_input_changes_invalidate_request_and_query_digests
  path: tests/smt_lowering.rs
  role: examined
  excerpt: '#[trace("TC-002", "TC-010", "FR-001-AC-3", "FR-001-AC-5")]'
- id: statement_resource_bound_is_enforced
  path: tests/smt_lowering.rs
  role: examined
  excerpt: '#[trace("TC-003", "TC-010", "NFR-001-AC-2", "NFR-001-AC-3")]'
- id: capability_contract_is_explicit
  path: tests/smt_lowering.rs
  role: examined
  excerpt: '#[trace("TC-010", "FR-002-AC-2", "FR-002-AC-4", "FR-002-AC-5")]'
- id: invalid_argv_preserves_exit_and_diagnostic_streams
  path: tests/cli_streams.rs
  role: examined
  excerpt: '#[trace("TC-008", "FR-005-AC-1")]'
- id: dedicated_requests_are_role_exact_deterministic_and_fail_closed
  path: tests/analysis_semantics.rs
  role: examined
  excerpt: '#[trace("TC-001", "TC-003", "FR-001-AC-1", "FR-001-AC-4")]'
- id: all_ten_truth_table_cells_match_independent_finite_models
  path: tests/analysis_semantics.rs
  role: examined
  excerpt: '#[trace("TC-001", "TC-005", "FR-001-AC-1", "FR-004-AC-1", "FR-004-AC-5")]'
- id: model_decode_mapping_and_replay_are_fail_closed
  path: tests/analysis_semantics.rs
  role: examined
  excerpt: '#[trace("TC-004", "TC-006", "TC-007", "FR-004-AC-2", "FR-004-AC-3", "FR-004-AC-4")]'
- id: nonconclusive_adapter_states_remain_nonconclusive
  path: tests/analysis_semantics.rs
  role: examined
  excerpt: '#[trace("TC-005", "TC-007", "FR-004-AC-4", "FR-004-AC-5")]'
- id: differential_disposition_requires_two_verified_conclusive_results
  path: tests/analysis_semantics.rs
  role: examined
  excerpt: '#[trace("TC-006", "FR-005-AC-3", "StR-001-VC-2")]'
- id: official_z3_cvc5_differential_corpus_agrees
  path: tests/analysis_semantics.rs
  role: examined
  excerpt: '#[trace("TC-006", "FR-005-AC-3", "MP-001-M-06")]'
- id: report_bytes_are_canonical_reconstructed_and_published_without_overwrite
  path: tests/analysis_semantics.rs
  role: examined
  excerpt: '#[trace("TC-007", "TC-008", "FR-005-AC-1", "FR-005-AC-2", "FR-005-AC-4")]'
- id: a_forged_conclusion_is_refused_by_re_derivation_from_retained_evidence
  path: tests/analysis_semantics.rs
  role: examined
  excerpt: '#[trace("TC-007", "FR-005-AC-2", "NFR-002-AC-1", "NFR-002-AC-3")]'
- id: differential_v2_requires_retained_identities_and_rejects_v1
  path: tests/analysis_semantics.rs
  role: examined
  excerpt: '#[trace("TC-007", "FR-005-AC-6")]'
- id: candidate_identity_rules_match_the_retained_measurement
  path: tests/adr_0010.rs
  role: examined
  excerpt: '#[trace("TC-009", "FR-001-AC-2")]'
- id: explicit_bindings_reject_incompatible_members
  path: tests/adr_0010.rs
  role: examined
  excerpt: '#[trace("TC-009", "FR-001-AC-2")]'
- id: adr_binds_versions_hashes_and_supersession
  path: tests/adr_0010.rs
  role: examined
  excerpt: '#[trace("TC-009", "FR-001-AC-3", "FR-001-AC-5")]'
- id: exact_engine_contract_and_protocol_states
  path: tests/solver_adapters.rs
  role: examined
  excerpt: '#[trace("TC-005", "TC-006", "FR-003-AC-2", "FR-003-AC-3")]'
- id: paths_are_absolute_pinned_and_not_shell_interpreted
  path: tests/solver_adapters.rs
  role: examined
  excerpt: '#[trace("TC-005", "FR-003-AC-5")]'
- id: executable_identity_probe_fails_closed
  path: tests/solver_adapters.rs
  role: examined
  excerpt: '#[trace("TC-005", "FR-003-AC-2", "FR-003-AC-4")]'
- id: every_adapter_owned_limit_is_finite_and_enforced
  path: tests/solver_adapters.rs
  role: examined
  excerpt: '#[trace("TC-005", "NFR-001-AC-2", "NFR-001-AC-3")]'
- id: hostile_pipe_pressure_remains_bounded
  path: tests/solver_adapters.rs
  role: examined
  excerpt: '#[trace("TC-005", "NFR-001-AC-2", "NFR-001-AC-3")]'
- id: timeout_and_cancellation_cleanup_are_measured
  path: tests/solver_adapters.rs
  role: examined
  excerpt: '#[trace("TC-005", "FR-003-AC-1", "NFR-001-AC-2")]'
- id: a_nonzero_exit_is_not_a_conclusion_for_either_engine
  path: tests/solver_adapters.rs
  role: examined
  excerpt: '#[trace("TC-005", "FR-003-AC-2", "FR-003-AC-4", "NFR-002-AC-2")]'
- id: tc_014_canonicalizes_arbitrary_finite_set_like_inputs
  path: tests/hybrid_synthesis.rs
  role: examined
  excerpt: '#[trace("TC-014", "FR-008-AC-1")]'
- id: tc_013_preserves_mode_guard_reset_and_exact_replay
  path: tests/hybrid_synthesis.rs
  role: examined
  excerpt: '#[trace("TC-013", "FR-007-AC-1", "FR-007-AC-3")]'
- id: tc_013_refuses_malformed_models_and_keeps_bound_exhaustion_nonconclusive
  path: tests/hybrid_synthesis.rs
  role: examined
  excerpt: '#[trace("TC-013", "FR-007-AC-2")]'
- id: tc_014_canonical_candidate_requires_exact_independent_validation
  path: tests/hybrid_synthesis.rs
  role: examined
  excerpt: '#[trace("TC-014", "FR-008-AC-1", "FR-008-AC-2")]'
- id: tc_014_distinguishes_exhaustive_no_candidate_from_incomplete_search
  path: tests/hybrid_synthesis.rs
  role: examined
  excerpt: '#[trace("TC-014", "FR-008-AC-3")]'
- id: tc_014_distinguishes_delimited_atoms_and_stops_at_search_bound
  path: tests/hybrid_synthesis.rs
  role: examined
  excerpt: '#[trace("TC-014", "FR-008-AC-1", "FR-008-AC-3")]'
- id: tc_014_enforces_aggregate_atom_byte_boundary
  path: tests/hybrid_synthesis.rs
  role: examined
  excerpt: '#[trace("TC-014", "FR-008-AC-1")]'
- id: binders_preserve_exact_envelope_and_refuse_wrong_method
  path: src/hybrid_synthesis.rs
  role: examined
  excerpt: '#[trace("TC-013", "TC-014")]'
- id: Cargo.toml
  path: Cargo.toml
  role: examined
  excerpt: ix-trace-rs = { git = "https://github.com/agent-ix/ix-trace-rs", branch
    = "main" }
- id: Cargo.lock
  path: Cargo.lock
  role: examined
  excerpt: ix-trace-rs 0.1.2, e29cc07f70394ff018884d12406c1e150bded5f4; one package
    and root dependency
- id: deny.toml
  path: deny.toml
  role: examined
  excerpt: Targeted ix-trace-rs AGPL-3.0-or-later exception and Git source allow entry
bindings:
- test_id: tests/foundation.rs::foundation_keeps_license_publication_and_ci_authority_bounded
  ac_id: NFR-002-AC-4
  trace: correct
- test_id: tests/foundation.rs::foundation_defines_closed_requirements_and_non_conclusive_states
  ac_id: NFR-002-AC-2
  trace: correct
- test_id: tests/foundation.rs::foundation_plan_advances_only_first_unblocked_child
  ac_id: PLAN-001
  trace: correct
- test_id: tests/smt_lowering.rs::golden_query_is_order_independent_and_source_mapped
  ac_id: TC-002
  trace: correct
- test_id: tests/smt_lowering.rs::golden_query_is_order_independent_and_source_mapped
  ac_id: TC-004
  trace: correct
- test_id: tests/smt_lowering.rs::golden_query_is_order_independent_and_source_mapped
  ac_id: TC-010
  trace: correct
- test_id: tests/smt_lowering.rs::golden_query_is_order_independent_and_source_mapped
  ac_id: FR-002-AC-1
  trace: correct
- test_id: tests/smt_lowering.rs::golden_query_is_order_independent_and_source_mapped
  ac_id: FR-002-AC-3
  trace: correct
- test_id: tests/smt_lowering.rs::golden_query_is_order_independent_and_source_mapped
  ac_id: FR-002-AC-5
  trace: correct
- test_id: tests/smt_lowering.rs::golden_query_is_order_independent_and_source_mapped
  ac_id: NFR-001-AC-1
  trace: correct
- test_id: tests/smt_lowering.rs::statement_input_recomputes_identity_and_rejects_provenance_mismatch
  ac_id: TC-003
  trace: correct
- test_id: tests/smt_lowering.rs::statement_input_recomputes_identity_and_rejects_provenance_mismatch
  ac_id: TC-010
  trace: correct
- test_id: tests/smt_lowering.rs::statement_input_recomputes_identity_and_rejects_provenance_mismatch
  ac_id: FR-001-AC-4
  trace: correct
- test_id: tests/smt_lowering.rs::statement_input_recomputes_identity_and_rejects_provenance_mismatch
  ac_id: NFR-002-AC-1
  trace: correct
- test_id: tests/smt_lowering.rs::every_supported_boolean_operator_has_an_exact_encoding
  ac_id: TC-010
  trace: correct
- test_id: tests/smt_lowering.rs::every_supported_boolean_operator_has_an_exact_encoding
  ac_id: FR-002-AC-1
  trace: correct
- test_id: tests/smt_lowering.rs::every_supported_boolean_operator_has_an_exact_encoding
  ac_id: FR-002-AC-2
  trace: correct
- test_id: tests/smt_lowering.rs::arithmetic_is_explicitly_unsupported
  ac_id: TC-003
  trace: correct
- test_id: tests/smt_lowering.rs::arithmetic_is_explicitly_unsupported
  ac_id: TC-010
  trace: correct
- test_id: tests/smt_lowering.rs::arithmetic_is_explicitly_unsupported
  ac_id: FR-002-AC-2
  trace: correct
- test_id: tests/smt_lowering.rs::arithmetic_is_explicitly_unsupported
  ac_id: FR-002-AC-4
  trace: correct
- test_id: tests/smt_lowering.rs::quantification_and_data_types_are_explicitly_unsupported
  ac_id: TC-003
  trace: correct
- test_id: tests/smt_lowering.rs::quantification_and_data_types_are_explicitly_unsupported
  ac_id: TC-010
  trace: correct
- test_id: tests/smt_lowering.rs::quantification_and_data_types_are_explicitly_unsupported
  ac_id: FR-002-AC-2
  trace: correct
- test_id: tests/smt_lowering.rs::quantification_and_data_types_are_explicitly_unsupported
  ac_id: FR-002-AC-4
  trace: correct
- test_id: tests/smt_lowering.rs::explicit_binding_merges_two_complete_origins
  ac_id: TC-004
  trace: correct
- test_id: tests/smt_lowering.rs::explicit_binding_merges_two_complete_origins
  ac_id: TC-010
  trace: correct
- test_id: tests/smt_lowering.rs::explicit_binding_merges_two_complete_origins
  ac_id: FR-001-AC-2
  trace: correct
- test_id: tests/smt_lowering.rs::explicit_binding_merges_two_complete_origins
  ac_id: FR-002-AC-3
  trace: correct
- test_id: tests/smt_lowering.rs::state_observations_are_identity_bearing
  ac_id: TC-004
  trace: correct
- test_id: tests/smt_lowering.rs::state_observations_are_identity_bearing
  ac_id: TC-010
  trace: correct
- test_id: tests/smt_lowering.rs::state_observations_are_identity_bearing
  ac_id: FR-001-AC-2
  trace: correct
- test_id: tests/smt_lowering.rs::state_observations_are_identity_bearing
  ac_id: FR-002-AC-3
  trace: correct
- test_id: tests/smt_lowering.rs::malformed_binding_sets_are_rejected_before_query_generation
  ac_id: TC-003
  trace: correct
- test_id: tests/smt_lowering.rs::malformed_binding_sets_are_rejected_before_query_generation
  ac_id: TC-010
  trace: correct
- test_id: tests/smt_lowering.rs::malformed_binding_sets_are_rejected_before_query_generation
  ac_id: FR-001-AC-2
  trace: correct
- test_id: tests/smt_lowering.rs::malformed_binding_sets_are_rejected_before_query_generation
  ac_id: FR-001-AC-4
  trace: correct
- test_id: tests/smt_lowering.rs::type_shape_is_owner_independent_and_structural
  ac_id: TC-002
  trace: correct
- test_id: tests/smt_lowering.rs::type_shape_is_owner_independent_and_structural
  ac_id: TC-009
  trace: correct
- test_id: tests/smt_lowering.rs::type_shape_is_owner_independent_and_structural
  ac_id: TC-010
  trace: correct
- test_id: tests/smt_lowering.rs::type_shape_is_owner_independent_and_structural
  ac_id: FR-001-AC-3
  trace: correct
- test_id: tests/smt_lowering.rs::named_type_shape_is_declaration_order_independent
  ac_id: TC-002
  trace: correct
- test_id: tests/smt_lowering.rs::named_type_shape_is_declaration_order_independent
  ac_id: TC-010
  trace: correct
- test_id: tests/smt_lowering.rs::named_type_shape_is_declaration_order_independent
  ac_id: FR-001-AC-3
  trace: correct
- test_id: tests/smt_lowering.rs::material_input_changes_invalidate_request_and_query_digests
  ac_id: TC-002
  trace: correct
- test_id: tests/smt_lowering.rs::material_input_changes_invalidate_request_and_query_digests
  ac_id: TC-010
  trace: correct
- test_id: tests/smt_lowering.rs::material_input_changes_invalidate_request_and_query_digests
  ac_id: FR-001-AC-3
  trace: correct
- test_id: tests/smt_lowering.rs::material_input_changes_invalidate_request_and_query_digests
  ac_id: FR-001-AC-5
  trace: correct
- test_id: tests/smt_lowering.rs::statement_resource_bound_is_enforced
  ac_id: TC-003
  trace: correct
- test_id: tests/smt_lowering.rs::statement_resource_bound_is_enforced
  ac_id: TC-010
  trace: correct
- test_id: tests/smt_lowering.rs::statement_resource_bound_is_enforced
  ac_id: NFR-001-AC-2
  trace: correct
- test_id: tests/smt_lowering.rs::statement_resource_bound_is_enforced
  ac_id: NFR-001-AC-3
  trace: correct
- test_id: tests/smt_lowering.rs::capability_contract_is_explicit
  ac_id: TC-010
  trace: correct
- test_id: tests/smt_lowering.rs::capability_contract_is_explicit
  ac_id: FR-002-AC-2
  trace: correct
- test_id: tests/smt_lowering.rs::capability_contract_is_explicit
  ac_id: FR-002-AC-4
  trace: correct
- test_id: tests/smt_lowering.rs::capability_contract_is_explicit
  ac_id: FR-002-AC-5
  trace: correct
- test_id: tests/cli_streams.rs::invalid_argv_preserves_exit_and_diagnostic_streams
  ac_id: TC-008
  trace: correct
- test_id: tests/cli_streams.rs::invalid_argv_preserves_exit_and_diagnostic_streams
  ac_id: FR-005-AC-1
  trace: correct
- test_id: tests/analysis_semantics.rs::dedicated_requests_are_role_exact_deterministic_and_fail_closed
  ac_id: TC-001
  trace: correct
- test_id: tests/analysis_semantics.rs::dedicated_requests_are_role_exact_deterministic_and_fail_closed
  ac_id: TC-003
  trace: correct
- test_id: tests/analysis_semantics.rs::dedicated_requests_are_role_exact_deterministic_and_fail_closed
  ac_id: FR-001-AC-1
  trace: correct
- test_id: tests/analysis_semantics.rs::dedicated_requests_are_role_exact_deterministic_and_fail_closed
  ac_id: FR-001-AC-4
  trace: correct
- test_id: tests/analysis_semantics.rs::all_ten_truth_table_cells_match_independent_finite_models
  ac_id: TC-001
  trace: correct
- test_id: tests/analysis_semantics.rs::all_ten_truth_table_cells_match_independent_finite_models
  ac_id: TC-005
  trace: correct
- test_id: tests/analysis_semantics.rs::all_ten_truth_table_cells_match_independent_finite_models
  ac_id: FR-001-AC-1
  trace: correct
- test_id: tests/analysis_semantics.rs::all_ten_truth_table_cells_match_independent_finite_models
  ac_id: FR-004-AC-1
  trace: correct
- test_id: tests/analysis_semantics.rs::all_ten_truth_table_cells_match_independent_finite_models
  ac_id: FR-004-AC-5
  trace: correct
- test_id: tests/analysis_semantics.rs::model_decode_mapping_and_replay_are_fail_closed
  ac_id: TC-004
  trace: correct
- test_id: tests/analysis_semantics.rs::model_decode_mapping_and_replay_are_fail_closed
  ac_id: TC-006
  trace: correct
- test_id: tests/analysis_semantics.rs::model_decode_mapping_and_replay_are_fail_closed
  ac_id: TC-007
  trace: correct
- test_id: tests/analysis_semantics.rs::model_decode_mapping_and_replay_are_fail_closed
  ac_id: FR-004-AC-2
  trace: correct
- test_id: tests/analysis_semantics.rs::model_decode_mapping_and_replay_are_fail_closed
  ac_id: FR-004-AC-3
  trace: correct
- test_id: tests/analysis_semantics.rs::model_decode_mapping_and_replay_are_fail_closed
  ac_id: FR-004-AC-4
  trace: correct
- test_id: tests/analysis_semantics.rs::nonconclusive_adapter_states_remain_nonconclusive
  ac_id: TC-005
  trace: correct
- test_id: tests/analysis_semantics.rs::nonconclusive_adapter_states_remain_nonconclusive
  ac_id: TC-007
  trace: correct
- test_id: tests/analysis_semantics.rs::nonconclusive_adapter_states_remain_nonconclusive
  ac_id: FR-004-AC-4
  trace: correct
- test_id: tests/analysis_semantics.rs::nonconclusive_adapter_states_remain_nonconclusive
  ac_id: FR-004-AC-5
  trace: correct
- test_id: tests/analysis_semantics.rs::differential_disposition_requires_two_verified_conclusive_results
  ac_id: TC-006
  trace: correct
- test_id: tests/analysis_semantics.rs::differential_disposition_requires_two_verified_conclusive_results
  ac_id: FR-005-AC-3
  trace: correct
- test_id: tests/analysis_semantics.rs::differential_disposition_requires_two_verified_conclusive_results
  ac_id: StR-001-VC-2
  trace: correct
- test_id: tests/analysis_semantics.rs::official_z3_cvc5_differential_corpus_agrees
  ac_id: TC-006
  trace: correct
- test_id: tests/analysis_semantics.rs::official_z3_cvc5_differential_corpus_agrees
  ac_id: FR-005-AC-3
  trace: correct
- test_id: tests/analysis_semantics.rs::official_z3_cvc5_differential_corpus_agrees
  ac_id: MP-001-M-06
  trace: correct
- test_id: tests/analysis_semantics.rs::report_bytes_are_canonical_reconstructed_and_published_without_overwrite
  ac_id: TC-007
  trace: correct
- test_id: tests/analysis_semantics.rs::report_bytes_are_canonical_reconstructed_and_published_without_overwrite
  ac_id: TC-008
  trace: correct
- test_id: tests/analysis_semantics.rs::report_bytes_are_canonical_reconstructed_and_published_without_overwrite
  ac_id: FR-005-AC-1
  trace: correct
- test_id: tests/analysis_semantics.rs::report_bytes_are_canonical_reconstructed_and_published_without_overwrite
  ac_id: FR-005-AC-2
  trace: correct
- test_id: tests/analysis_semantics.rs::report_bytes_are_canonical_reconstructed_and_published_without_overwrite
  ac_id: FR-005-AC-4
  trace: correct
- test_id: tests/analysis_semantics.rs::a_forged_conclusion_is_refused_by_re_derivation_from_retained_evidence
  ac_id: TC-007
  trace: correct
- test_id: tests/analysis_semantics.rs::a_forged_conclusion_is_refused_by_re_derivation_from_retained_evidence
  ac_id: FR-005-AC-2
  trace: correct
- test_id: tests/analysis_semantics.rs::a_forged_conclusion_is_refused_by_re_derivation_from_retained_evidence
  ac_id: NFR-002-AC-1
  trace: correct
- test_id: tests/analysis_semantics.rs::a_forged_conclusion_is_refused_by_re_derivation_from_retained_evidence
  ac_id: NFR-002-AC-3
  trace: correct
- test_id: tests/analysis_semantics.rs::differential_v2_requires_retained_identities_and_rejects_v1
  ac_id: TC-007
  trace: correct
- test_id: tests/analysis_semantics.rs::differential_v2_requires_retained_identities_and_rejects_v1
  ac_id: FR-005-AC-6
  trace: correct
- test_id: tests/adr_0010.rs::candidate_identity_rules_match_the_retained_measurement
  ac_id: TC-009
  trace: correct
- test_id: tests/adr_0010.rs::candidate_identity_rules_match_the_retained_measurement
  ac_id: FR-001-AC-2
  trace: correct
- test_id: tests/adr_0010.rs::explicit_bindings_reject_incompatible_members
  ac_id: TC-009
  trace: correct
- test_id: tests/adr_0010.rs::explicit_bindings_reject_incompatible_members
  ac_id: FR-001-AC-2
  trace: correct
- test_id: tests/adr_0010.rs::adr_binds_versions_hashes_and_supersession
  ac_id: TC-009
  trace: correct
- test_id: tests/adr_0010.rs::adr_binds_versions_hashes_and_supersession
  ac_id: FR-001-AC-3
  trace: correct
- test_id: tests/adr_0010.rs::adr_binds_versions_hashes_and_supersession
  ac_id: FR-001-AC-5
  trace: correct
- test_id: tests/solver_adapters.rs::exact_engine_contract_and_protocol_states
  ac_id: TC-005
  trace: correct
- test_id: tests/solver_adapters.rs::exact_engine_contract_and_protocol_states
  ac_id: TC-006
  trace: correct
- test_id: tests/solver_adapters.rs::exact_engine_contract_and_protocol_states
  ac_id: FR-003-AC-2
  trace: correct
- test_id: tests/solver_adapters.rs::exact_engine_contract_and_protocol_states
  ac_id: FR-003-AC-3
  trace: correct
- test_id: tests/solver_adapters.rs::paths_are_absolute_pinned_and_not_shell_interpreted
  ac_id: TC-005
  trace: correct
- test_id: tests/solver_adapters.rs::paths_are_absolute_pinned_and_not_shell_interpreted
  ac_id: FR-003-AC-5
  trace: correct
- test_id: tests/solver_adapters.rs::executable_identity_probe_fails_closed
  ac_id: TC-005
  trace: correct
- test_id: tests/solver_adapters.rs::executable_identity_probe_fails_closed
  ac_id: FR-003-AC-2
  trace: correct
- test_id: tests/solver_adapters.rs::executable_identity_probe_fails_closed
  ac_id: FR-003-AC-4
  trace: correct
- test_id: tests/solver_adapters.rs::every_adapter_owned_limit_is_finite_and_enforced
  ac_id: TC-005
  trace: correct
- test_id: tests/solver_adapters.rs::every_adapter_owned_limit_is_finite_and_enforced
  ac_id: NFR-001-AC-2
  trace: correct
- test_id: tests/solver_adapters.rs::every_adapter_owned_limit_is_finite_and_enforced
  ac_id: NFR-001-AC-3
  trace: correct
- test_id: tests/solver_adapters.rs::hostile_pipe_pressure_remains_bounded
  ac_id: TC-005
  trace: correct
- test_id: tests/solver_adapters.rs::hostile_pipe_pressure_remains_bounded
  ac_id: NFR-001-AC-2
  trace: correct
- test_id: tests/solver_adapters.rs::hostile_pipe_pressure_remains_bounded
  ac_id: NFR-001-AC-3
  trace: correct
- test_id: tests/solver_adapters.rs::timeout_and_cancellation_cleanup_are_measured
  ac_id: TC-005
  trace: correct
- test_id: tests/solver_adapters.rs::timeout_and_cancellation_cleanup_are_measured
  ac_id: FR-003-AC-1
  trace: correct
- test_id: tests/solver_adapters.rs::timeout_and_cancellation_cleanup_are_measured
  ac_id: NFR-001-AC-2
  trace: correct
- test_id: tests/solver_adapters.rs::a_nonzero_exit_is_not_a_conclusion_for_either_engine
  ac_id: TC-005
  trace: correct
- test_id: tests/solver_adapters.rs::a_nonzero_exit_is_not_a_conclusion_for_either_engine
  ac_id: FR-003-AC-2
  trace: correct
- test_id: tests/solver_adapters.rs::a_nonzero_exit_is_not_a_conclusion_for_either_engine
  ac_id: FR-003-AC-4
  trace: correct
- test_id: tests/solver_adapters.rs::a_nonzero_exit_is_not_a_conclusion_for_either_engine
  ac_id: NFR-002-AC-2
  trace: correct
- test_id: tests/hybrid_synthesis.rs::tc_014_canonicalizes_arbitrary_finite_set_like_inputs
  ac_id: TC-014
  trace: correct
- test_id: tests/hybrid_synthesis.rs::tc_014_canonicalizes_arbitrary_finite_set_like_inputs
  ac_id: FR-008-AC-1
  trace: correct
- test_id: tests/hybrid_synthesis.rs::tc_013_preserves_mode_guard_reset_and_exact_replay
  ac_id: TC-013
  trace: correct
- test_id: tests/hybrid_synthesis.rs::tc_013_preserves_mode_guard_reset_and_exact_replay
  ac_id: FR-007-AC-1
  trace: correct
- test_id: tests/hybrid_synthesis.rs::tc_013_preserves_mode_guard_reset_and_exact_replay
  ac_id: FR-007-AC-3
  trace: correct
- test_id: tests/hybrid_synthesis.rs::tc_013_refuses_malformed_models_and_keeps_bound_exhaustion_nonconclusive
  ac_id: TC-013
  trace: correct
- test_id: tests/hybrid_synthesis.rs::tc_013_refuses_malformed_models_and_keeps_bound_exhaustion_nonconclusive
  ac_id: FR-007-AC-2
  trace: correct
- test_id: tests/hybrid_synthesis.rs::tc_014_canonical_candidate_requires_exact_independent_validation
  ac_id: TC-014
  trace: correct
- test_id: tests/hybrid_synthesis.rs::tc_014_canonical_candidate_requires_exact_independent_validation
  ac_id: FR-008-AC-1
  trace: correct
- test_id: tests/hybrid_synthesis.rs::tc_014_canonical_candidate_requires_exact_independent_validation
  ac_id: FR-008-AC-2
  trace: correct
- test_id: tests/hybrid_synthesis.rs::tc_014_distinguishes_exhaustive_no_candidate_from_incomplete_search
  ac_id: TC-014
  trace: correct
- test_id: tests/hybrid_synthesis.rs::tc_014_distinguishes_exhaustive_no_candidate_from_incomplete_search
  ac_id: FR-008-AC-3
  trace: correct
- test_id: tests/hybrid_synthesis.rs::tc_014_distinguishes_delimited_atoms_and_stops_at_search_bound
  ac_id: TC-014
  trace: correct
- test_id: tests/hybrid_synthesis.rs::tc_014_distinguishes_delimited_atoms_and_stops_at_search_bound
  ac_id: FR-008-AC-1
  trace: correct
- test_id: tests/hybrid_synthesis.rs::tc_014_distinguishes_delimited_atoms_and_stops_at_search_bound
  ac_id: FR-008-AC-3
  trace: correct
- test_id: tests/hybrid_synthesis.rs::tc_014_enforces_aggregate_atom_byte_boundary
  ac_id: TC-014
  trace: correct
- test_id: tests/hybrid_synthesis.rs::tc_014_enforces_aggregate_atom_byte_boundary
  ac_id: FR-008-AC-1
  trace: correct
- test_id: src/hybrid_synthesis.rs::binders_preserve_exact_envelope_and_refuse_wrong_method
  ac_id: TC-013
  trace: correct
- test_id: src/hybrid_synthesis.rs::binders_preserve_exact_envelope_and_refuse_wrong_method
  ac_id: TC-014
  trace: correct
```
