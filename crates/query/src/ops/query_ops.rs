use enum_as_inner::EnumAsInner;
use schemajs_primitives::column::types::DataValue;
use schemajs_primitives::index::Index;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct QueryVal {
    pub key: String,
    pub filter_type: String,
    pub value: DataValue,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum QueryOps {
    And(Vec<QueryOps>),
    Or(Vec<QueryOps>),
    Condition(QueryVal),
}

#[cfg(test)]
mod tests {
    use crate::ops::query_ops::{QueryOps, QueryVal};
    use schemajs_primitives::column::types::DataValue;
    use uuid::Uuid;

    #[test]
    fn test_json_ser() {
        let op = QueryOps::Or(vec![
            QueryOps::And(vec![
                QueryOps::Condition(QueryVal {
                    key: "user_age".to_string(),
                    filter_type: "=".to_string(),
                    value: DataValue::String("22".to_string()),
                }),
                QueryOps::Condition(QueryVal {
                    key: "user_country".to_string(),
                    filter_type: "=".to_string(),
                    value: DataValue::String("AR".to_string()),
                }),
                QueryOps::Or(vec![
                    QueryOps::Condition(QueryVal {
                        key: "enabled".to_string(),
                        filter_type: "=".to_string(),
                        value: DataValue::String("true".to_string()),
                    }),
                    QueryOps::Condition(QueryVal {
                        key: "internal".to_string(),
                        filter_type: "=".to_string(),
                        value: DataValue::Uuid(Uuid::new_v4()),
                    }),
                ]),
            ]),
            QueryOps::Condition(QueryVal {
                key: "user_name".to_string(),
                filter_type: "=".to_string(),
                value: DataValue::Null,
            }),
        ]);

        let val = serde_json::to_string(&op).unwrap();
        println!("{}", val);
    }
}
