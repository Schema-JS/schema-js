use enum_as_inner::EnumAsInner;
use schemajs_primitives::column::types::DataValue;
use schemajs_primitives::index::Index;
use std::fmt::Display;

#[derive(Debug, Eq, PartialEq, Clone, EnumAsInner)]
pub enum FilterType {
    Equal,
    GreaterThan,
    LowerThan,
    GreaterOrEqualTo,
    LowerOrEqualTo,
    NotEqual,
}

impl Display for FilterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            FilterType::Equal => String::from("="),
            FilterType::GreaterThan => String::from(">"),
            FilterType::LowerThan => String::from("<"),
            FilterType::GreaterOrEqualTo => String::from(">="),
            FilterType::LowerOrEqualTo => String::from("<="),
            FilterType::NotEqual => String::from("!="),
        };
        write!(f, "{}", str)
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct QueryVal {
    pub key: String,
    pub filter_type: String,
    pub value: DataValue,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum QueryOps {
    And(Vec<QueryOps>),
    Or(Vec<QueryOps>),
    Condition(QueryVal),
}
