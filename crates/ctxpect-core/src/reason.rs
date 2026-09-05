//! Unknown reason codes.
//!
//! When Contexpect cannot establish a fact, it must say *why* in a closed
//! vocabulary rather than degrade silently. These codes are frozen by
//! `acceptance/claim-validity-matrix.yaml`.

use std::fmt;

macro_rules! reason_codes {
    ( $( $variant:ident => $wire:literal ),+ $(,)? ) => {
        /// Why a claim is `Indeterminate` rather than `Present` or `Absent`.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum UnknownReason {
            $( $variant ),+
        }

        impl UnknownReason {
            /// Every code, in the order the frozen contract declares them.
            pub const ALL: &'static [UnknownReason] = &[ $( UnknownReason::$variant ),+ ];

            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $( UnknownReason::$variant => $wire ),+
                }
            }

            /// Unknown codes fail closed: an unrecognised reason is not a reason.
            #[must_use]
            pub fn from_wire(text: &str) -> Option<UnknownReason> {
                match text {
                    $( $wire => Some(UnknownReason::$variant), )+
                    _ => None,
                }
            }
        }

        impl fmt::Display for UnknownReason {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

reason_codes! {
    SurfaceNotExposed => "surface_not_exposed",
    UnsupportedHarnessVersion => "unsupported_harness_version",
    PermissionNotGranted => "permission_not_granted",
    RuntimeSnapshotMissing => "runtime_snapshot_missing",
    CloudSettingUnavailable => "cloud_setting_unavailable",
    DynamicAgentSelection => "dynamic_agent_selection",
    ToolSchemaNotExported => "tool_schema_not_exported",
    CurrentOccupancyNotReported => "current_occupancy_not_reported",
    ContentRedactedByPolicy => "content_redacted_by_policy",
    ImportParseFailed => "import_parse_failed",
    EvidenceStale => "evidence_stale",
    NotInstalled => "not_installed",
    ConnectorRequired => "connector_required",
    ConfigResidueOnly => "config_residue_only",
    AuthenticationUnavailable => "authentication_unavailable",
    SandboxUnavailable => "sandbox_unavailable",
    ConfiguredModelUnsupported => "configured_model_unsupported",
    AttachmentUnavailable => "attachment_unavailable",
    OfficialDistributionNotCaptured => "official_distribution_not_captured",
    HermeticFixtureOnly => "hermetic_fixture_only",
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip() {
        for code in UnknownReason::ALL {
            assert_eq!(UnknownReason::from_wire(code.as_str()), Some(*code));
        }
    }

    #[test]
    fn unknown_codes_fail_closed() {
        assert_eq!(UnknownReason::from_wire("surface-not-exposed"), None);
        assert_eq!(UnknownReason::from_wire("because"), None);
        assert_eq!(UnknownReason::from_wire(""), None);
    }

    #[test]
    fn codes_are_unique() {
        let mut seen: Vec<&str> = UnknownReason::ALL.iter().map(|c| c.as_str()).collect();
        let total = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), total, "duplicate reason code");
    }
}
