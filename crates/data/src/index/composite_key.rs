use std::cmp::Ordering;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct CompositeKey(Vec<(String, String)>);

impl Ord for CompositeKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for CompositeKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
