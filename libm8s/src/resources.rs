use crate::file_format::{
    HelmLocal, HelmRemote, Manifest, Resource, ResourceWithDependencies, Shell,
};
use indexmap::{indexmap, IndexMap};
use log::{debug, info};
use serde::Deserialize;
use std::io;

pub fn run_resources(
    resources: &IndexMap<String, ResourceWithDependencies>,
    resources_args_namespace: Option<String>,
    resources_args: Vec<String>,
    dependencies: bool,
    kubeconfig: Option<String>,
    dry_run: bool,
) -> io::Result<()> {
    println!("resource args {:?}", resources_args);

    info!(
        "Running resources... resources_args = {} {:?}",
        resources_args_namespace
            .clone()
            .unwrap_or("root".to_string()),
        resources_args
    );

    let resources_args_part_0 = get_resources_args_part_0(&resources_args);
    let filtered_resources = get_filtered_resources(resources, resources_args_part_0, dependencies);
    debug!(
        "Resources filtered based on config: {:?}",
        filtered_resources
    );

    for (resource_key, ResourceWithDependencies { resource, .. }) in filtered_resources.iter() {
        debug!("Running resource {} = {:?}", resource_key, resource);
        match resource {
            Resource::Noop { noop: _ } => {}
            Resource::Shell { shell } => {
                run_resource_shell(dry_run, &shell, kubeconfig.clone())?;
            }
            Resource::Manifest { manifest } => {
                run_resource_manifest(dry_run, manifest, kubeconfig.clone())?;
            }
            Resource::HelmRemote { helm_remote } => {
                run_resource_helm_remote(dry_run, helm_remote, kubeconfig.clone())?;
            }
            Resource::HelmLocal { helm_local } => {
                run_resource_helm_local(dry_run, helm_local, kubeconfig.clone())?;
            }
            Resource::Group { group } => {
                let group_namespace =
                    get_group_namespace(resources_args_namespace.clone(), resource_key.as_str());

                // When no resource args are given for a group, it means it was passed from the CLI
                // and the user means to enable dependency resolution for the resources within this
                // group, even if --no-dependencies was passed.
                // In addition, in that specific case, we'll run all the resources in the group.
                let resource_args_for_group =
                    get_resources_args_part_1(&resources_args, resource_key.clone());
                let dependencies = dependencies || resource_args_for_group.len() == 0;
                let actual_resource_args_for_group = if resource_args_for_group.len() > 0 {
                    resource_args_for_group
                } else {
                    group.keys().map(|s| s.to_string()).collect()
                };

                run_resources(
                    group,
                    group_namespace,
                    actual_resource_args_for_group,
                    dependencies,
                    kubeconfig.clone(),
                    dry_run,
                )?;
            }
        }
    }

    Ok(())
}

fn get_group_namespace(parent_namespace: Option<String>, resource_key: &str) -> Option<String> {
    if let Some(ref ns) = parent_namespace {
        Some(format!("{}:{}", ns, resource_key))
    } else {
        Some(resource_key.to_string())
    }
}

#[test]
fn test_get_group_namespace_returns_parent_ns_with_resource_key() {
    assert_eq!(
        "parent:this-resource",
        get_group_namespace(Some("parent".to_string()), "this-resource").unwrap()
    );
}

#[test]
fn test_get_group_namespace_returns_resource_key_if_empty_parent() {
    assert_eq!(
        "this-resource",
        get_group_namespace(None, "this-resource").unwrap()
    );
}

fn get_resources_args_part_0(resources_args: &Vec<String>) -> Vec<String> {
    resources_args
        .clone()
        .iter()
        .map(|ua| {
            return ua
                .splitn(2, ":")
                .collect::<Vec<&str>>()
                .get(0)
                .unwrap()
                .to_string();
        })
        .collect()
}

#[test]
fn test_get_resources_args_part_0_returns_part_before_colon() {
    assert_eq!(
        vec!["a".to_string(), "b".to_string()],
        get_resources_args_part_0(&vec!["a".to_string(), "b:c".to_string()])
    );
}

fn get_resources_args_part_1(resources_args: &Vec<String>, resource_key: String) -> Vec<String> {
    let resource_key_group_prefix = format!("{}:", resource_key);
    resources_args
        .iter()
        .filter(|ua| ua.starts_with(resource_key_group_prefix.as_str()))
        .map(|ua| {
            ua.splitn(2, ":")
                .collect::<Vec<&str>>()
                .get(1)
                .unwrap()
                .to_string()
        })
        .collect()
}

#[test]
fn test_get_resources_args_part_1_returns_part_after_colon_or_nothing() {
    assert_eq!(
        vec!["c".to_string()],
        get_resources_args_part_1(
            &vec!["a".to_string(), "b:c".to_string(), "d:e".to_string()],
            "b".to_string()
        )
    );
    assert_eq!(
        Vec::<String>::new(),
        get_resources_args_part_1(
            &vec!["a".to_string(), "b:c".to_string(), "d:e".to_string()],
            "a".to_string()
        )
    );
}

fn reorder_resources_from_dependencies(
    resources: IndexMap<String, ResourceWithDependencies>,
) -> IndexMap<String, ResourceWithDependencies> {
    let mut output = IndexMap::new();
    while has_pending_resources(
        &resources,
        output.keys().collect::<Vec<&String>>().as_slice(),
    ) {
        for (resource_key, ResourceWithDependencies { depends_on, .. }) in resources.iter() {
            let depends_on: Vec<String> = depends_on.clone().unwrap_or(Vec::new());
            let missing_dependencies = depends_on
                .iter()
                .filter(|item| !output.keys().collect::<Vec<&String>>().contains(item))
                .collect::<Vec<&String>>();
            if missing_dependencies.len() > 0 {
                debug!(
                    "Skipping resource \"{}\", waiting for dependencies: {:?}",
                    resource_key, missing_dependencies
                );
                continue;
            }

            output.insert(resource_key.to_string(), resources[resource_key].clone());
        }
    }

    output
}

#[test]
fn test_reorder_resources_from_dependencies_returns_resources_in_right_order() {
    let resources = indexmap! {
        "b".to_string() => ResourceWithDependencies {
            resource: Resource::Noop {
                noop: "".to_string(),
            },
            depends_on: Some(vec!["a".to_string()]),
        },
        "a".to_string() => ResourceWithDependencies {
            resource: Resource::Noop {
                noop: "".to_string(),
            },
            depends_on: None,
        },
        "c".to_string() => ResourceWithDependencies {
            resource: Resource::Noop {
                noop: "".to_string(),
            },
            depends_on: Some(vec!["b".to_string()]),
        },
    };

    assert_eq!(
        indexmap! {
            "a".to_string() => ResourceWithDependencies {
                resource: Resource::Noop {
                    noop: "".to_string(),
                },
                depends_on: None,
            },
            "b".to_string() => ResourceWithDependencies {
                resource: Resource::Noop {
                    noop: "".to_string(),
                },
                depends_on: Some(vec!["a".to_string()]),
            },
            "c".to_string() => ResourceWithDependencies {
                resource: Resource::Noop {
                    noop: "".to_string(),
                },
                depends_on: Some(vec!["b".to_string()]),
            },
        }
        .as_slice(),
        reorder_resources_from_dependencies(resources).as_slice()
    );
}

fn get_filtered_resources(
    resources: &IndexMap<String, ResourceWithDependencies>,
    resources_args: Vec<String>,
    dependencies: bool,
) -> IndexMap<String, ResourceWithDependencies> {
    let mut dependencies_by_resource_key = IndexMap::new();
    for (resource_key, resource) in resources.iter() {
        dependencies_by_resource_key.insert(
            resource_key.clone(),
            resource.depends_on.clone().unwrap_or(Vec::new()),
        );
    }

    let mut filtered_resources = IndexMap::new();

    let mut stack = resources_args.clone();
    while let Some(next_resource_to_visit) = stack.pop() {
        filtered_resources.insert(
            next_resource_to_visit.to_string(),
            resources.get(&next_resource_to_visit).unwrap().clone(),
        );

        if !dependencies {
            continue;
        }

        for neighbor in dependencies_by_resource_key
            .get(&next_resource_to_visit.to_string())
            .unwrap()
        {
            stack.push(neighbor.to_string());
        }
    }

    if dependencies {
        reorder_resources_from_dependencies(filtered_resources)
    } else {
        filtered_resources
    }
}

#[test]
fn test_get_filtered_resources_returns_resources_recursively_based_on_dependencies_parameter() {
    let resources = indexmap! {
        "b".to_string() => ResourceWithDependencies {
            resource: Resource::Noop {
                noop: "".to_string(),
            },
            depends_on: Some(vec!["a".to_string()]),
        },
        "a".to_string() => ResourceWithDependencies {
            resource: Resource::Noop {
                noop: "".to_string(),
            },
            depends_on: None,
        },
        "c".to_string() => ResourceWithDependencies {
            resource: Resource::Noop {
                noop: "".to_string(),
            },
            depends_on: Some(vec!["b".to_string()]),
        },
        "d".to_string() => ResourceWithDependencies {
            resource: Resource::Noop {
                noop: "".to_string(),
            },
            depends_on: None,
        },
    };

    assert_eq!(
        indexmap! {
            "a".to_string() => ResourceWithDependencies {
                resource: Resource::Noop {
                    noop: "".to_string(),
                },
                depends_on: None,
            },
            "b".to_string() => ResourceWithDependencies {
                resource: Resource::Noop {
                    noop: "".to_string(),
                },
                depends_on: Some(vec!["a".to_string()]),
            },
            "c".to_string() => ResourceWithDependencies {
                resource: Resource::Noop {
                    noop: "".to_string(),
                },
                depends_on: Some(vec!["b".to_string()]),
            },
        }
        .as_slice(),
        get_filtered_resources(&resources, vec!["c".to_string()], true).as_slice()
    );

    assert_eq!(
        indexmap! {
            "c".to_string() => ResourceWithDependencies {
                resource: Resource::Noop {
                    noop: "".to_string(),
                },
                depends_on: Some(vec!["b".to_string()]),
            },
        }
        .as_slice(),
        get_filtered_resources(&resources, vec!["c".to_string()], false).as_slice()
    );
}

fn has_pending_resources(
    resources: &IndexMap<String, ResourceWithDependencies>,
    resource_keys_done: &[&String],
) -> bool {
    let next_resource_not_yet_ran = resources
        .iter()
        .filter(|(resource_key, _)| !resource_keys_done.contains(resource_key))
        .next();
    next_resource_not_yet_ran.is_some()
}

#[derive(Debug, Deserialize, PartialEq)]
struct HelmRelease {
    name: String,
    namespace: String,
}

fn helm_release_exists(
    name: &str,
    namespace: &str,
    kubeconfig: Option<String>,
    dry_run: bool,
) -> io::Result<bool> {
    let mut command = std::process::Command::new("helm");
    command
        .arg("list")
        .arg("--namespace")
        .arg(namespace)
        .arg("--output")
        .arg("yaml");

    if let Some(c) = kubeconfig {
        command.env("KUBECONFIG", c.to_string());
    }

    if dry_run {
        return Ok(false);
    }

    let output = command.output()?;

    let helm_releases: Vec<HelmRelease> = serde_yaml::from_str(
        String::from_utf8_lossy(output.stdout.as_slice()).as_ref(),
    )
    .map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Could not read helm releases: {}", err),
        )
    })?;

    Ok(helm_releases.contains(&HelmRelease {
        name: name.to_string(),
        namespace: namespace.to_string(),
    }))
}

fn run_resource_helm_local(
    dry_run: bool,
    helm_local: &HelmLocal,
    kubeconfig: Option<String>,
) -> Result<(), io::Error> {
    let already_installed = helm_release_exists(
        helm_local.name.as_str(),
        helm_local.namespace.as_str(),
        kubeconfig.clone(),
        dry_run,
    )?;

    let mut args = Vec::<String>::new();
    args.push(
        if already_installed {
            "upgrade"
        } else {
            "install"
        }
        .to_string(),
    );
    args.push(helm_local.name.to_string());
    args.push(helm_local.chart_path.to_string());
    args.push("--namespace".to_string());
    args.push(helm_local.namespace.to_string());

    for values_file in helm_local.values.clone().unwrap_or(Vec::new()).as_slice() {
        args.push("-f".to_string());
        args.push(values_file.clone());
    }

    crate::utils::run_command_with_piped_stdio(
        "helm",
        args.iter()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>()
            .as_slice(),
        kubeconfig.clone(),
        dry_run,
    )?;
    Ok(())
}

fn run_resource_helm_remote(
    dry_run: bool,
    helm_remote: &HelmRemote,
    kubeconfig: Option<String>,
) -> Result<(), io::Error> {
    let already_installed = helm_release_exists(
        helm_remote.name.as_str(),
        helm_remote.namespace.as_str(),
        kubeconfig.clone(),
        dry_run,
    )?;

    let mut args = Vec::<String>::new();
    args.push(
        if already_installed {
            "upgrade"
        } else {
            "install"
        }
        .to_string(),
    );
    args.push(helm_remote.name.to_string());
    args.push(helm_remote.chart_name.to_string());
    args.push("--version".to_string());
    args.push(helm_remote.chart_version.to_string());
    args.push("--namespace".to_string());
    args.push(helm_remote.namespace.to_string());

    for values_file in helm_remote.values.clone().unwrap_or(Vec::new()).as_slice() {
        args.push("-f".to_string());
        args.push(values_file.clone());
    }

    crate::utils::run_command_with_piped_stdio(
        "helm",
        args.iter()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>()
            .as_slice(),
        kubeconfig,
        dry_run,
    )?;
    Ok(())
}

fn run_resource_manifest(
    dry_run: bool,
    manifest: &Manifest,
    kubeconfig: Option<String>,
) -> Result<(), io::Error> {
    crate::utils::run_command_with_piped_stdio(
        "kubectl",
        &["apply", "-f", manifest.path.as_str()],
        kubeconfig,
        dry_run,
    )?;
    Ok(())
}

fn run_resource_shell(
    dry_run: bool,
    shell: &&Shell,
    kubeconfig: Option<String>,
) -> Result<(), io::Error> {
    crate::utils::run_command_with_piped_stdio(
        "bash",
        &["-c", shell.input.as_str()],
        kubeconfig,
        dry_run,
    )?;
    Ok(())
}
