use crate::ops::query_ops::{QueryOps, QueryVal};
use schemajs_index::composite_key::CompositeKey;
use schemajs_primitives::index::Index;

// Function to check if an Index is present in the CompositeKey
pub fn matches_index(index: &Index, composite_key: &CompositeKey) -> bool {
    let keys: Vec<String> = composite_key
        .0
        .iter()
        .map(|(key, _value)| key.clone())
        .collect();
    index.members.iter().all(|member| keys.contains(member))
}

// Function to get all matching Indexes for a given CompositeKey
pub fn matching_indexes(indexes: &[Index], composite_key: &CompositeKey) -> Vec<Index> {
    indexes
        .iter()
        .cloned()
        .filter(|index| matches_index(index, composite_key))
        .collect()
}

// Function to check if an Index can be used based on QueryVal keys
pub fn index_matches_condition(index: &Index, condition: &QueryVal) -> bool {
    index.members.contains(&condition.key)
}

// Function to check if an Index can be used based on QueryVal keys
pub fn index_matches_query_ops(index: &Index, query_ops: &QueryOps) -> bool {
    match query_ops {
        QueryOps::And(ops) => ops.iter().all(|op| index_matches_query_ops(index, op)),
        QueryOps::Or(ops) => ops.iter().any(|op| index_matches_query_ops(index, op)),
        QueryOps::Condition(cond) => index_matches_condition(index, cond),
    }
}

// Function to check if an Index can be used to satisfy a subset of the query keys
pub fn index_is_subset_of_query(index: &Index, query_vals: &[QueryVal]) -> bool {
    let query_keys: Vec<&String> = query_vals.iter().map(|qv| &qv.key).collect();
    // Check if all members of the index are in the query keys
    index.members.iter().all(|member| query_keys.contains(&member)) &&
        // Ensure the index is not a superset of the query keys (i.e., it does not contain extra keys not in the query)
        index.members.len() <= query_keys.len()
}

// Function to get all matching Indexes for a given QueryOps with strict key matching
pub fn matching_indexes_for_query(indexes: &[Index], query_ops: &QueryOps) -> Vec<Index> {
    let query_vals = extract_query_vals(query_ops);

    indexes
        .iter()
        .cloned()
        .filter(|index| index_is_subset_of_query(index, &query_vals))
        .collect()
}

// Function to extract all QueryVals from a QueryOps structure
fn extract_query_vals(query_ops: &QueryOps) -> Vec<QueryVal> {
    match query_ops {
        QueryOps::And(ops) | QueryOps::Or(ops) => ops.iter().flat_map(extract_query_vals).collect(),
        QueryOps::Condition(cond) => vec![cond.clone()],
    }
}

#[cfg(test)]
mod tests {
    use crate::ops::query_ops::{FilterType, QueryOps, QueryVal};
    use crate::utils::index_utils::{matches_index, matching_indexes, matching_indexes_for_query};
    use schemajs_index::composite_key::CompositeKey;
    use schemajs_index::index_type::IndexType;
    use schemajs_primitives::column::types::DataValue;
    use schemajs_primitives::index::Index;

    #[test]
    fn test_matches_index_full_match() {
        let index = Index {
            name: "indx_first_name_and_email".to_string(),
            members: vec!["first_name".to_string(), "email".to_string()],
            index_type: IndexType::Hash,
        };

        let composite_key = CompositeKey(vec![
            ("first_name".to_string(), "Juan".to_string()),
            ("email".to_string(), "some@email.com".to_string()),
        ]);

        assert!(matches_index(&index, &composite_key));
    }

    #[test]
    fn test_matches_index_partial_match_not_allowed() {
        let index = Index {
            name: "indx_first_name_and_email".to_string(),
            members: vec!["first_name".to_string(), "email".to_string()],
            index_type: IndexType::Hash,
        };

        let composite_key = CompositeKey(vec![("first_name".to_string(), "Juan".to_string())]);

        assert!(!matches_index(&index, &composite_key));
    }

    #[test]
    fn test_matches_index_single_column_match() {
        let index = Index {
            name: "indx_email".to_string(),
            members: vec!["email".to_string()],
            index_type: IndexType::Hash,
        };

        let composite_key = CompositeKey(vec![
            ("first_name".to_string(), "Juan".to_string()),
            ("email".to_string(), "some@email.com".to_string()),
        ]);

        assert!(matches_index(&index, &composite_key));
    }

    #[test]
    fn test_matching_indexes_multiple_matches() {
        let index1 = Index {
            name: "indx_first_name_and_email".to_string(),
            members: vec!["first_name".to_string(), "email".to_string()],
            index_type: IndexType::Hash,
        };

        let index2 = Index {
            name: "indx_email".to_string(),
            members: vec!["email".to_string()],
            index_type: IndexType::Hash,
        };

        let indexes = vec![index1.clone(), index2.clone()];

        let composite_key = CompositeKey(vec![
            ("first_name".to_string(), "Juan".to_string()),
            ("email".to_string(), "some@email.com".to_string()),
        ]);

        let matching = matching_indexes(&indexes, &composite_key);

        assert_eq!(matching, vec![index1, index2]);
    }

    #[test]
    fn test_matching_indexes_no_matches() {
        let index1 = Index {
            name: "indx_first_name_and_email".to_string(),
            members: vec!["first_name".to_string(), "email".to_string()],
            index_type: IndexType::Hash,
        };

        let index2 = Index {
            name: "indx_email".to_string(),
            members: vec!["email".to_string()],
            index_type: IndexType::Hash,
        };

        let indexes = vec![index1, index2];

        let composite_key = CompositeKey(vec![("phone".to_string(), "123456".to_string())]);

        let matching = matching_indexes(&indexes, &composite_key);

        assert!(matching.is_empty());
    }

    #[test]
    fn test_matching_indexes_partial_key_match() {
        let index1 = Index {
            name: "indx_first_name_and_email".to_string(),
            members: vec!["first_name".to_string(), "email".to_string()],
            index_type: IndexType::Hash,
        };

        let index2 = Index {
            name: "indx_email".to_string(),
            members: vec!["email".to_string()],
            index_type: IndexType::Hash,
        };

        let indexes = vec![index1.clone(), index2.clone()];

        let composite_key = CompositeKey(vec![("email".to_string(), "some@email.com".to_string())]);

        let matching = matching_indexes(&indexes, &composite_key);

        assert_eq!(matching, vec![index2]);
    }

    #[test]
    fn test_strict_and_query() {
        let index1 = Index {
            name: "indx_first_name_and_email".to_string(),
            members: vec!["first_name".to_string(), "email".to_string()],
            index_type: IndexType::Hash,
        };

        let index2 = Index {
            name: "indx_email".to_string(),
            members: vec!["email".to_string()],
            index_type: IndexType::Hash,
        };

        let indexes = vec![index1.clone(), index2.clone()];

        let query = QueryOps::And(vec![
            QueryOps::Condition(QueryVal {
                key: "first_name".to_string(),
                filter_type: FilterType::Equal,
                value: DataValue::String("Juan".to_string()),
            }),
            QueryOps::Condition(QueryVal {
                key: "email".to_string(),
                filter_type: FilterType::Equal,
                value: DataValue::String("some@email.com".to_string()),
            }),
        ]);

        let matching_indexes = matching_indexes_for_query(&indexes, &query);

        // Expecting both index1 and index2 to match because index1 has both "first_name" and "email",
        // and index2 strictly matches the "email" key in the query.
        assert_eq!(matching_indexes, vec![index1, index2]);
    }

    #[test]
    fn test_subset_matching_query() {
        let index1 = Index {
            name: "indx_first_name".to_string(),
            members: vec!["first_name".to_string()],
            index_type: IndexType::Hash,
        };

        let index2 = Index {
            name: "indx_email".to_string(),
            members: vec!["email".to_string()],
            index_type: IndexType::Hash,
        };

        let indexes = vec![index1.clone(), index2.clone()];

        let query = QueryOps::And(vec![
            QueryOps::Condition(QueryVal {
                key: "first_name".to_string(),
                filter_type: FilterType::Equal,
                value: DataValue::String("Juan".to_string()),
            }),
            QueryOps::Condition(QueryVal {
                key: "email".to_string(),
                filter_type: FilterType::Equal,
                value: DataValue::String("some@email.com".to_string()),
            }),
        ]);

        let matching_indexes = matching_indexes_for_query(&indexes, &query);

        // Both index1 and index2 should match because they are subsets of the query keys.
        assert_eq!(matching_indexes, vec![index1, index2]);
    }

    #[test]
    fn test_no_superset_matching() {
        let index1 = Index {
            name: "indx_first_name_and_email".to_string(),
            members: vec!["first_name".to_string(), "email".to_string()],
            index_type: IndexType::Hash,
        };

        let index2 = Index {
            name: "indx_email".to_string(),
            members: vec!["email".to_string()],
            index_type: IndexType::Hash,
        };

        let indexes = vec![index1.clone(), index2.clone()];

        let query = QueryOps::And(vec![QueryOps::Condition(QueryVal {
            key: "email".to_string(),
            filter_type: FilterType::Equal,
            value: DataValue::String("some@email.com".to_string()),
        })]);

        let matching_indexes = matching_indexes_for_query(&indexes, &query);

        // Only index2 should match because it's a subset of the query, while index1 should not match
        // since it requires both "first_name" and "email" but only "email" is present in the query.
        assert_eq!(matching_indexes, vec![index2]);
    }

    #[test]
    fn test_combined_and_or_query() {
        let index1 = Index {
            name: "indx_first_name".to_string(),
            members: vec!["first_name".to_string()],
            index_type: IndexType::Hash,
        };

        let index2 = Index {
            name: "indx_email".to_string(),
            members: vec!["email".to_string()],
            index_type: IndexType::Hash,
        };

        let index3 = Index {
            name: "indx_last_name".to_string(),
            members: vec!["last_name".to_string()],
            index_type: IndexType::Hash,
        };

        let indexes = vec![index1.clone(), index2.clone(), index3.clone()];

        let query = QueryOps::Or(vec![
            QueryOps::And(vec![
                QueryOps::Condition(QueryVal {
                    key: "first_name".to_string(),
                    filter_type: FilterType::Equal,
                    value: DataValue::String("Juan".to_string()),
                }),
                QueryOps::Condition(QueryVal {
                    key: "email".to_string(),
                    filter_type: FilterType::Equal,
                    value: DataValue::String("some@email.com".to_string()),
                }),
            ]),
            QueryOps::Condition(QueryVal {
                key: "last_name".to_string(),
                filter_type: FilterType::Equal,
                value: DataValue::String("Doe".to_string()),
            }),
        ]);

        let matching_indexes = matching_indexes_for_query(&indexes, &query);

        // Only index1, index2, and index3 should match.
        assert_eq!(matching_indexes, vec![index1, index2, index3]);
    }
}
