use serde::{Deserialize, Serialize};

/// Byte and human-readable location in textual source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSpan {
    /// Inclusive byte offset.
    pub start: usize,
    /// Exclusive byte offset.
    pub end: usize,
    /// One-based line number, or zero when the source has no textual span.
    pub line: usize,
    /// One-based column number, or zero when unavailable.
    pub column: usize,
}

impl SourceSpan {
    /// Span used for JSON-projected values without an individual textual token.
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            start: 0,
            end: 0,
            line: 0,
            column: 0,
        }
    }

    /// Combines two source spans.
    #[must_use]
    pub fn join(self, other: Self) -> Self {
        if self.line == 0 {
            return other;
        }
        if other.line == 0 {
            return self;
        }
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            line: self.line,
            column: self.column,
        }
    }
}
