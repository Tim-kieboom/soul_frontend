use std::fmt::Display;

use crate::span::Span;

#[derive(
    Debug,
    Clone,
    Default,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum Severity {
    Note = 0,
    Warning = 1,
    #[default]
    Error = 2,
}

/// Fallback fault kind for call sites that have not yet been migrated to a
/// structured, crate-specific error-kind enum. Carries the old free-text
/// message verbatim so `.message()` keeps working unchanged during migration.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct UnclassifiedKind(pub Box<str>);

impl Display for UnclassifiedKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<Box<str>> for UnclassifiedKind {
    fn from(value: Box<str>) -> Self {
        UnclassifiedKind(value)
    }
}

pub trait FromFaultKind<Kind> {
    fn from_kind(value: Kind) -> Self;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Fault<K = UnclassifiedKind> {
    severity: Severity,
    kind: K,
    span: Option<Span>,

    #[cfg(feature = "error_backtrace")]
    backtrace: Box<str>,
}

impl<K> Fault<K> {
    pub fn severity(&self) -> Severity {
        self.severity
    }

    pub fn span(&self) -> Option<Span> {
        self.span
    }

    pub fn kind(&self) -> &K {
        &self.kind
    }

    pub fn into_kind<K2>(self) -> Fault<K2>
    where
        K: Into<K2>,
    {
        Fault {
            severity: self.severity,
            kind: self.kind.into(),
            span: self.span,
            #[cfg(feature = "error_backtrace")]
            backtrace: self.backtrace,
        }
    }
}

impl<K: Display> Fault<K> {
    pub fn error_with_kind(kind: K, span: Option<Span>) -> Self {
        Fault::new(Severity::Error, kind, span)
    }

    pub fn warning_with_kind(kind: K, span: Option<Span>) -> Self {
        Fault::new(Severity::Warning, kind, span)
    }

    pub fn note_with_kind(kind: K, span: Option<Span>) -> Self {
        Fault::new(Severity::Note, kind, span)
    }

    #[cfg(feature = "error_backtrace")]
    fn new(severity: Severity, kind: K, span: Option<Span>) -> Self {
        use std::backtrace::Backtrace;

        Fault {
            backtrace: Backtrace::force_capture().to_string().into_boxed_str(),
            severity,
            kind,
            span,
        }
    }

    #[cfg(not(feature = "error_backtrace"))]
    fn new(severity: Severity, kind: K, span: Option<Span>) -> Self {
        Fault {
            severity,
            kind,
            span,
        }
    }

    pub fn message(&self) -> String {
        self.kind.to_string()
    }

    #[cfg(feature = "error_backtrace")]
    pub fn backtract(&self) -> &str {
        &self.backtrace
    }
}

/// Convenience constructors for unmigrated call sites: same signatures as
/// before this type went generic, just producing `Fault<UnclassifiedKind>`.
impl Fault<UnclassifiedKind> {
    pub fn error(message: impl Into<Box<str>>, span: Option<Span>) -> Self {
        Fault::error_with_kind(UnclassifiedKind(message.into()), span)
    }

    pub fn warning(message: impl Into<Box<str>>, span: Option<Span>) -> Self {
        Fault::warning_with_kind(UnclassifiedKind(message.into()), span)
    }

    pub fn note(message: impl Into<Box<str>>, span: Option<Span>) -> Self {
        Fault::note_with_kind(UnclassifiedKind(message.into()), span)
    }

    pub fn empty() -> Self {
        Fault::error_with_kind(UnclassifiedKind(Box::from("")), None)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FaultCollector<K = UnclassifiedKind> {
    pub faults: Vec<Fault<K>>,
}

impl<K> Default for FaultCollector<K> {
    fn default() -> Self {
        FaultCollector { faults: Vec::new() }
    }
}

impl<K> FaultCollector<K> {
    pub fn push(&mut self, fault: Fault<K>) {
        self.faults.push(fault);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Fault<K>> {
        self.faults.iter()
    }

    pub fn count_severity(&self, severity: Severity) -> usize {
        self.faults
            .iter()
            .filter(|f| f.severity() == severity)
            .count()
    }

    pub fn fails(&self, fail_level: Severity) -> bool {
        self.faults.iter().any(|d| d.severity == fail_level)
    }

    pub fn into_unclassified(self) -> FaultCollector<UnclassifiedKind>
    where
        K: Into<UnclassifiedKind>,
    {
        FaultCollector {
            faults: self.faults.into_iter().map(Fault::into_kind).collect(),
        }
    }
}
impl<K> IntoIterator for FaultCollector<K> {
    type Item = Fault<K>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.faults.into_iter()
    }
}
impl FaultCollector<UnclassifiedKind> {
    pub fn extend_into<K>(&mut self, faults: FaultCollector<K>)
    where
        K: Into<UnclassifiedKind>,
    {
        for fault in faults.into_iter() {
            self.push(fault.into_kind());
        }
    }
}

impl<K: Display> FaultCollector<K> {
    pub fn push_error_with_kind(&mut self, kind: K, span: Option<Span>) {
        self.faults.push(Fault::error_with_kind(kind, span));
    }

    pub fn push_warning_with_kind(&mut self, kind: K, span: Option<Span>) {
        self.faults.push(Fault::warning_with_kind(kind, span));
    }

    pub fn push_note_with_kind(&mut self, kind: K, span: Option<Span>) {
        self.faults.push(Fault::note_with_kind(kind, span));
    }
}

impl FaultCollector<UnclassifiedKind> {
    pub fn push_error(&mut self, message: impl Into<Box<str>>, span: Option<Span>) {
        self.faults.push(Fault::error(message, span));
    }

    pub fn push_warning(&mut self, message: impl Into<Box<str>>, span: Option<Span>) {
        self.faults.push(Fault::warning(message, span));
    }

    pub fn push_note(&mut self, message: impl Into<Box<str>>, span: Option<Span>) {
        self.faults.push(Fault::note(message, span));
    }
}
