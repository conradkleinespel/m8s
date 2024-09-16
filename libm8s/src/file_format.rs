use indexmap::{indexmap, IndexMap};
use serde::Deserialize;
use std::collections::HashSet;
use std::{fs, io};

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub helm_repositories: Option<Vec<HelmRepository>>,
    pub units: IndexMap<String, UnitWithDependencies>,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct HelmRepository {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct UnitWithDependencies {
    #[serde(flatten)]
    pub unit: Unit,
    pub depends_on: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum Unit {
    #[serde(rename_all = "camelCase")]
    Shell { shell: Shell },
    #[serde(rename_all = "camelCase")]
    Manifest { manifest: Manifest },
    #[serde(rename_all = "camelCase")]
    HelmRemote { helm_remote: HelmRemote },
    #[serde(rename_all = "camelCase")]
    HelmLocal { helm_local: HelmLocal },
    #[serde(rename_all = "camelCase")]
    Noop {
        #[allow(dead_code)]
        noop: String,
    },
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Shell {
    pub input: String,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub path: String,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HelmRemote {
    pub name: String,
    pub namespace: String,
    pub chart_name: String,
    pub chart_version: String,
    pub values: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HelmLocal {
    pub name: String,
    pub namespace: String,
    pub chart_path: String,
    pub values: Option<Vec<String>>,
}

/// Looks for cycles using a depth-first approach
/// See: https://en.wikipedia.org/wiki/Depth-first_search#Pseudocode
fn analyse_cycles(
    unit_key: &String,
    dependencies_by_unit_key: &IndexMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    stack: &mut Vec<String>,
) -> Result<(), Vec<String>> {
    if stack.contains(unit_key) {
        stack.push(unit_key.to_string());
        return Err(stack.clone());
    }
    if visited.contains(unit_key) {
        return Ok(());
    }

    visited.insert(unit_key.to_string());
    stack.push(unit_key.to_string());

    if let Some(neighbors) = dependencies_by_unit_key.get(unit_key) {
        for neighbor in neighbors {
            analyse_cycles(neighbor, dependencies_by_unit_key, visited, stack)?;
        }
    }

    stack.pop();
    Ok(())
}

#[test]
fn test_analyse_cycles_returns_cycle_as_error() {
    assert_eq!(
        vec![
            "a".to_string(),
            "c".to_string(),
            "b".to_string(),
            "a".to_string()
        ],
        analyse_cycles(
            &"a".to_string(),
            &indexmap! {
                "a".to_string() => vec!["c".to_string()],
                "b".to_string() => vec!["a".to_string()],
                "c".to_string() => vec!["b".to_string()],
            },
            &mut HashSet::new(),
            &mut Vec::new(),
        )
        .err()
        .unwrap()
    );
}

#[test]
fn test_analyse_cycles_returns_ok_when_no_cycle_is_detected() {
    assert_eq!(
        Ok(()),
        analyse_cycles(
            &"c".to_string(),
            &indexmap! {
                "a".to_string() => vec![],
                "b".to_string() => vec!["a".to_string()],
                "c".to_string() => vec!["b".to_string()],
            },
            &mut HashSet::new(),
            &mut Vec::new(),
        )
    );
}

pub fn check_dependency_cycles(units: &IndexMap<String, UnitWithDependencies>) -> io::Result<()> {
    let mut dependencies_by_unit_key = IndexMap::new();
    for (unit_key, unit) in units {
        dependencies_by_unit_key.insert(
            unit_key.clone(),
            unit.depends_on.clone().unwrap_or(Vec::new()),
        );
    }

    for unit_key in dependencies_by_unit_key.keys() {
        if let Err(cycle) = analyse_cycles(
            unit_key,
            &dependencies_by_unit_key,
            &mut HashSet::new(),
            &mut Vec::new(),
        ) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Configuration is invalid, dependency cycle for \"{}\": {}",
                    unit_key,
                    cycle.join(" -> ")
                ),
            ));
        }
    }
    Ok(())
}

fn create_file_not_exists_error(unit_key: &str, path: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Invalid unit {}, references file that doesn't exist: {}",
            unit_key, path
        ),
    )
}

fn create_directory_not_exists_error(unit_key: &str, path: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Invalid unit {}, references directory that doesn't exist: {}",
            unit_key, path
        ),
    )
}

pub fn check_files_exist(units: &IndexMap<String, UnitWithDependencies>) -> io::Result<()> {
    for (unit_key, UnitWithDependencies { unit, .. }) in units {
        match unit {
            Unit::Shell { .. } => {}
            Unit::Manifest { manifest, .. } => {
                if fs::exists(manifest.path.as_str())? == false {
                    return Err(create_file_not_exists_error(
                        unit_key.as_str(),
                        manifest.path.as_str(),
                    ));
                }
            }
            Unit::HelmRemote { helm_remote } => {
                for value in helm_remote.values.clone().unwrap_or(Vec::new()) {
                    if fs::exists(value.as_str())? == false {
                        return Err(create_file_not_exists_error(
                            unit_key.as_str(),
                            value.as_str(),
                        ));
                    }
                }
            }
            Unit::HelmLocal { helm_local } => {
                for value in helm_local.values.clone().unwrap_or(Vec::new()) {
                    if fs::exists(value.as_str())? == false {
                        return Err(create_file_not_exists_error(
                            unit_key.as_str(),
                            value.as_str(),
                        ));
                    }
                }

                if fs::exists(helm_local.chart_path.as_str())? == false {
                    return Err(create_directory_not_exists_error(
                        unit_key.as_str(),
                        helm_local.chart_path.as_str(),
                    ));
                }
            }
            Unit::Noop { .. } => {}
        }
    }
    Ok(())
}

pub fn check_invalid_unit_keys(units: &IndexMap<String, UnitWithDependencies>) -> io::Result<()> {
    let unit_keys: Vec<String> = units
        .iter()
        .map(|(unit_key, _)| unit_key.to_string())
        .collect();

    let depends_on_unit_keys: Vec<String> = units
        .iter()
        .map(|(_, unit_with_dependencies)| {
            unit_with_dependencies
                .depends_on
                .clone()
                .unwrap_or(Vec::new())
        })
        .flatten()
        .collect();

    let depends_on_unit_keys_invalid = depends_on_unit_keys
        .iter()
        .filter(|item| !unit_keys.contains(item))
        .map(|item| item.to_string())
        .collect::<Vec<String>>()
        .into_iter();

    // Deduplicate invalid unit keys that appear multiple times
    let mut depends_on_unit_keys_invalid = depends_on_unit_keys_invalid
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<String>>();

    depends_on_unit_keys_invalid.sort();

    if depends_on_unit_keys_invalid.len() > 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Configuration is invalid, invalid dependencies: {}",
                depends_on_unit_keys_invalid.join(", ")
            ),
        ));
    }
    Ok(())
}
