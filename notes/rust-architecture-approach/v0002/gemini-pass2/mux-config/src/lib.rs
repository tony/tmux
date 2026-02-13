use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub keys: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub socket_path: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            socket_path: None,
        }
    }
}

pub fn load(s: &str) -> Result<Config, toml::de::Error> {
    toml::from_str(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let c = Config::default();
        assert!(c.server.socket_path.is_none());
        assert!(c.keys.is_empty());
    }

    #[test]
    fn test_load_empty() {
        let c = load("").unwrap();
        assert!(c.server.socket_path.is_none());
    }

    #[test]
    fn test_load_server_socket() {
        let toml = r#"
            [server]
            socket_path = "/tmp/tf.sock"
        "#;
        let c = load(toml).unwrap();
        assert_eq!(c.server.socket_path, Some("/tmp/tf.sock".to_string()));
    }

    #[test]
    fn test_load_keys() {
        let toml = r#"
            [keys]
            "Ctrl+a" = "prefix"
        "#;
        let c = load(toml).unwrap();
        assert_eq!(c.keys.get("Ctrl+a"), Some(&"prefix".to_string()));
    }

    #[test]
    fn test_invalid_toml() {
        assert!(load("invalid").is_err());
    }
}
