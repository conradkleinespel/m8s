use indexmap::{indexmap, IndexMap};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashSet;
use std::io::Error;
use std::{fs, io};

#[derive(Debug, Deserialize, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub helm: Option<Helm>,
    pub resources: IndexMap<String, ResourceWithDepdencies>,
}

#[derive(Debug, Deserialize, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct Helm {
    pub repositories: Option<Vec<HelmRepository>>,
}

#[derive(Debug, Deserialize, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct HelmRepository {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Deserialize, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceWithDepdencies {
    #[serde(flatten)]
    pub resource: Resource,
    pub depends_on: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, PartialEq, Clone, JsonSchema)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub enum Resource {
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
        group: IndexMap<String, ResourceWithDepdencies>,
    },
    #[serde(rename_all = "camelCase")]
    Noop {
        #[allow(dead_code)]
        noop: String,
    },
}

#[derive(Debug, Deserialize, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct Shell {
    pub input: String,
}

#[derive(Debug, Deserialize, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub path: String,
}

#[derive(Debug, Deserialize, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct HelmRemote {
    pub name: String,
    pub namespace: String,
    pub chart_name: String,
    pub chart_version: String,
    pub values: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct HelmLocal {
    pub name: String,
    pub namespace: String,
    pub chart_path: String,
    pub values: Option<Vec<String>>,
}

pub fn create_json_schema() -> io::Result<String> {
    let schema = schemars::schema_for!(Config);
    serde_json::to_string_pretty(&schema)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

#[test]
fn test_create_json_schema_returns_json_schema() {
    assert!(create_json_schema().unwrap().contains("\"$schema\":"))
}

fn analyse_cycles(
    resource_key: &String,
    dependencies_by_resource_key: &IndexMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    stack: &mut Vec<String>,
) -> Result<(), Vec<String>> {
    if stack.contains(resource_key) {
        stack.push(resource_key.to_string());
        return Err(stack.clone());
    }
    if visited.contains(resource_key) {
        return Ok(());
    }

    visited.insert(resource_key.to_string());
    stack.push(resource_key.to_string());

    if let Some(neighbors) = dependencies_by_resource_key.get(resource_key) {
        for neighbor in neighbors {
            analyse_cycles(neighbor, dependencies_by_resource_key, visited, stack)?;
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

pub fn check_dependency_cycles(
    resources: &IndexMap<String, ResourceWithDepdencies>,
) -> io::Result<()> {
    let mut dependencies_by_resource_key = IndexMap::new();
    for (resource_key, resource) in resources.iter() {
        dependencies_by_resource_key.insert(
            resource_key.clone(),
            resource.depends_on.clone().unwrap_or(Vec::new()),
        );
    }

    for resource_key in dependencies_by_resource_key.keys() {
        if let Err(cycle) = analyse_cycles(
            resource_key,
            &dependencies_by_resource_key,
            &mut HashSet::new(),
            &mut Vec::new(),
        ) {
            return Err(create_dependency_cycle_error(resource_key, cycle));
        }
    }
    Ok(())
}

fn create_dependency_cycle_error(resource_key: &String, cycle: Vec<String>) -> Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Configuration is invalid, dependency cycle for \"{}\": {}",
            resource_key,
            cycle.join(" -> ")
        ),
    )
}

fn create_file_not_exists_error(resource_key: &str, path: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Invalid resource {}, references file that doesn't exist: {}",
            resource_key, path
        ),
    )
}

fn create_directory_not_exists_error(resource_key: &str, path: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Invalid resource {}, references directory that doesn't exist: {}",
            resource_key, path
        ),
    )
}

pub fn check_helm_remote_repositories(
    resources: &IndexMap<String, ResourceWithDepdencies>,
    helm_repositories: &Option<Vec<HelmRepository>>,
) -> io::Result<()> {
    for (resource_key, ResourceWithDepdencies { resource, .. }) in resources {
        match resource {
            Resource::HelmRemote { helm_remote } => match helm_remote.chart_name.split_once("/") {
                None => {
                    return Err(create_invalid_helm_chart_name_error(
                        resource_key,
                        helm_remote,
                    ))
                }
                Some((repository_name, _)) => match helm_repositories {
                    None => {
                        return Err(create_helm_no_repositories_error(
                            resource_key,
                            repository_name,
                        ))
                    }
                    Some(helm_repositories) => {
                        if let None = helm_repositories
                            .iter()
                            .filter(|r| r.name == repository_name)
                            .next()
                        {
                            return Err(create_helm_repository_not_exists_error(
                                resource_key,
                                repository_name,
                                helm_repositories,
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

fn create_invalid_helm_chart_name_error(resource_key: &String, helm_remote: &HelmRemote) -> Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Invalid resource {}, chart name \"{}\" doesn't start with a repository name",
            resource_key, helm_remote.chart_name
        ),
    )
}

fn create_helm_no_repositories_error(resource_key: &String, repository_name: &str) -> Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Invalid resource {}, repository with name \"{}\" doesn't exist, no repositories configured",
            resource_key, repository_name
        ),
    )
}

fn create_helm_repository_not_exists_error(
    resource_key: &String,
    repository_name: &str,
    helm_repositories: &Vec<HelmRepository>,
) -> Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Invalid resource {}, repository with name \"{}\" doesn't exist, valid values are [{}]",
            resource_key,
            repository_name,
            helm_repositories
                .iter()
                .map(|r| r.name.clone())
                .collect::<Vec<String>>()
                .join(", ")
        ),
    )
}

pub fn check_files_exist(resources: &IndexMap<String, ResourceWithDepdencies>) -> io::Result<()> {
    for (resource_key, ResourceWithDepdencies { resource, .. }) in resources {
        match resource {
            Resource::Shell { .. } => {}
            Resource::Manifest { manifest, .. } => {
                if fs::exists(manifest.path.as_str())? == false {
                    return Err(create_file_not_exists_error(
                        resource_key.as_str(),
                        manifest.path.as_str(),
                    ));
                }
            }
            Resource::HelmRemote { helm_remote } => {
                for value in helm_remote.values.clone().unwrap_or(Vec::new()) {
                    if fs::exists(value.as_str())? == false {
                        return Err(create_file_not_exists_error(
                            resource_key.as_str(),
                            value.as_str(),
                        ));
                    }
                }
            }
            Resource::HelmLocal { helm_local } => {
                for value in helm_local.values.clone().unwrap_or(Vec::new()) {
                    if fs::exists(value.as_str())? == false {
                        return Err(create_file_not_exists_error(
                            resource_key.as_str(),
                            value.as_str(),
                        ));
                    }
                }

                if fs::exists(helm_local.chart_path.as_str())? == false {
                    return Err(create_directory_not_exists_error(
                        resource_key.as_str(),
                        helm_local.chart_path.as_str(),
                    ));
                }
            }
            Resource::Group { group } => {
                check_files_exist(group)?;
            }
            Resource::Noop { .. } => {}
        }
    }
    Ok(())
}

pub fn check_invalid_resource_keys(
    resources: &IndexMap<String, ResourceWithDepdencies>,
) -> io::Result<()> {
    let depends_on_resource_keys_invalid = get_invalid_resource_keys_for_group(resources);

    // Deduplicate invalid resource keys that appear multiple times
    let mut depends_on_resource_keys_invalid = depends_on_resource_keys_invalid
        .into_iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<String>>();

    depends_on_resource_keys_invalid.sort();

    if depends_on_resource_keys_invalid.len() > 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Configuration is invalid, invalid dependencies: {}",
                depends_on_resource_keys_invalid.join(", ")
            ),
        ));
    }
    Ok(())
}

fn get_invalid_resource_keys_for_group(
    resources: &IndexMap<String, ResourceWithDepdencies>,
) -> Vec<String> {
    let resource_keys: Vec<String> = resources.keys().map(|k| k.to_string()).collect();
    let depends_on_resource_keys: Vec<String> = resources
        .iter()
        .map(|(_, resource)| resource.depends_on.clone().unwrap_or(Vec::new()).clone())
        .flatten()
        .collect();

    let mut depends_on_resource_keys_invalid = depends_on_resource_keys
        .iter()
        .filter(|item| !resource_keys.contains(item))
        .map(|item| item.to_string())
        .collect::<Vec<String>>();

    for (_, resource) in resources {
        match &resource.resource {
            Resource::Group { group } => {
                depends_on_resource_keys_invalid.extend(get_invalid_resource_keys_for_group(group))
            }
            _ => {}
        }
    }

    depends_on_resource_keys_invalid
}

pub fn check_resource_keys_format(
    resources: &IndexMap<String, ResourceWithDepdencies>,
) -> io::Result<()> {
    for (resource_key, resource) in resources {
        if !is_resource_key_format_valid(resource_key.as_str()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Configuration is invalid, resource key can only contain [a-zA-Z0-9]: {}",
                    resource_key
                ),
            ));
        }

        if let Resource::Group { ref group } = resource.resource {
            check_resource_keys_format(group)?;
        }
    }
    Ok(())
}

fn is_resource_key_format_valid(key: &str) -> bool {
    key.chars().all(|c| c.is_ascii_alphanumeric())
}
