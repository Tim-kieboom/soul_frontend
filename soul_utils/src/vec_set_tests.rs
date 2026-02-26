use crate::{vec_map::VecMapIndex, vec_set::VecSet};

// Simple index type for testing
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct TestIndex(usize);
impl VecMapIndex for TestIndex {
    fn new_index(value: usize) -> Self {
        TestIndex(value)
    }

    fn index(&self) -> usize {
        self.0
    }
}

// VecSet tests
type TestSet = VecSet<TestIndex>;

#[test]
fn test_vecset_new() {
    let set: TestSet = TestSet::new();
    assert_eq!(set.entries().count(), 0);
}

#[test]
fn test_vecset_insert() {
    let mut set: TestSet = TestSet::new();
    assert_eq!(set.insert(TestIndex(0)), None);
    assert!(set.contains(TestIndex(0)));
    assert_eq!(set.insert(TestIndex(0)), Some(()));
    assert!(set.contains(TestIndex(0)));
}

#[test]
fn test_vecset_remove() {
    let mut set: TestSet = TestSet::new();
    set.insert(TestIndex(1));
    assert!(set.contains(TestIndex(1)));
    assert_eq!(set.remove(TestIndex(1)), Some(()));
    assert!(!set.contains(TestIndex(1)));
}

#[test]
fn test_vecset_from_slice() {
    let slice = vec![TestIndex(2), TestIndex(0), TestIndex(2)];
    let set = TestSet::from_slice(&slice);
    assert_eq!(set.entries().count(), 2);
    assert!(set.contains(TestIndex(0)));
    assert!(set.contains(TestIndex(2)));
}

#[test]
fn test_vecset_index() {
    let mut set: TestSet = TestSet::new();
    set.insert(TestIndex(5));
    assert_eq!(set[TestIndex(5)], true);
    assert_eq!(set[TestIndex(0)], false);
}

#[test]
fn test_vecset_serde() {
    let map = TestSet::from_vec(vec![(TestIndex(0)), (TestIndex(1)), (TestIndex(10))]);
    let json = serde_json::to_value(&map).unwrap();
    let new_map: TestSet = serde_json::from_value(json).unwrap();

    assert_eq!(map, new_map)
}
