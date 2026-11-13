//! Typed values, replay terms and the bounded model-value parser for datatype witnesses.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    analysis::{take_model_scalar, ModelScalar, ModelToken},
    smt::{
        EnumSortMap, OptionSortMap, RecordSortMap, ReplayAssignment, ReplayExpression, ReplayFault,
        ReplayTerm, MAX_DATATYPE_DEPTH,
    },
    QueryBundle,
};

/// A typed value of a model: a Boolean, an integer, or an enum, record or option value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReplayValue {
    Bool(bool),
    Int(i128),
    /// An enum value, by its constructor symbol.
    Enum(String),
    /// A record value, every field in declared field order.
    Record(Vec<ReplayValue>),
    /// An option value: absent, or present with its payload.
    Option(Option<Box<ReplayValue>>),
}

/// The declared type of a datatype-sorted variable: its sorts, field order and integer ranges.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReplayShape {
    Bool,
    Int {
        minimum: i128,
        maximum: i128,
    },
    Enum {
        sort: String,
    },
    Record {
        sort: String,
        fields: Vec<ReplayShape>,
    },
    Option {
        sort: String,
        payload: Box<ReplayShape>,
    },
}

impl ReplayShape {
    fn sort(&self) -> Option<&str> {
        match self {
            Self::Bool | Self::Int { .. } => None,
            Self::Enum { sort } | Self::Record { sort, .. } | Self::Option { sort, .. } => {
                Some(sort)
            }
        }
    }

    /// Whether an integer part of the value lies outside its declared range.
    pub(crate) fn out_of_range(&self, value: &ReplayValue) -> bool {
        match (self, value) {
            (Self::Int { minimum, maximum }, ReplayValue::Int(value)) => {
                value < minimum || value > maximum
            }
            (Self::Record { fields, .. }, ReplayValue::Record(values)) => fields
                .iter()
                .zip(values)
                .any(|(shape, value)| shape.out_of_range(value)),
            (Self::Option { payload, .. }, ReplayValue::Option(Some(value))) => {
                payload.out_of_range(value)
            }
            _ => false,
        }
    }
}

/// A datatype-valued replay expression, evaluated against the typed model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReplayDatum {
    Variable(String),
    /// An enum literal, by its constructor symbol.
    Enum(String),
    /// A record literal, every field in declared field order.
    Record(Vec<ReplayItem>),
    /// The field at a declared position of a record.
    Field {
        base: Box<ReplayDatum>,
        index: usize,
    },
    None,
    Some(Box<ReplayItem>),
    /// The payload of an option; an absent option is a replay failure.
    Unwrap(Box<ReplayDatum>),
}

/// A replay expression of any sort: a field value of a record or the payload of an option.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReplayItem {
    Bool(ReplayExpression),
    Int(ReplayTerm),
    Datum(ReplayDatum),
}

impl ReplayDatum {
    pub(crate) fn evaluate(&self, values: &ReplayAssignment) -> Result<ReplayValue, ReplayFault> {
        match self {
            Self::Variable(symbol) => values
                .datums
                .get(symbol)
                .cloned()
                .ok_or(ReplayFault::Unassigned),
            Self::Enum(constructor) => Ok(ReplayValue::Enum(constructor.clone())),
            Self::Record(items) => items
                .iter()
                .map(|item| item.evaluate(values))
                .collect::<Result<Vec<_>, _>>()
                .map(ReplayValue::Record),
            Self::Field { base, index } => match base.evaluate(values)? {
                ReplayValue::Record(mut fields) if *index < fields.len() => {
                    Ok(fields.swap_remove(*index))
                }
                _ => Err(ReplayFault::IllTyped),
            },
            Self::None => Ok(ReplayValue::Option(None)),
            Self::Some(item) => Ok(ReplayValue::Option(Some(Box::new(item.evaluate(values)?)))),
            Self::Unwrap(option) => match option.evaluate(values)? {
                ReplayValue::Option(Some(payload)) => Ok(*payload),
                ReplayValue::Option(None) => Err(ReplayFault::AbsentOption),
                _ => Err(ReplayFault::IllTyped),
            },
        }
    }
}

impl ReplayItem {
    pub(crate) fn evaluate(&self, values: &ReplayAssignment) -> Result<ReplayValue, ReplayFault> {
        match self {
            Self::Bool(expression) => expression.evaluate(values).map(ReplayValue::Bool),
            Self::Int(term) => term.evaluate(values).map(ReplayValue::Int),
            Self::Datum(datum) => datum.evaluate(values),
        }
    }
}

/// Why a datatype value does not match its declared type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ValueDefect {
    /// The text is not a value, or names a constructor no datatype of the query declares.
    Malformed,
    /// A constructor of another datatype, a wrong arity or a wrong qualifying sort.
    SortMismatch,
    /// An integer beyond 128 bits, or a value nested beyond the datatype depth bound.
    OutOfDomain,
}

/// A sort as a solver prints it: `S`, or the parametric `(Option S)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ModelSort {
    Atom(String),
    Applied(String, String),
}

/// Reads a sort term; `None` when it is neither an atom nor a one-argument application.
pub(crate) fn read_sort(tokens: &[ModelToken], cursor: &mut usize) -> Option<ModelSort> {
    match tokens.get(*cursor)? {
        ModelToken::Atom(atom) => {
            *cursor += 1;
            Some(ModelSort::Atom(atom.clone()))
        }
        ModelToken::Open => match tokens.get(*cursor + 1..*cursor + 4)? {
            [ModelToken::Atom(head), ModelToken::Atom(argument), ModelToken::Close] => {
                *cursor += 4;
                Some(ModelSort::Applied(head.clone(), argument.clone()))
            }
            _ => None,
        },
        ModelToken::Close => None,
    }
}

/// The query's symbol map, the only source of constructor and sort symbols.
pub(crate) struct DatatypeContext<'a> {
    shapes: &'a BTreeMap<String, ReplayShape>,
    enums: BTreeMap<&'a str, &'a EnumSortMap>,
    records: BTreeMap<&'a str, &'a RecordSortMap>,
    options: BTreeMap<&'a str, &'a OptionSortMap>,
    constructors: BTreeSet<&'a str>,
}

impl<'a> DatatypeContext<'a> {
    pub(crate) fn new(query: &'a QueryBundle) -> Self {
        let mut constructors = BTreeSet::new();
        for sort in query.enum_sorts() {
            constructors.extend(sort.constructors.iter().map(|c| c.symbol.as_str()));
        }
        for sort in query.record_sorts() {
            constructors.insert(sort.constructor.as_str());
        }
        for sort in query.option_sorts() {
            constructors.insert(sort.none.as_str());
            constructors.insert(sort.some.as_str());
        }
        Self {
            shapes: query.datatype_shapes(),
            enums: query
                .enum_sorts()
                .iter()
                .map(|s| (s.sort.as_str(), s))
                .collect(),
            records: query
                .record_sorts()
                .iter()
                .map(|s| (s.sort.as_str(), s))
                .collect(),
            options: query
                .option_sorts()
                .iter()
                .map(|s| (s.sort.as_str(), s))
                .collect(),
            constructors,
        }
    }

    pub(crate) fn shape(&self, symbol: &str) -> Option<&'a ReplayShape> {
        self.shapes.get(symbol)
    }

    /// Whether the sort names a datatype declared by the query.
    pub(crate) fn is_sort(&self, sort: &ModelSort) -> bool {
        match sort {
            ModelSort::Atom(atom) => {
                self.enums.contains_key(atom.as_str())
                    || self.records.contains_key(atom.as_str())
                    || self.options.contains_key(atom.as_str())
            }
            ModelSort::Applied(..) => false,
        }
    }

    /// Whether a printed sort spells the shape's sort symbol.
    pub(crate) fn sort_matches(&self, sort: &ModelSort, shape: &ReplayShape) -> bool {
        let Some(symbol) = shape.sort() else {
            return false;
        };
        match sort {
            ModelSort::Atom(atom) => atom == symbol,
            ModelSort::Applied(head, argument) => {
                head == "Option"
                    && self
                        .options
                        .get(symbol)
                        .is_some_and(|map| map.payload_sort == *argument)
            }
        }
    }

    /// Reads one typed value from the cursor. The cursor is unspecified after a defect.
    pub(crate) fn parse_value(
        &self,
        tokens: &[ModelToken],
        cursor: &mut usize,
        shape: &ReplayShape,
        depth: usize,
    ) -> Result<ReplayValue, ValueDefect> {
        match shape {
            ReplayShape::Bool => {
                match take_model_scalar(tokens, cursor).unwrap_or(ModelScalar::Malformed) {
                    ModelScalar::Boolean(value) => Ok(ReplayValue::Bool(value)),
                    ModelScalar::Integer(_) => Err(ValueDefect::SortMismatch),
                    ModelScalar::OutOfDomain => Err(ValueDefect::OutOfDomain),
                    ModelScalar::Malformed => Err(ValueDefect::Malformed),
                }
            }
            ReplayShape::Int { .. } => {
                match take_model_scalar(tokens, cursor).unwrap_or(ModelScalar::Malformed) {
                    ModelScalar::Integer(value) => Ok(ReplayValue::Int(value)),
                    ModelScalar::Boolean(_) => Err(ValueDefect::SortMismatch),
                    ModelScalar::OutOfDomain => Err(ValueDefect::OutOfDomain),
                    ModelScalar::Malformed => Err(ValueDefect::Malformed),
                }
            }
            datatype => self.parse_datatype(tokens, cursor, datatype, depth + 1),
        }
    }

    fn parse_datatype(
        &self,
        tokens: &[ModelToken],
        cursor: &mut usize,
        shape: &ReplayShape,
        depth: usize,
    ) -> Result<ReplayValue, ValueDefect> {
        if depth > MAX_DATATYPE_DEPTH {
            return Err(ValueDefect::OutOfDomain);
        }
        let (constructor, applied) = self.read_head(tokens, cursor, shape)?;
        match shape {
            ReplayShape::Enum { sort } => {
                let map = self
                    .enums
                    .get(sort.as_str())
                    .ok_or(ValueDefect::Malformed)?;
                let known = map.constructors.iter().any(|c| c.symbol == constructor);
                if !known {
                    return Err(self.unknown_constructor(constructor));
                }
                if applied {
                    return Err(ValueDefect::SortMismatch);
                }
                Ok(ReplayValue::Enum(constructor.to_owned()))
            }
            ReplayShape::Record { sort, fields } => {
                let map = self
                    .records
                    .get(sort.as_str())
                    .ok_or(ValueDefect::Malformed)?;
                if constructor != map.constructor {
                    return Err(self.unknown_constructor(constructor));
                }
                if applied == fields.is_empty() {
                    return Err(ValueDefect::SortMismatch);
                }
                let mut values = Vec::with_capacity(fields.len());
                for field in fields {
                    values.push(self.parse_argument(tokens, cursor, field, depth)?);
                }
                finish(tokens, cursor, applied)?;
                Ok(ReplayValue::Record(values))
            }
            ReplayShape::Option { sort, payload } => {
                let map = self
                    .options
                    .get(sort.as_str())
                    .ok_or(ValueDefect::Malformed)?;
                if constructor == map.none {
                    if applied {
                        return Err(ValueDefect::SortMismatch);
                    }
                    Ok(ReplayValue::Option(None))
                } else if constructor == map.some {
                    if !applied {
                        return Err(ValueDefect::SortMismatch);
                    }
                    let value = self.parse_argument(tokens, cursor, payload, depth)?;
                    finish(tokens, cursor, applied)?;
                    Ok(ReplayValue::Option(Some(Box::new(value))))
                } else {
                    Err(self.unknown_constructor(constructor))
                }
            }
            ReplayShape::Bool | ReplayShape::Int { .. } => Err(ValueDefect::Malformed),
        }
    }

    fn parse_argument(
        &self,
        tokens: &[ModelToken],
        cursor: &mut usize,
        shape: &ReplayShape,
        depth: usize,
    ) -> Result<ReplayValue, ValueDefect> {
        if matches!(tokens.get(*cursor), Some(ModelToken::Close) | None) {
            return Err(ValueDefect::SortMismatch);
        }
        self.parse_value(tokens, cursor, shape, depth)
    }

    /// A constructor of another datatype is a sort mismatch; one the query never declared is not
    /// a value of any declared datatype.
    fn unknown_constructor(&self, constructor: &str) -> ValueDefect {
        if self.constructors.contains(constructor) {
            ValueDefect::SortMismatch
        } else {
            ValueDefect::Malformed
        }
    }

    /// Reads `C`, `(as C S)` or the opening of `(C ...)`. The flag is true for an application,
    /// whose closing parenthesis is still unread.
    fn read_head<'t>(
        &self,
        tokens: &'t [ModelToken],
        cursor: &mut usize,
        shape: &ReplayShape,
    ) -> Result<(&'t str, bool), ValueDefect> {
        match tokens.get(*cursor) {
            Some(ModelToken::Atom(constructor)) => {
                *cursor += 1;
                Ok((constructor, false))
            }
            Some(ModelToken::Open) => match tokens.get(*cursor + 1) {
                Some(ModelToken::Atom(head)) if head == "as" => {
                    let Some(ModelToken::Atom(constructor)) = tokens.get(*cursor + 2) else {
                        return Err(ValueDefect::Malformed);
                    };
                    *cursor += 3;
                    let sort = read_sort(tokens, cursor).ok_or(ValueDefect::Malformed)?;
                    if !matches!(tokens.get(*cursor), Some(ModelToken::Close)) {
                        return Err(ValueDefect::Malformed);
                    }
                    *cursor += 1;
                    if !self.sort_matches(&sort, shape) {
                        return Err(ValueDefect::SortMismatch);
                    }
                    Ok((constructor, false))
                }
                Some(ModelToken::Atom(constructor)) => {
                    *cursor += 2;
                    Ok((constructor, true))
                }
                _ => Err(ValueDefect::Malformed),
            },
            _ => Err(ValueDefect::Malformed),
        }
    }
}

/// Consumes the closing parenthesis of an application; an extra argument is a wrong arity.
fn finish(tokens: &[ModelToken], cursor: &mut usize, applied: bool) -> Result<(), ValueDefect> {
    if !applied {
        return Ok(());
    }
    if matches!(tokens.get(*cursor), Some(ModelToken::Close)) {
        *cursor += 1;
        Ok(())
    } else {
        Err(ValueDefect::SortMismatch)
    }
}

/// A replay-verified value as published: enums by variant name, records by field name, options
/// as absent or present with the payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WitnessValue {
    Boolean(bool),
    Integer(i128),
    Enum {
        enumeration: String,
        variant: String,
    },
    Record {
        record: String,
        fields: Vec<(String, WitnessValue)>,
    },
    Option(Option<Box<WitnessValue>>),
}

impl std::fmt::Display for WitnessValue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Boolean(value) => write!(formatter, "{value}"),
            Self::Integer(value) => write!(formatter, "{value}"),
            Self::Enum { variant, .. } => formatter.write_str(variant),
            Self::Record { fields, .. } => {
                formatter.write_str("{")?;
                for (index, (name, value)) in fields.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(", ")?;
                    }
                    write!(formatter, "{name}: {value}")?;
                }
                formatter.write_str("}")
            }
            Self::Option(None) => formatter.write_str("absent"),
            Self::Option(Some(value)) => write!(formatter, "present({value})"),
        }
    }
}

impl DatatypeContext<'_> {
    /// The published form of a value already checked against its shape.
    pub(crate) fn witness(&self, shape: &ReplayShape, value: &ReplayValue) -> Option<WitnessValue> {
        Some(match (shape, value) {
            (ReplayShape::Bool, ReplayValue::Bool(value)) => WitnessValue::Boolean(*value),
            (ReplayShape::Int { .. }, ReplayValue::Int(value)) => WitnessValue::Integer(*value),
            (ReplayShape::Enum { sort }, ReplayValue::Enum(symbol)) => {
                let map = self.enums.get(sort.as_str())?;
                WitnessValue::Enum {
                    enumeration: map.name.clone(),
                    variant: map
                        .constructors
                        .iter()
                        .find(|constructor| constructor.symbol == *symbol)?
                        .variant
                        .clone(),
                }
            }
            (ReplayShape::Record { sort, fields }, ReplayValue::Record(values)) => {
                let map = self.records.get(sort.as_str())?;
                let fields = map
                    .fields
                    .iter()
                    .zip(fields)
                    .zip(values)
                    .map(|((field, shape), value)| {
                        Some((field.field.clone(), self.witness(shape, value)?))
                    })
                    .collect::<Option<Vec<_>>>()?;
                WitnessValue::Record {
                    record: map.name.clone(),
                    fields,
                }
            }
            (ReplayShape::Option { payload, .. }, ReplayValue::Option(value)) => {
                WitnessValue::Option(match value {
                    Some(value) => Some(Box::new(self.witness(payload, value)?)),
                    None => None,
                })
            }
            _ => return None,
        })
    }
}
