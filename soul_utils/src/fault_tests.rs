use std::fmt::Display;

use crate::fault::{Fault, FaultCollector, Severity, UnclassifiedKind};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
enum TestErrorKind {
    Wrong(Box<str>),
    Warned,
}

impl Display for TestErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestErrorKind::Wrong(msg) => write!(f, "something went wrong: {msg}"),
            TestErrorKind::Warned => write!(f, "a warning"),
        }
    }
}

impl From<TestErrorKind> for UnclassifiedKind {
    fn from(value: TestErrorKind) -> Self {
        UnclassifiedKind(value.to_string().into_boxed_str())
    }
}

#[test]
fn into_unclassified_preserves_message_severity_and_span() {
    let mut collector = FaultCollector::<TestErrorKind>::default();
    collector.push(Fault::error_with_kind(
        TestErrorKind::Wrong("oops".into()),
        None,
    ));
    collector.push(Fault::warning_with_kind(TestErrorKind::Warned, None));

    let unclassified = collector.into_unclassified();

    assert_eq!(unclassified.faults.len(), 2);
    assert_eq!(
        unclassified.faults[0].message(),
        "something went wrong: oops"
    );
    assert_eq!(unclassified.faults[0].severity(), Severity::Error);
    assert_eq!(unclassified.faults[1].message(), "a warning");
    assert_eq!(unclassified.faults[1].severity(), Severity::Warning);
}

#[test]
fn into_unclassified_on_an_empty_collector_stays_empty() {
    let collector = FaultCollector::<TestErrorKind>::default();
    let unclassified = collector.into_unclassified();
    assert!(unclassified.faults.is_empty());
}

#[test]
fn faults_from_two_differently_typed_collectors_can_be_combined() {
    let mut first = FaultCollector::<TestErrorKind>::default();
    first.push(Fault::error_with_kind(TestErrorKind::Warned, None));

    let mut second = FaultCollector::<UnclassifiedKind>::default();
    second.push_error("a plain message", None);

    let mut combined = first.into_unclassified();
    combined.faults.extend(second.into_unclassified().faults);

    assert_eq!(combined.faults.len(), 2);
    assert_eq!(combined.count_severity(Severity::Error), 2);
}
