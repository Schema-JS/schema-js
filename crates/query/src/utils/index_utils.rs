use crate::ops::query_ops::{QueryOps, QueryPlan, QueryVal};
use schemajs_index::composite_key::CompositeKey;
use schemajs_primitives::index::Index;

// Helper function to find the best index for a condition
fn find_best_index_for_condition(query_val: &QueryVal, indexes: &[Index]) -> Option<Index> {
    indexes
        .iter()
        .find(|index| index.members.contains(&query_val.key))
        .cloned()
}

// Function to recursively create the query plan
pub fn create_query_plan(query_ops: &QueryOps, indexes: &[Index]) -> QueryPlan {
    match query_ops {
        QueryOps::And(ops) => {
            let mut plans = Vec::new();
            for op in ops {
                let sub_plan = create_query_plan(op, indexes);
                plans.push(sub_plan);
            }
            QueryPlan::And(plans)
        }
        QueryOps::Or(ops) => {
            let mut plans = Vec::new();
            for op in ops {
                let sub_plan = create_query_plan(op, indexes);
                plans.push(sub_plan);
            }
            QueryPlan::Or(plans)
        }
        QueryOps::Condition(query_val) => {
            let index = find_best_index_for_condition(query_val, indexes);
            QueryPlan::Index(index)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemajs_index::index_type::IndexType;
    use schemajs_primitives::column::types::DataValue;

    #[test]
    fn test_simple_and_query() {
        let index1 = Index {
            name: "index_first_name".to_string(),
            members: vec!["first_name".to_string()],
            index_type: IndexType::Hash,
        };

        let index2 = Index {
            name: "index_email".to_string(),
            members: vec!["email".to_string()],
            index_type: IndexType::Hash,
        };

        let indexes = vec![index1.clone(), index2.clone()];

        let query = QueryOps::And(vec![
            QueryOps::Condition(QueryVal {
                key: "first_name".to_string(),
                filter_type: "=".to_string(),
                value: DataValue::String("Juan".to_string()),
            }),
            QueryOps::Condition(QueryVal {
                key: "email".to_string(),
                filter_type: "=".to_string(),
                value: DataValue::String("juan@example.com".to_string()),
            }),
        ]);

        let query_plan = create_query_plan(&query, &indexes);

        match query_plan {
            QueryPlan::And(ref plans) => {
                assert_eq!(plans.len(), 2);
                match &plans[0] {
                    QueryPlan::Index(Some(index)) => assert_eq!(index.name, "index_first_name"),
                    _ => panic!("Expected index for first_name"),
                }
                match &plans[1] {
                    QueryPlan::Index(Some(index)) => assert_eq!(index.name, "index_email"),
                    _ => panic!("Expected index for email"),
                }
            }
            _ => panic!("Expected AND query plan"),
        }
    }

    #[test]
    fn test_simple_or_query() {
        let index1 = Index {
            name: "index_last_name".to_string(),
            members: vec!["last_name".to_string()],
            index_type: IndexType::Hash,
        };

        let index2 = Index {
            name: "index_email".to_string(),
            members: vec!["email".to_string()],
            index_type: IndexType::Hash,
        };

        let indexes = vec![index1.clone(), index2.clone()];

        let query = QueryOps::Or(vec![
            QueryOps::Condition(QueryVal {
                key: "last_name".to_string(),
                filter_type: "=".to_string(),
                value: DataValue::String("Doe".to_string()),
            }),
            QueryOps::Condition(QueryVal {
                key: "email".to_string(),
                filter_type: "=".to_string(),
                value: DataValue::String("doe@example.com".to_string()),
            }),
        ]);

        let query_plan = create_query_plan(&query, &indexes);

        match query_plan {
            QueryPlan::Or(ref plans) => {
                assert_eq!(plans.len(), 2);
                match &plans[0] {
                    QueryPlan::Index(Some(index)) => assert_eq!(index.name, "index_last_name"),
                    _ => panic!("Expected index for last_name"),
                }
                match &plans[1] {
                    QueryPlan::Index(Some(index)) => assert_eq!(index.name, "index_email"),
                    _ => panic!("Expected index for email"),
                }
            }
            _ => panic!("Expected OR query plan"),
        }
    }

    #[test]
    fn test_nested_and_or_query() {
        let index1 = Index {
            name: "index_first_name".to_string(),
            members: vec!["first_name".to_string()],
            index_type: IndexType::Hash,
        };

        let index2 = Index {
            name: "index_last_name".to_string(),
            members: vec!["last_name".to_string()],
            index_type: IndexType::Hash,
        };

        let index3 = Index {
            name: "index_email".to_string(),
            members: vec!["email".to_string()],
            index_type: IndexType::Hash,
        };

        let indexes = vec![index1.clone(), index2.clone(), index3.clone()];

        let query = QueryOps::And(vec![
            QueryOps::Or(vec![
                QueryOps::Condition(QueryVal {
                    key: "last_name".to_string(),
                    filter_type: "=".to_string(),
                    value: DataValue::String("Doe".to_string()),
                }),
                QueryOps::Condition(QueryVal {
                    key: "email".to_string(),
                    filter_type: "=".to_string(),
                    value: DataValue::String("doe@example.com".to_string()),
                }),
            ]),
            QueryOps::Condition(QueryVal {
                key: "first_name".to_string(),
                filter_type: "=".to_string(),
                value: DataValue::String("John".to_string()),
            }),
        ]);

        let query_plan = create_query_plan(&query, &indexes);

        match query_plan {
            QueryPlan::And(ref plans) => {
                assert_eq!(plans.len(), 2);
                match &plans[0] {
                    QueryPlan::Or(ref or_plans) => {
                        assert_eq!(or_plans.len(), 2);
                        match &or_plans[0] {
                            QueryPlan::Index(Some(index)) => {
                                assert_eq!(index.name, "index_last_name")
                            }
                            _ => panic!("Expected index for last_name"),
                        }
                        match &or_plans[1] {
                            QueryPlan::Index(Some(index)) => assert_eq!(index.name, "index_email"),
                            _ => panic!("Expected index for email"),
                        }
                    }
                    _ => panic!("Expected OR plan inside AND"),
                }
                match &plans[1] {
                    QueryPlan::Index(Some(index)) => assert_eq!(index.name, "index_first_name"),
                    _ => panic!("Expected index for first_name"),
                }
            }
            _ => panic!("Expected AND query plan"),
        }
    }

    #[test]
    fn test_no_matching_index() {
        let index1 = Index {
            name: "index_first_name".to_string(),
            members: vec!["first_name".to_string()],
            index_type: IndexType::Hash,
        };

        let indexes = vec![index1.clone()];

        let query = QueryOps::Condition(QueryVal {
            key: "email".to_string(),
            filter_type: "=".to_string(),
            value: DataValue::String("juan@example.com".to_string()),
        });

        let query_plan = create_query_plan(&query, &indexes);

        match query_plan {
            QueryPlan::Index(None) => (), // Expected behavior
            _ => panic!("Expected None for unmatched index"),
        }
    }
}
