//! Defines diagnostics for parser.

#![allow(clippy::needless_pass_by_value)]

use std::fmt::Display;

use ry_ast::{
    token::{LexError, Token},
    ModuleItemKind,
};
use ry_diagnostics::diagnostic::Diagnostic;
use ry_diagnostics::{BuildDiagnostic, LocationExt};
use ry_english_commons::enumeration;
use ry_filesystem::location::{ByteOffset, Location};
use ry_interner::PathID;

/// Represents list of expected tokens.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Expected(pub Vec<String>);

/// Allows to construct [`Expected`] object shorter:
///
/// ```
/// use ry_parser::{expected, diagnostics::Expected};
///
/// assert_eq!(expected!("a", "b"), Expected(vec!["a".to_owned(), "b".to_owned()]));
/// ```
#[macro_export]
macro_rules! expected {
    ($($e:expr),*) => {{
        $crate::diagnostics::Expected(vec![$(format!("{}", $e)),*])
    }};
}

/// Context in which the unnecessary visibility qualifier error is found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnnecessaryVisibilityQualifierContext {
    /// ```ry
    /// pub interface F {
    ///     pub fun t() {}
    ///     ^^^
    /// }
    /// ```
    InterfaceMethod {
        /// Location of a method name.
        name_location: Location,
    },

    /// ```ry
    /// pub import ...;
    /// ^^^
    /// ```
    Import,
}

/// Diagnostic related to an error occured when tokenizing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LexErrorDiagnostic(pub LexError);

impl BuildDiagnostic for LexErrorDiagnostic {
    #[inline(always)]
    fn build(self) -> Diagnostic<PathID> {
        Diagnostic::error()
            .with_message(self.0.raw.to_string())
            .with_code("E000")
            .with_labels(vec![self.0.location.to_primary_label()])
    }
}

/// Diagnostic related to an unexpected token error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnexpectedTokenDiagnostic {
    /// End byte offset of the token before unexpected one.
    pub offset: Option<ByteOffset>,

    /// The token that was not expected.
    pub got: Token,

    /// Tokens that were expected.
    pub expected: Expected,

    /// AST Node at which the error occurred while parsing.
    pub node: String,
}

impl UnexpectedTokenDiagnostic {
    /// Creates a new instance of [`UnexpectedTokenDiagnostic`].
    #[inline(always)]
    #[must_use]
    pub fn new(
        offset: Option<ByteOffset>,
        got: Token,
        expected: Expected,
        node: impl ToString,
    ) -> Self {
        Self {
            offset,
            got,
            expected,
            node: node.to_string(),
        }
    }
}

impl BuildDiagnostic for UnexpectedTokenDiagnostic {
    #[inline(always)]
    fn build(self) -> Diagnostic<PathID> {
        Diagnostic::error()
            .with_message(format!(
                "expected {}, found {}",
                self.expected, self.got.raw
            ))
            .with_code("E001")
            .with_labels(if let Some(offset) = self.offset {
                vec![
                    offset
                        .next_byte_location_at(self.got.location.file_path_id)
                        .to_secondary_label()
                        .with_message(format!("expected {}", self.expected)),
                    self.got
                        .location
                        .to_primary_label()
                        .with_message(format!("found {}", self.got.raw)),
                ]
            } else {
                vec![self
                    .got
                    .location
                    .to_primary_label()
                    .with_message(format!("expected {} for {}", self.expected, self.node))]
            })
    }
}

/// Diagnostic related to ana integer overflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntegerOverflowDiagnostic {
    /// Location of number when parsing which, overflow happened.
    pub location: Location,
}

impl BuildDiagnostic for IntegerOverflowDiagnostic {
    #[inline(always)]
    fn build(self) -> Diagnostic<PathID> {
        Diagnostic::error()
            .with_message("unexpected integer overflow".to_owned())
            .with_code("E002")
            .with_labels(vec![self.location.to_primary_label()
                .with_message("error appeared when parsing this integer")])
            .with_notes(vec![
                "note: integer cannot exceed the maximum value of `u64` (u64.max() == 18_446_744_073_709_551_615)".to_owned(),
                "note: you can use exponent to do so, but be careful!".to_owned()
            ])
    }
}

/// Diagnostic related to a float overflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatOverflowDiagnostic {
    /// Location of number when parsing which, overflow happened.
    pub location: Location,
}

impl BuildDiagnostic for FloatOverflowDiagnostic {
    #[inline(always)]
    fn build(self) -> Diagnostic<PathID> {
        Diagnostic::error()
            .with_message("unexpected float overflow".to_owned())
            .with_code("E003")
            .with_labels(vec![self.location.to_primary_label()
                .with_message("error appeared when parsing this float literal")
            ])
            .with_notes(vec![
                "note: float literal cannot exceed the maximum value of `f64` (f64.max() == 1.7976931348623157E+308)".to_owned(),
                "note: you can use exponent to do so, but be careful, especially when working with floats!".to_owned()
            ])
    }
}

/// Diagnostic related to an unnecessary visibility qualifier error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnnecessaryVisibilityQualifierDiagnostic {
    /// Location of `pub`.
    pub location: Location,

    /// Context in which the error is found.
    pub context: UnnecessaryVisibilityQualifierContext,
}

impl BuildDiagnostic for UnnecessaryVisibilityQualifierDiagnostic {
    #[inline(always)]
    fn build(self) -> Diagnostic<PathID> {
        let mut labels = vec![self
            .location
            .to_primary_label()
            .with_message("consider removing this `pub`")];

        if let UnnecessaryVisibilityQualifierContext::InterfaceMethod { name_location } =
            self.context
        {
            labels.push(
                name_location
                    .to_secondary_label()
                    .with_message("happened when analyzing the interface method"),
            );
        }

        Diagnostic::error()
            .with_message("unnecessary visibility qualifier".to_owned())
            .with_code("E004")
            .with_labels(labels)
            .with_notes(match self.context {
                UnnecessaryVisibilityQualifierContext::InterfaceMethod { .. } => {
                    vec![
                        "note: using `pub` for interface method will not make the method public"
                            .to_owned(),
                        "note: all interface methods are public by default".to_owned(),
                    ]
                }
                UnnecessaryVisibilityQualifierContext::Import => {
                    vec!["note: using `pub` will not make the import public.".to_owned()]
                }
            })
    }
}

/// Diagnostic related to an EOF instead of close brace error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EOFInsteadOfCloseBrace {
    /// Type of item in which error occurred.
    pub item_kind: ModuleItemKind,

    /// Location of item name.
    pub item_location: Location,

    /// EOF token location.
    pub location: Location,
}

impl BuildDiagnostic for EOFInsteadOfCloseBrace {
    #[inline(always)]
    fn build(self) -> Diagnostic<PathID> {
        Diagnostic::error()
            .with_message("unexpected end of file".to_owned())
            .with_code("E001")
            .with_labels(vec![
                self.item_location
                    .to_primary_label()
                    .with_message(format!("happened when parsing this {}", self.item_kind)),
                self.location
                    .to_secondary_label()
                    .with_message("consider adding `}`".to_owned()),
            ])
    }
}

impl Display for Expected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&enumeration::one_of(self.0.iter(), false))
    }
}
