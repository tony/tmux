use serde::{Deserialize, Serialize};

/// Build profile for runtime identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildProfile {
    Debug,
    Release,
    Profiling,
}

/// Canonical project metadata (S1/S91+ and INV-034).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityManifest {
    pub name: String,
    pub version: String,
    pub edition: String,
    pub msrv: String,
    pub license: String,
    pub build_profile: BuildProfile,
    pub git_sha: Option<String>,
    pub target_triple: String,
    pub feature_flags: Vec<String>,
    pub protocol_version: u8,
}

impl IdentityManifest {
    pub fn default_manifest() -> Self {
        Self {
            name: "termforge".to_string(),
            version: "0.1.0".to_string(),
            edition: "2021".to_string(),
            msrv: "1.75.0".to_string(),
            license: "MIT OR Apache-2.0".to_string(),
            build_profile: BuildProfile::Debug,
            git_sha: None,
            target_triple: std::env::consts::ARCH.to_string(),
            feature_flags: Vec::new(),
            protocol_version: 8,
        }
    }

    pub fn has_feature(&self, feature: &str) -> bool {
        self.feature_flags.iter().any(|f| f == feature)
    }

    pub fn version_string(&self) -> String {
        format!(
            "{} {}+v15 (protocol v{}, edition {})",
            self.name, self.version, self.protocol_version, self.edition
        )
    }

    /// Deterministic JSON-like representation with stable key order.
    pub fn canonical_json(&self) -> String {
        let mut features = self.feature_flags.clone();
        features.sort();
        let features = features
            .into_iter()
            .map(|f| format!("\"{}\"", escape_json(&f)))
            .collect::<Vec<_>>()
            .join(",");
        let git = match &self.git_sha {
            Some(v) => format!("\"{}\"", escape_json(v)),
            None => "null".to_string(),
        };
        format!(
            concat!(
                "{{",
                "\"build_profile\":\"{}\",",
                "\"edition\":\"{}\",",
                "\"feature_flags\":[{}],",
                "\"git_sha\":{},",
                "\"license\":\"{}\",",
                "\"msrv\":\"{}\",",
                "\"name\":\"{}\",",
                "\"protocol_version\":{},",
                "\"target_triple\":\"{}\",",
                "\"version\":\"{}\"",
                "}}"
            ),
            profile_name(&self.build_profile),
            escape_json(&self.edition),
            features,
            git,
            escape_json(&self.license),
            escape_json(&self.msrv),
            escape_json(&self.name),
            self.protocol_version,
            escape_json(&self.target_triple),
            escape_json(&self.version),
        )
    }
}

fn profile_name(profile: &BuildProfile) -> &'static str {
    match profile {
        BuildProfile::Debug => "debug",
        BuildProfile::Release => "release",
        BuildProfile::Profiling => "profiling",
    }
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_reasonable() {
        let m = IdentityManifest::default_manifest();
        assert_eq!(m.name, "termforge");
        assert_eq!(m.protocol_version, 8);
        assert!(m.target_triple.len() >= 3);
    }

    #[test]
    fn version_string_contains_required_tags() {
        let m = IdentityManifest::default_manifest();
        let s = m.version_string();
        assert!(s.contains("v15"));
        assert!(s.contains("protocol v8"));
    }

    #[test]
    fn feature_lookup_works() {
        let mut m = IdentityManifest::default_manifest();
        m.feature_flags.push("crdt".to_string());
        assert!(m.has_feature("crdt"));
        assert!(!m.has_feature("wasm"));
    }

    #[test]
    fn canonical_serialization_is_deterministic() {
        let mut a = IdentityManifest::default_manifest();
        a.feature_flags = vec!["wasm".to_string(), "crdt".to_string()];
        let mut b = a.clone();
        b.feature_flags.reverse();
        assert_eq!(a.canonical_json(), b.canonical_json());
    }
}
