use indexmap::{indexmap, IndexMap};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::{fs, io};

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub helm_repositories: Option<Vec<HelmRepository>>,
    pub units: IndexMap<String, UnitWithDependencies>,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HelmRepository {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UnitWithDependencies {
    #[serde(flatten)]
    pub unit: Unit,
    pub depends_on: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
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
    Group {
        group: IndexMap<String, UnitWithDependencies>,
    },
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
    for (unit_key, unit) in units.iter() {
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

pub fn check_helm_remote_repositories(
    units: &IndexMap<String, UnitWithDependencies>,
    helm_repositories: &Option<Vec<HelmRepository>>,
) -> io::Result<()> {
    for (unit_key, UnitWithDependencies { unit, .. }) in units {
        match unit {
            Unit::HelmRemote { helm_remote } => match helm_remote.chart_name.split_once("/") {
                None => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                        "Invalid unit {}, chart name \"{}\" doesn't start with a repository name",
                        unit_key, helm_remote.chart_name
                    ),
                    ))
                }
                Some((repository_name,_)) => match helm_repositories {
                    None => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "Invalid unit {}, repository with name \"{}\" doesn't exist, no repositories configured",
                                unit_key, repository_name
                            ),
                        ))
                    }
                    Some(helm_repositories) => {
                        if helm_repositories
                            .iter()
                            .filter(|r| r.name == repository_name)
                            .next()
                            .is_none()
                        {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                format!(
                                    "Invalid unit {}, repository with name \"{}\" doesn't exist, valid values are [{}]",
                                    unit_key,
                                    repository_name,
                                    helm_repositories.iter().map(|r| r.name.clone()).collect::<Vec<String>>().join(", ")
                                ),
                            ));
                        }
                    }
                },
            },
            _ => {}
        }
    }
    Ok(())
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
            Unit::Group { group } => {
                check_files_exist(group)?;
            }
            Unit::Noop { .. } => {}
        }
    }
    Ok(())
}

pub fn check_invalid_unit_keys(units: &IndexMap<String, UnitWithDependencies>) -> io::Result<()> {
    let depends_on_unit_keys_invalid = get_invalid_unit_keys_for_group(units);

    // Deduplicate invalid unit keys that appear multiple times
    let mut depends_on_unit_keys_invalid = depends_on_unit_keys_invalid
        .into_iter()
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

fn get_invalid_unit_keys_for_group(units: &IndexMap<String, UnitWithDependencies>) -> Vec<String> {
    let unit_keys: Vec<String> = units.keys().map(|k| k.to_string()).collect();
    let depends_on_unit_keys: Vec<String> = units
        .iter()
        .map(|(_, unit)| unit.depends_on.clone().unwrap_or(Vec::new()).clone())
        .flatten()
        .collect();

    let mut depends_on_unit_keys_invalid = depends_on_unit_keys
        .iter()
        .filter(|item| !unit_keys.contains(item))
        .map(|item| item.to_string())
        .collect::<Vec<String>>();

    for (_, unit) in units {
        match &unit.unit {
            Unit::Group { group } => {
                depends_on_unit_keys_invalid.extend(get_invalid_unit_keys_for_group(group))
            }
            _ => {}
        }
    }

    depends_on_unit_keys_invalid
}

pub fn check_duplicate_unit_keys(units: &IndexMap<String, UnitWithDependencies>) -> io::Result<()> {
    let unit_keys = get_unit_keys_for_group(units);
    let mut counts = HashMap::new();
    for item in unit_keys.clone() {
        *counts.entry(item).or_insert(0) += 1;
    }

    let mut duplicate_unit_keys: Vec<String> = unit_keys
        .into_iter()
        .filter(|item| counts[item] > 1)
        .collect::<Vec<String>>()
        .into_iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<String>>();
    duplicate_unit_keys.sort();

    if duplicate_unit_keys.len() > 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Configuration is invalid, duplicate keys: {}",
                duplicate_unit_keys.join(", ")
            ),
        ));
    }
    Ok(())
}

fn get_unit_keys_for_group(units: &IndexMap<String, UnitWithDependencies>) -> Vec<String> {
    let mut unit_keys: Vec<String> = units.keys().map(|k| k.to_string()).collect();
    for (_, unit) in units {
        match &unit.unit {
            Unit::Group { group } => unit_keys.extend(get_unit_keys_for_group(group)),
            _ => {}
        }
    }

    unit_keys
}
