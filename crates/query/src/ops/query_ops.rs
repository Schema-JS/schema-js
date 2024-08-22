use enum_as_inner::EnumAsInner;
use schemajs_primitives::column::types::DataValue;

#[derive(Debug, Eq, PartialEq, Clone, EnumAsInner)]
pub enum FilterType {
    Equal,
    GreaterThan,
    LowerThan,
    GreaterOrEqualTo,
    LowerOrEqualTo,
    NotEqual,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct QueryVal {
    pub key: String,
    pub filter_type: FilterType,
    pub value: DataValue,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum QueryOps {
    And(Vec<QueryOps>),
    Or(Vec<QueryOps>),
    Condition(QueryVal),
}
