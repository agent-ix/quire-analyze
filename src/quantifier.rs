//! Quantifiers over bounded integer ranges (FR-030, FR-032, NFR-030).
//!
//! The contract model has no range-domain quantifier form, so the analysis model carries one at
//! the lowering layer: an [`AnalysisFormula`] whose leaves are contract expressions and whose
//! quantifier nodes are [`RangeQuantifier`]s. A leaf names a quantifier's bound variable with
//! `ExpressionKind::LocalReference`.

use std::fmt::Write as _;

use quire_contract_model::{Expression, IntegerType, QuantifierKind, SymbolName};

/// Bound on the instances that expansion may produce in one request, summed over all clauses.
pub const MAX_EXPANDED_INSTANCES: u128 = 1024;

/// The closed set of reasons a quantifier is refused (FR-030).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum QuantifierReason {
    UnboundedDomain,
    NonFiniteBound,
    TextDomain,
    RationalDomain,
    UnfixedCollection,
    UnencodableElement,
    ExpansionCeiling,
}

impl QuantifierReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnboundedDomain => "unbounded-domain",
            Self::NonFiniteBound => "non-finite-bound",
            Self::TextDomain => "text-domain",
            Self::RationalDomain => "rational-domain",
            Self::UnfixedCollection => "unfixed-collection",
            Self::UnencodableElement => "unencodable-element",
            Self::ExpansionCeiling => "expansion-ceiling",
        }
    }
}

/// A refused quantifier: its source identity and the reason class.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuantifierRefusal {
    pub identity: String,
    pub reason: QuantifierReason,
}

/// One end of an inclusive integer range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RangeBound {
    /// An integer literal.
    Literal(i128),
    /// The declared bounds of a bounded integer input, read at the current observation.
    Declared(SymbolName),
    /// A bound that is not provably finite.
    Unbounded,
}

/// A universal or existential quantifier over an inclusive integer range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RangeQuantifier {
    pub identity: String,
    pub kind: QuantifierKind,
    pub local: SymbolName,
    pub local_type: IntegerType,
    pub lower: RangeBound,
    pub upper: RangeBound,
    pub body: AnalysisFormula,
}

/// A Boolean statement body in the analysis model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnalysisFormula {
    /// A quantifier-free contract expression.
    Term(Box<Expression>),
    Not(Box<AnalysisFormula>),
    And(Box<AnalysisFormula>, Box<AnalysisFormula>),
    Or(Box<AnalysisFormula>, Box<AnalysisFormula>),
    Implies(Box<AnalysisFormula>, Box<AnalysisFormula>),
    Range(Box<RangeQuantifier>),
}

impl AnalysisFormula {
    /// A stable text form, used to bind a statement's identity to its formula.
    pub(crate) fn canonical(&self, out: &mut String) {
        match self {
            Self::Term(expression) => {
                out.push_str("term(");
                out.push_str(&serde_json::to_string(expression).unwrap_or_default());
                out.push(')');
            }
            Self::Not(operand) => {
                out.push_str("not(");
                operand.canonical(out);
                out.push(')');
            }
            Self::And(left, right) => binary("and", left, right, out),
            Self::Or(left, right) => binary("or", left, right, out),
            Self::Implies(left, right) => binary("implies", left, right, out),
            Self::Range(quantifier) => {
                let _ = write!(
                    out,
                    "range({},{:?},{},{:?},{:?},{:?},",
                    quantifier.identity,
                    quantifier.kind,
                    quantifier.local.as_str(),
                    quantifier.local_type,
                    quantifier.lower,
                    quantifier.upper
                );
                quantifier.body.canonical(out);
                out.push(')');
            }
        }
    }
}

fn binary(name: &str, left: &AnalysisFormula, right: &AnalysisFormula, out: &mut String) {
    out.push_str(name);
    out.push('(');
    left.canonical(out);
    out.push(',');
    right.canonical(out);
    out.push(')');
}

/// The domain class recorded in a quantifier-map entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuantifierDomainClass {
    Enum,
    Collection,
    Range,
}

impl QuantifierDomainClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Enum => "enum",
            Self::Collection => "collection",
            Self::Range => "range",
        }
    }
}

/// How a quantifier is encoded in the query.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuantifierEncoding {
    Expanded,
    Bounded,
}

impl QuantifierEncoding {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Expanded => "expanded",
            Self::Bounded => "bounded",
        }
    }
}

/// The quantifier-map entry of one admitted quantifier (FR-030, FR-032).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuantifierEntry {
    pub source: String,
    pub kind: QuantifierKind,
    pub domain: QuantifierDomainClass,
    pub encoding: QuantifierEncoding,
    /// Instances expansion produced; zero for a bounded quantifier, which consumes no budget.
    pub instance_count: u128,
    pub lower: RangeBound,
    pub upper: RangeBound,
}

/// The width of an inclusive literal range, or `None` when it is not representable. An empty
/// range has width zero.
pub(crate) fn literal_width(lower: i128, upper: i128) -> Option<u128> {
    if lower > upper {
        return Some(0);
    }
    upper.abs_diff(lower).checked_add(1)
}

/// Picks the encoding for a literal range from the remaining budget. `enclosing` is the product
/// of the instance counts of the enclosing expanded quantifiers. Returns the instance count to
/// charge, or `None` when the quantifier takes the bounded form.
pub(crate) fn expansion_cost(width: Option<u128>, enclosing: u128, used: u128) -> Option<u128> {
    let count = width?.checked_mul(enclosing)?;
    let total = used.checked_add(count)?;
    (total <= MAX_EXPANDED_INSTANCES).then_some(count)
}
