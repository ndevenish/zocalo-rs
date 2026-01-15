use std::collections::HashMap;

use serde::Deserialize;

use super::ConfigError;

#[derive(Debug, Clone)]
pub struct Environment {
    groups: HashMap<String, Vec<String>>,
    alias: Option<String>,
}

impl Environment {
    pub fn from_raw(raw: RawEnvironment) -> Result<Self, ConfigError> {
        match raw {
            RawEnvironment::Alias(alias) => Ok(Environment {
                groups: HashMap::new(),
                alias: Some(alias),
            }),
            RawEnvironment::Definition(def) => {
                let mut groups = HashMap::new();
                for (group_name, group_plugins) in def {
                    groups.insert(group_name, group_plugins);
                }
                Ok(Environment {
                    groups,
                    alias: None,
                })
            }
            RawEnvironment::PluginList(plugins) => {
                let mut groups = HashMap::new();
                groups.insert("plugins".to_string(), plugins);
                Ok(Environment {
                    groups,
                    alias: None,
                })
            }
        }
    }

    pub fn alias(&self) -> Option<&str> {
        self.alias.as_deref()
    }

    pub fn groups(&self) -> impl Iterator<Item = &str> {
        self.groups.keys().map(|s| s.as_str())
    }

    pub fn get_group(&self, name: &str) -> Option<&[String]> {
        self.groups.get(name).map(|v| v.as_slice())
    }

    pub fn all_plugins(&self) -> impl Iterator<Item = &str> {
        // Return plugins in order: alphabetically sorted groups (except "plugins" which comes last)
        let mut group_names: Vec<&String> = self
            .groups
            .keys()
            .filter(|k| k.as_str() != "plugins")
            .collect();
        group_names.sort();

        let plugins_group: Vec<&str> = self
            .groups
            .get("plugins")
            .map(|v| v.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default();

        group_names
            .into_iter()
            .flat_map(move |name| self.groups.get(name).into_iter().flatten())
            .map(|s| s.as_str())
            .chain(plugins_group)
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RawEnvironment {
    Alias(String),
    Definition(HashMap<String, Vec<String>>),
    PluginList(Vec<String>),
}
