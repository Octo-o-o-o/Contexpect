//! The boundary between this parser and Python's `json`, as data.
//!
//! # What the tests in `tests/boundary_doc.rs` observe
//!
//! - `BOUNDARY.md` equals `render_markdown()` byte for byte.
//! - Each row's `we_accept` equals what `crate::parse` does with `document`.
//! - Each row's `python_accepts` equals what `json.loads` does with `document`.
//! - Each rendered section's membership equals the set computed from the row fields.
//! - Each row's `rationale` is one the row's accept/reject pattern permits.
//! - `src/lib.rs` carries no hand-written prose about the boundary.
//! - Seven documents named by earlier reviews are still present.
//!
//! # What they do not observe
//!
//! - **Whether a [`Rationale`] sentence is true.** [`Rationale::permits`] checks
//!   which variant a row may cite, given its accept/reject pattern. It does not
//!   check what the sentence says. Editing [`Rationale::as_prose`] to something
//!   false still passes; review-5 demonstrated exactly that against the previous
//!   free-text field, and this change narrows the surface from per-row prose to
//!   seven fixed sentences rather than eliminating it. Machine-checking whether a
//!   sentence is true is not something a test here can do.
//! - Nothing outside this crate. Other crates' documentation is unguarded.
//! - Whether [`BOUNDARY`] is complete against the two parsers. The rows are the
//!   cases someone wrote down; completeness is established by differential testing
//!   during review, not by a test here.

/// Why a row behaves as it does.
///
/// A closed set rather than free text: review-5 found an earlier `note: &str`
/// field carrying arbitrary prose into the published documentation, including a
/// promise a previous review had already found untrue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rationale {
    /// Shortest-representation algorithms disagree on rounding ties.
    FloatRoundingTies,
    /// A float literal, rejected along with all floats.
    FloatLiteral,
    /// Outside the integer range this crate represents.
    IntegerBeyondI64,
    /// A literal Python accepts as an extension but JSON does not define.
    NonJsonLiteral,
    /// A code point Rust's `String` cannot hold.
    LoneSurrogate,
    /// Malformed under the JSON grammar itself.
    InvalidJsonSyntax,
    /// The subset both sides agree on.
    SupportedSubset,
}

impl Rationale {
    /// The sentence rendered into the documentation for this variant.
    #[must_use]
    pub const fn as_prose(self) -> &'static str {
        match self {
            Rationale::FloatRoundingTies => {
                "Python's shortest representation breaks rounding ties to even, Rust's breaks away \
                 from zero, so the same float can render differently and digest differently."
            }
            Rationale::FloatLiteral => "A float literal; floats are not supported.",
            Rationale::IntegerBeyondI64 => {
                "Outside `i64`. Python has arbitrary precision integers; this crate does not."
            }
            Rationale::NonJsonLiteral => {
                "Not defined by JSON. Python accepts it as an extension; this crate does not."
            }
            Rationale::LoneSurrogate => {
                "A lone surrogate. Python yields a `str` holding it; Rust's `String` cannot."
            }
            Rationale::InvalidJsonSyntax => "Malformed under the JSON grammar.",
            Rationale::SupportedSubset => {
                "Objects, arrays, strings, `i64` integers, booleans and null. Within this subset \
                 the canonical form and its digest match the acceptance generator."
            }
        }
    }

    /// Whether this rationale may be used by a row with the given behaviour.
    ///
    /// Keeps a row from citing, say, `SupportedSubset` for something it rejects.
    #[must_use]
    pub const fn permits(self, we_accept: bool, python_accepts: bool) -> bool {
        match self {
            // We reject, Python accepts.
            Rationale::FloatRoundingTies
            | Rationale::FloatLiteral
            | Rationale::IntegerBeyondI64
            | Rationale::NonJsonLiteral
            | Rationale::LoneSurrogate => !we_accept && python_accepts,
            // Neither side accepts malformed input.
            Rationale::InvalidJsonSyntax => !we_accept && !python_accepts,
            // Both sides accept the supported subset.
            Rationale::SupportedSubset => we_accept && python_accepts,
        }
    }
}

/// One documented case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundaryCase {
    /// A complete JSON document exercising the case.
    pub document: &'static str,
    /// Whether `crate::parse` accepts it.
    pub we_accept: bool,
    /// Whether Python's `json.loads` accepts it.
    pub python_accepts: bool,
    /// Why, from the closed set.
    pub rationale: Rationale,
}

impl BoundaryCase {
    /// True when the two implementations disagree about this document.
    #[must_use]
    pub const fn is_divergence(&self) -> bool {
        self.we_accept != self.python_accepts
    }

    /// Which rendered section this row belongs in, derived from its fields alone.
    #[must_use]
    pub const fn section(&self) -> Section {
        if self.we_accept {
            Section::AcceptedHere
        } else if self.python_accepts {
            Section::RejectedHereAcceptedByPython
        } else {
            Section::RejectedByBoth
        }
    }
}

/// The rendered sections. Membership is computed, never chosen by hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    AcceptedHere,
    RejectedHereAcceptedByPython,
    RejectedByBoth,
}

impl Section {
    pub const ALL: &'static [Section] = &[
        Section::AcceptedHere,
        Section::RejectedHereAcceptedByPython,
        Section::RejectedByBoth,
    ];

    #[must_use]
    pub const fn heading(self) -> &'static str {
        match self {
            Section::AcceptedHere => "## Accepted by both",
            Section::RejectedHereAcceptedByPython => "## Rejected here, accepted by Python",
            Section::RejectedByBoth => "## Rejected by both",
        }
    }
}

/// Every documented case.
pub const BOUNDARY: &[BoundaryCase] = &[
    BoundaryCase {
        document: r#"{"v":1000000000000000.25}"#,
        we_accept: false,
        python_accepts: true,
        rationale: Rationale::FloatRoundingTies,
    },
    BoundaryCase {
        document: r#"{"v":1.0}"#,
        we_accept: false,
        python_accepts: true,
        rationale: Rationale::FloatLiteral,
    },
    BoundaryCase {
        document: r#"{"v":1e16}"#,
        we_accept: false,
        python_accepts: true,
        rationale: Rationale::FloatLiteral,
    },
    BoundaryCase {
        document: r#"{"v":9223372036854775808}"#,
        we_accept: false,
        python_accepts: true,
        rationale: Rationale::IntegerBeyondI64,
    },
    BoundaryCase {
        document: r#"{"v":NaN}"#,
        we_accept: false,
        python_accepts: true,
        rationale: Rationale::NonJsonLiteral,
    },
    BoundaryCase {
        document: r#"{"v":Infinity}"#,
        we_accept: false,
        python_accepts: true,
        rationale: Rationale::NonJsonLiteral,
    },
    BoundaryCase {
        document: r#"{"v":"\ud800"}"#,
        we_accept: false,
        python_accepts: true,
        rationale: Rationale::LoneSurrogate,
    },
    BoundaryCase {
        document: r#"{"v":01}"#,
        we_accept: false,
        python_accepts: false,
        rationale: Rationale::InvalidJsonSyntax,
    },
    BoundaryCase {
        document: r#"{"v":"\u+041"}"#,
        we_accept: false,
        python_accepts: false,
        rationale: Rationale::InvalidJsonSyntax,
    },
    BoundaryCase {
        document: r#"{"v":1.}"#,
        we_accept: false,
        python_accepts: false,
        rationale: Rationale::InvalidJsonSyntax,
    },
    BoundaryCase {
        document: r#"{"v":0.e5}"#,
        we_accept: false,
        python_accepts: false,
        rationale: Rationale::InvalidJsonSyntax,
    },
    BoundaryCase {
        document: "{\"v\":\"a\u{1}b\"}",
        we_accept: false,
        python_accepts: false,
        rationale: Rationale::InvalidJsonSyntax,
    },
    BoundaryCase {
        document: r#"{"v":1,}"#,
        we_accept: false,
        python_accepts: false,
        rationale: Rationale::InvalidJsonSyntax,
    },
    BoundaryCase {
        document: r#"{'v':1}"#,
        we_accept: false,
        python_accepts: false,
        rationale: Rationale::InvalidJsonSyntax,
    },
    BoundaryCase {
        document: r#"{"v":[1,{"k":"值"},null,true,-7]}"#,
        we_accept: true,
        python_accepts: true,
        rationale: Rationale::SupportedSubset,
    },
];

/// Render [`BOUNDARY`] as the crate's documentation section.
///
/// Section membership comes from [`BoundaryCase::section`], and the accept/reject
/// wording from the boolean fields; only [`Rationale::as_prose`] is a fixed string.
#[must_use]
pub fn render_markdown() -> String {
    let mut out = String::new();
    out.push_str("# Boundary against Python's `json`\n\n");
    out.push_str(
        "*Generated from `BOUNDARY` in `src/boundary.rs` by `cargo run -p ctxpect-schema \
         --example render_boundary`. Editing this file directly fails `cargo test`.*\n",
    );

    for section in Section::ALL {
        out.push('\n');
        out.push_str(section.heading());
        out.push_str("\n\n");
        for case in BOUNDARY.iter().filter(|c| c.section() == *section) {
            // The behaviour half of each line is derived, not written.
            let behaviour = match (case.we_accept, case.python_accepts) {
                (true, true) => "accepted by both",
                (false, true) => "rejected here, accepted by Python",
                (false, false) => "rejected by both",
                (true, false) => "accepted here, rejected by Python",
            };
            out.push_str(&format!(
                "- `{}` — {} — {}\n",
                case.document,
                behaviour,
                case.rationale.as_prose()
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_is_distinct() {
        let mut documents: Vec<&str> = BOUNDARY.iter().map(|c| c.document).collect();
        let total = documents.len();
        documents.sort_unstable();
        documents.dedup();
        assert_eq!(documents.len(), total, "duplicate boundary document");
    }

    #[test]
    fn every_row_uses_a_rationale_its_behaviour_permits() {
        for case in BOUNDARY {
            assert!(
                case.rationale.permits(case.we_accept, case.python_accepts),
                "{} is {}/{} which {:?} does not permit",
                case.document,
                case.we_accept,
                case.python_accepts,
                case.rationale
            );
        }
    }

    #[test]
    fn section_is_derived_from_the_fields() {
        for case in BOUNDARY {
            let expected = if case.we_accept {
                Section::AcceptedHere
            } else if case.python_accepts {
                Section::RejectedHereAcceptedByPython
            } else {
                Section::RejectedByBoth
            };
            assert_eq!(case.section(), expected, "{}", case.document);
        }
    }

    #[test]
    fn all_three_sections_are_populated() {
        for section in Section::ALL {
            assert!(
                BOUNDARY.iter().any(|c| c.section() == *section),
                "{:?} has no rows",
                section
            );
        }
    }

    #[test]
    fn permits_rejects_mismatched_pairings() {
        // The guard is only meaningful if it can say no.
        assert!(!Rationale::SupportedSubset.permits(false, true));
        assert!(!Rationale::FloatLiteral.permits(true, true));
        assert!(!Rationale::InvalidJsonSyntax.permits(false, true));
        assert!(Rationale::InvalidJsonSyntax.permits(false, false));
    }
}
