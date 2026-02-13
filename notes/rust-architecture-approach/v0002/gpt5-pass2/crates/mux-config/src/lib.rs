#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConfig {
    pub socket_path: String,
    pub max_clients: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderConfig {
    pub flood_row_quota: usize,
    pub passthrough_graphics: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub render: RenderConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig { socket_path: "/tmp/termforge.sock".into(), max_clients: 64 },
            render: RenderConfig { flood_row_quota: 8, passthrough_graphics: false },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReloadPlan {
    pub changed: bool,
    pub old: Config,
    pub new: Config,
}

#[must_use]
pub fn parse_toml(input: &str) -> Result<Config, toml::de::Error> {
    toml::from_str(input)
}

#[must_use]
pub fn to_toml(config: &Config) -> Result<String, toml::ser::Error> {
    toml::to_string(config)
}

#[must_use]
pub fn plan_reload(old: &Config, new: &Config) -> ReloadPlan {
    ReloadPlan { changed: old != new, old: old.clone(), new: new.clone() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_safe() {
        let c = Config::default();
        assert!(!c.render.passthrough_graphics);
    }

    #[test]
    fn parse_toml_roundtrip() {
        let c = Config::default();
        let s = to_toml(&c).expect("serialize");
        let decoded = parse_toml(&s).expect("parse");
        assert_eq!(decoded, c);
    }

    #[test]
    fn parse_custom_values() {
        let s = r#"
[server]
socket_path = "/tmp/x.sock"
max_clients = 12

[render]
flood_row_quota = 2
passthrough_graphics = true
"#;
        let c = parse_toml(s).expect("parse");
        assert_eq!(c.server.max_clients, 12);
        assert!(c.render.passthrough_graphics);
    }

    #[test]
    fn reload_detects_no_change() {
        let c = Config::default();
        let p = plan_reload(&c, &c);
        assert!(!p.changed);
    }

    #[test]
    fn reload_detects_change() {
        let old = Config::default();
        let mut new = old.clone();
        new.server.max_clients = 90;
        assert!(plan_reload(&old, &new).changed);
    }

    #[test]
    fn invalid_toml_fails() {
        assert!(parse_toml("not = [valid").is_err());
    }
}
