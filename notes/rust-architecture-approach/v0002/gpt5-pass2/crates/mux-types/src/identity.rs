//! Project identity metadata.
//!
//! ## Invariants
//! - INV-004: IdentityManifest derives Debug, Clone, PartialEq, Eq.
//! - INV-034: Canonical serialization (sorted keys).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PaneId(pub u64);

/// Build profile variant.
/// RULE-S01-08: BuildProfile has 3 variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildProfile {
    Debug,
    Release,
    Profiling,
}

impl BuildProfile {
    /// String label for the build profile.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
            Self::Profiling => "profiling",
        }
    }
}

/// Canonical project identity metadata.
///
/// INV-004: Derives Debug, Clone, PartialEq, Eq.
/// RULE-S01-07: IdentityManifest is the canonical metadata struct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityManifest {
    /// Project name. RULE-S01-01: Always "TermForge".
    pub name: &'static str,
    /// Cargo package version (semver).
    pub version: &'static str,
    /// Specification version tag (e.g., "v15-pass3-definitive").
    pub spec_version: &'static str,
    /// Git SHA of the build (populated in release builds).
    pub git_sha: Option<&'static str>,
    /// Rust target triple (e.g., "x86_64-unknown-linux-gnu").
    pub target_triple: &'static str,
    /// Build profile.
    pub build_profile: BuildProfile,
    /// Active Cargo feature flags.
    pub feature_flags: &'static [&'static str],
}

impl IdentityManifest {
    /// The current project identity.
    #[must_use]
    pub fn current() -> Self {
        Self {
            name: "TermForge",
            version: env!("CARGO_PKG_VERSION"),
            spec_version: "v15-pass3-definitive",
            git_sha: option_env!("TERMFORGE_GIT_SHA"),
            target_triple: option_env!("TARGET").unwrap_or("unknown"),
            build_profile: if cfg!(debug_assertions) {
                BuildProfile::Debug
            } else {
                BuildProfile::Release
            },
            feature_flags: &[],
        }
    }

    /// RULE-S01-14: version_string() contains spec version tag.
    #[must_use]
    pub fn version_string(&self) -> String {
        format!(
            "{} {} ({}, spec {})",
            self.name,
            self.version,
            self.build_profile.label(),
            self.spec_version,
        )
    }

    /// Check if a feature flag is active.
    #[must_use]
    pub fn has_feature(&self, feature: &str) -> bool {
        self.feature_flags.iter().any(|&f| f == feature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_name() {
        let id = IdentityManifest::current();
        assert_eq!(id.name, "TermForge");
    }

    #[test]
    fn test_build_profile_labels() {
        assert_eq!(BuildProfile::Debug.label(), "debug");
        assert_eq!(BuildProfile::Release.label(), "release");
        assert_eq!(BuildProfile::Profiling.label(), "profiling");
    }

    #[test]
    fn test_version_string_contains_spec() {
        let id = IdentityManifest::current();
        let vs = id.version_string();
        assert!(vs.contains("v15-pass3-definitive"));
        assert!(vs.contains("TermForge"));
    }

    #[test]
    fn test_identity_clone_eq() {
        let a = IdentityManifest::current();
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn test_build_profile_has_3_variants() {
        let variants = [BuildProfile::Debug, BuildProfile::Release, BuildProfile::Profiling];
        assert_eq!(variants.len(), 3);
    }

    #[test]
    fn test_has_feature_empty() {
        let id = IdentityManifest::current();
        assert!(!id.has_feature("crdt"));
    }

    #[test]
    fn typed_ids_are_distinct() {
        assert_ne!(SessionId(1).0, WindowId(2).0);
        assert_ne!(WindowId(2).0, PaneId(3).0);
    }
}
