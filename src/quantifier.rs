//! Quantifiers over bounded integer ranges, enum domains and finite collections (FR-030,
//! FR-031, FR-032, NFR-030).
//!
//! The contract model has no range-domain quantifier form, so the analysis model carries one at
//! the lowering layer: an [`AnalysisFormula`] whose leaves are contract expressions and whose
//! quantifier nodes are [`RangeQuantifier`]s. A leaf names a quantifier's bound variable with
//! `ExpressionKind::LocalReference`.

use quire_contract_model::{Expression, IntegerType, QuantifierKind, SymbolName, ValueType};

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
    /// A quantifier over an enum domain or a finite collection.
    Domain(Box<DomainQuantifier>),
}

/// One literal element of a finite collection.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DomainElement {
    Boolean(bool),
    Enum {
        enumeration: SymbolName,
        variant: SymbolName,
    },
    Integer(i128),
}

/// The catalog constructor of a collection literal (`quire.op.collection.*`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollectionKind {
    Set,
    OrderedSet,
    Sequence,
    Bag,
}

impl CollectionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Set => "set",
            Self::OrderedSet => "ordered_set",
            Self::Sequence => "sequence",
            Self::Bag => "bag",
        }
    }

    /// Whether a repeated literal collapses to its first occurrence.
    pub(crate) const fn collapses_repeats(self) -> bool {
        matches!(self, Self::Set | Self::OrderedSet)
    }

    /// Whether elements are enumerated in ascending canonical order rather than as written.
    pub(crate) const fn sorted(self) -> bool {
        matches!(self, Self::Set | Self::Bag)
    }
}

/// A domain with no finite, exactly encodable members. Lowering refuses it (FR-030).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnboundedDomain {
    /// An integer with no finite declared bounds.
    Integer,
    Text,
    Rational,
    /// A collection that is a parameter reference or has a non-literal element.
    UnfixedCollection,
    /// A collection element sort with no exact encoding, such as a nested collection.
    UnencodableElement,
}

impl UnboundedDomain {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Integer => "integer",
            Self::Text => "text",
            Self::Rational => "rational",
            Self::UnfixedCollection => "unfixed_collection",
            Self::UnencodableElement => "unencodable_element",
        }
    }

    pub(crate) const fn reason(self) -> QuantifierReason {
        match self {
            Self::Integer => QuantifierReason::UnboundedDomain,
            Self::Text => QuantifierReason::TextDomain,
            Self::Rational => QuantifierReason::RationalDomain,
            Self::UnfixedCollection => QuantifierReason::UnfixedCollection,
            Self::UnencodableElement => QuantifierReason::UnencodableElement,
        }
    }
}

/// The members a quantifier ranges over.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuantifierDomain {
    /// Every declared member of the named enum, in declaration order.
    Enum(SymbolName),
    /// A collection literal of fixed elements.
    Collection {
        kind: CollectionKind,
        elements: Vec<DomainElement>,
    },
    /// A domain that is refused at lowering.
    Unbounded(UnboundedDomain),
}

/// A universal or existential quantifier over an enum domain or a finite collection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomainQuantifier {
    pub identity: String,
    pub kind: QuantifierKind,
    pub local: SymbolName,
    /// The sort of the bound variable: Boolean, an enum, or a bounded integer.
    pub local_type: ValueType,
    pub domain: QuantifierDomain,
    pub body: AnalysisFormula,
}

impl AnalysisFormula {
    /// A stable text form of the formula, used to bind a statement's identity to it. A leaf that
    /// cannot be serialized is an error, never an empty term.
    pub(crate) fn identity_text(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.to_value()?)
    }

    fn to_value(&self) -> Result<serde_json::Value, serde_json::Error> {
        use serde_json::json;
        Ok(match self {
            Self::Term(expression) => json!({ "term": serde_json::to_value(&**expression)? }),
            Self::Not(operand) => json!({ "not": operand.to_value()? }),
            Self::And(left, right) => json!({ "and": [left.to_value()?, right.to_value()?] }),
            Self::Or(left, right) => json!({ "or": [left.to_value()?, right.to_value()?] }),
            Self::Implies(left, right) => {
                json!({ "implies": [left.to_value()?, right.to_value()?] })
            }
            Self::Range(quantifier) => json!({
                "range": {
                    "identity": quantifier.identity,
                    "kind": match quantifier.kind {
                        QuantifierKind::ForAll => "forall",
                        QuantifierKind::Exists => "exists",
                    },
                    "local": quantifier.local.as_str(),
                    "local_type": serde_json::to_value(&quantifier.local_type)?,
                    "lower": quantifier.lower.to_value(),
                    "upper": quantifier.upper.to_value(),
                    "body": quantifier.body.to_value()?,
                }
            }),
            Self::Domain(quantifier) => json!({
                "domain": {
                    "identity": quantifier.identity,
                    "kind": match quantifier.kind {
                        QuantifierKind::ForAll => "forall",
                        QuantifierKind::Exists => "exists",
                    },
                    "local": quantifier.local.as_str(),
                    "local_type": serde_json::to_value(&quantifier.local_type)?,
                    "domain": quantifier.domain.to_value(),
                    "body": quantifier.body.to_value()?,
                }
            }),
        })
    }
}

impl QuantifierDomain {
    fn to_value(&self) -> serde_json::Value {
        use serde_json::json;
        match self {
            Self::Enum(name) => json!({ "enum": name.as_str() }),
            // Two spellings that enumerate the same members share one identity: a repeat that
            // collapses and the written order of an unordered collection do not matter.
            Self::Collection { kind, elements } => {
                let mut members: Vec<&DomainElement> = elements.iter().collect();
                if kind.sorted() {
                    members.sort();
                }
                if kind.collapses_repeats() {
                    let mut seen = std::collections::BTreeSet::new();
                    members.retain(|member| seen.insert(*member));
                }
                json!({
                    "collection": kind.as_str(),
                    "elements": members.into_iter().map(DomainElement::to_value).collect::<Vec<_>>(),
                })
            }
            Self::Unbounded(class) => json!({ "unbounded": class.as_str() }),
        }
    }
}

impl DomainElement {
    fn to_value(&self) -> serde_json::Value {
        use serde_json::json;
        match self {
            Self::Boolean(value) => json!({ "boolean": value }),
            Self::Enum {
                enumeration,
                variant,
            } => json!({ "enum": [enumeration.as_str(), variant.as_str()] }),
            Self::Integer(value) => json!({ "integer": value.to_string() }),
        }
    }

    /// The element as it is named in a quantifier-map entry.
    pub(crate) fn label(&self) -> String {
        match self {
            Self::Boolean(value) => value.to_string(),
            Self::Enum { variant, .. } => variant.as_str().to_owned(),
            Self::Integer(value) => value.to_string(),
        }
    }
}

impl RangeBound {
    fn to_value(&self) -> serde_json::Value {
        use serde_json::json;
        match self {
            Self::Literal(value) => json!({ "literal": value.to_string() }),
            Self::Declared(name) => json!({ "declared": name.as_str() }),
            Self::Unbounded => json!("unbounded"),
        }
    }
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
    pub encoding: QuantifierEncoding,
    /// Instances expansion produced; zero for a bounded quantifier, which consumes no budget.
    pub instance_count: u128,
    /// The range bounds or typed expanded members, matching the quantifier's domain class.
    pub domain_data: QuantifierEntryDomain,
}

/// Domain-specific data retained in a quantifier-map entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuantifierEntryDomain {
    /// Bounds of a range quantifier, including solver-bound forms.
    Range {
        lower: RangeBound,
        upper: RangeBound,
    },
    /// Typed members of an enum quantifier in declaration order.
    Enum { elements: Vec<DomainElement> },
    /// Typed members of a collection quantifier in expansion order.
    Collection { elements: Vec<DomainElement> },
}

impl QuantifierEntry {
    /// The quantifier's domain class, derived from its domain-specific entry data.
    pub const fn domain(&self) -> QuantifierDomainClass {
        match &self.domain_data {
            QuantifierEntryDomain::Range { .. } => QuantifierDomainClass::Range,
            QuantifierEntryDomain::Enum { .. } => QuantifierDomainClass::Enum,
            QuantifierEntryDomain::Collection { .. } => QuantifierDomainClass::Collection,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// NFR-030-AC-3: the checked products and sums never wrap.
    #[test]
    fn expansion_cost_is_checked() {
        assert_eq!(expansion_cost(Some(1024), 1, 0), Some(1024));
        assert_eq!(expansion_cost(Some(1025), 1, 0), None);
        assert_eq!(expansion_cost(Some(1), 1, 1024), None);
        assert_eq!(expansion_cost(Some(0), 1, 1024), Some(0));
        assert_eq!(expansion_cost(None, 1, 0), None);
        // A product past 2^128 and a sum past 2^128 both count as exceeding.
        assert_eq!(expansion_cost(Some(u128::MAX), 2, 0), None);
        assert_eq!(expansion_cost(Some(u128::MAX / 2 + 1), 2, 0), None);
        assert_eq!(expansion_cost(Some(1), 1, u128::MAX), None);
        assert_eq!(literal_width(i128::MIN, i128::MAX), None);
        assert_eq!(literal_width(i128::MIN, i128::MAX - 1), Some(u128::MAX));
        assert_eq!(literal_width(3, 2), Some(0));
    }
}
