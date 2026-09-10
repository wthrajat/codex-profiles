use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

pub const MARKER_FILE: &str = ".codex-profile-switcher.toml";
pub const CODEX_CONFIG_FILE: &str = "config.toml";
pub const SCHEMA_VERSION: u32 = 1;
pub const CODEX_CONFIG: &str = "cli_auth_credentials_store = \"file\"\n";

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileMarker {
    pub schema_version: u32,
    pub name: String,
    pub created_by_version: String,
}

impl ProfileMarker {
    pub fn new(name: &str) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            name: name.to_owned(),
            created_by_version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }

    pub fn serialize(&self) -> Result<String> {
        toml::to_string(self).map_err(|error| AppError::InvalidMarker {
            name: self.name.clone(),
            reason: error.to_string(),
        })
    }

    pub fn parse(name: &str, contents: &str) -> Result<Self> {
        let marker: Self = toml::from_str(contents).map_err(|error| AppError::InvalidMarker {
            name: name.to_owned(),
            reason: error.to_string(),
        })?;

        if marker.schema_version != SCHEMA_VERSION {
            return Err(AppError::InvalidMarker {
                name: name.to_owned(),
                reason: format!(
                    "unsupported schema version {} (expected {SCHEMA_VERSION})",
                    marker.schema_version
                ),
            });
        }
        if marker.name != name {
            return Err(AppError::InvalidMarker {
                name: name.to_owned(),
                reason: format!("marker names profile {:?}", marker.name),
            });
        }
        if marker.created_by_version.trim().is_empty() {
            return Err(AppError::InvalidMarker {
                name: name.to_owned(),
                reason: "created_by_version is empty".to_owned(),
            });
        }

        Ok(marker)
    }
}

pub fn verify_codex_config(name: &str, contents: &str) -> Result<()> {
    let document: toml::Value =
        toml::from_str(contents).map_err(|error| AppError::UnsafeCredentialConfig {
            name: name.to_owned(),
            reason: format!("config.toml is invalid TOML: {error}"),
        })?;

    match document.get("cli_auth_credentials_store") {
        Some(toml::Value::String(value)) if value == "file" => Ok(()),
        Some(value) => Err(AppError::UnsafeCredentialConfig {
            name: name.to_owned(),
            reason: format!("cli_auth_credentials_store is {}, expected \"file\"", value),
        }),
        None => Err(AppError::UnsafeCredentialConfig {
            name: name.to_owned(),
            reason: "config.toml is missing cli_auth_credentials_store = \"file\"".to_owned(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_round_trips() {
        let marker = ProfileMarker::new("work");
        let serialized = marker.serialize().unwrap();
        let parsed = ProfileMarker::parse("work", &serialized).unwrap();

        assert_eq!(parsed.name, "work");
        assert_eq!(parsed.schema_version, SCHEMA_VERSION);
    }

    #[test]
    fn config_can_contain_other_codex_settings() {
        verify_codex_config(
            "work",
            "model = \"example\"\ncli_auth_credentials_store = \"file\"\n",
        )
        .unwrap();
    }

    #[test]
    fn config_rejects_non_file_storage() {
        let error =
            verify_codex_config("work", "cli_auth_credentials_store = \"keyring\"\n").unwrap_err();

        assert!(error.to_string().contains("expected \"file\""));
    }
}
