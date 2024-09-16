use crate::file_format::{HelmLocal, HelmRemote, Manifest, Shell, Unit, UnitWithDependencies};
use indexmap::{indexmap, IndexMap};
use log::{debug, info};
use serde::Deserialize;
use std::collections::HashSet;
use std::io;

pub fn run_units(
    units: IndexMap<String, UnitWithDependencies>,
    units_args: Vec<String>,
    dependencies: bool,
    kubeconfig: Option<&String>,
    dry_run: bool,
) -> io::Result<()> {
    info!("Running units...");

    let units = get_filtered_units(units, units_args, dependencies, &mut HashSet::new());
    debug!("Units filtered based on config: {:?}", units);
    let units = reorder_units_from_dependencies(units, dependencies);
    debug!("Units filtered and re-ordered based on config: {:?}", units);

    for (unit_key, UnitWithDependencies { unit, .. }) in units.iter() {
        debug!("Running unit {} = {:?}", unit_key, unit);
        match unit {
            Unit::Noop { noop: _ } => {}
            Unit::Shell { shell } => {
                run_unit_shell(dry_run, &shell, kubeconfig)?;
            }
            Unit::Manifest { manifest } => {
                run_unit_manifest(dry_run, manifest, kubeconfig)?;
            }
            Unit::HelmRemote { helm_remote } => {
                run_unit_helm_remote(dry_run, helm_remote, kubeconfig)?;
            }
            Unit::HelmLocal { helm_local } => {
                run_unit_helm_local(dry_run, helm_local, kubeconfig)?;
            }
        }
    }

    Ok(())
}

fn reorder_units_from_dependencies(
    units: IndexMap<String, UnitWithDependencies>,
    dependencies: bool,
) -> IndexMap<String, UnitWithDependencies> {
    let mut output = IndexMap::new();
    while has_pending_units(&units, output.keys().collect::<Vec<&String>>().as_slice()) {
        for (unit_key, UnitWithDependencies { depends_on, .. }) in units.iter() {
            if dependencies {
                let depends_on: Vec<String> = depends_on.clone().unwrap_or(Vec::new());
                let missing_dependencies = depends_on
                    .iter()
                    .filter(|item| !output.keys().collect::<Vec<&String>>().contains(item))
                    .collect::<Vec<&String>>();
                if missing_dependencies.len() > 0 {
                    debug!(
                        "Skipping unit \"{}\", waiting for dependencies: {:?}",
                        unit_key, missing_dependencies
                    );
                    continue;
                }
            }

            output.insert(unit_key.to_string(), units[unit_key].clone());
        }
    }

    output
}

fn get_filtered_units(
    units: IndexMap<String, UnitWithDependencies>,
    units_args: Vec<String>,
    dependencies: bool,
    visited: &mut HashSet<String>,
) -> IndexMap<String, UnitWithDependencies> {
    let mut dependencies_by_unit_key = IndexMap::new();
    for (unit_key, unit) in units.iter() {
        dependencies_by_unit_key.insert(
            unit_key.clone(),
            unit.depends_on.clone().unwrap_or(Vec::new()),
        );
    }

    let mut filtered_units = IndexMap::new();

    let mut stack = units_args.clone();
    while let Some(next_unit_to_visit) = stack.pop() {
        if visited.contains(&next_unit_to_visit) {
            continue;
        }
        filtered_units.insert(
            next_unit_to_visit.to_string(),
            units.get(&next_unit_to_visit).unwrap().clone(),
        );

        if !dependencies {
            continue;
        }

        for neighbor in dependencies_by_unit_key
            .get(&next_unit_to_visit.to_string())
            .unwrap()
        {
            stack.push(neighbor.to_string());
        }
    }

    filtered_units
}

#[test]
fn test_get_filtered_units_returns_units_recursively_based_on_dependencies_parameter() {
    let units = indexmap! {
        "a".to_string() => UnitWithDependencies {
            unit: Unit::Noop {
                noop: "".to_string(),
            },
            depends_on: None,
        },
        "b".to_string() => UnitWithDependencies {
            unit: Unit::Noop {
                noop: "".to_string(),
            },
            depends_on: Some(vec!["a".to_string()]),
        },
        "c".to_string() => UnitWithDependencies {
            unit: Unit::Noop {
                noop: "".to_string(),
            },
            depends_on: Some(vec!["b".to_string()]),
        },
        "d".to_string() => UnitWithDependencies {
            unit: Unit::Noop {
                noop: "".to_string(),
            },
            depends_on: None,
        },
    };

    assert_eq!(
        indexmap! {
            "c".to_string() => UnitWithDependencies {
                unit: Unit::Noop {
                    noop: "".to_string(),
                },
                depends_on: Some(vec!["b".to_string()]),
            },
            "b".to_string() => UnitWithDependencies {
                unit: Unit::Noop {
                    noop: "".to_string(),
                },
                depends_on: Some(vec!["a".to_string()]),
            },
            "a".to_string() => UnitWithDependencies {
                unit: Unit::Noop {
                    noop: "".to_string(),
                },
                depends_on: None,
            },
        },
        get_filtered_units(
            units.clone(),
            vec!["c".to_string()],
            true,
            &mut HashSet::new()
        )
    );

    assert_eq!(
        indexmap! {
            "c".to_string() => UnitWithDependencies {
                unit: Unit::Noop {
                    noop: "".to_string(),
                },
                depends_on: Some(vec!["b".to_string()]),
            },
        },
        get_filtered_units(
            units.clone(),
            vec!["c".to_string()],
            false,
            &mut HashSet::new()
        )
    );
}

fn has_pending_units(
    units: &IndexMap<String, UnitWithDependencies>,
    unit_keys_done: &[&String],
) -> bool {
    let next_unit_not_yet_ran = units
        .iter()
        .filter(|(unit_key, _)| !unit_keys_done.contains(unit_key))
        .next();
    next_unit_not_yet_ran.is_some()
}

#[derive(Debug, Deserialize, PartialEq)]
struct HelmRelease {
    name: String,
    namespace: String,
}

fn helm_release_exists(
    name: &str,
    namespace: &str,
    kubeconfig: Option<&String>,
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

fn run_unit_helm_local(
    dry_run: bool,
    helm_local: &HelmLocal,
    kubeconfig: Option<&String>,
) -> Result<(), io::Error> {
    let already_installed = helm_release_exists(
        helm_local.name.as_str(),
        helm_local.namespace.as_str(),
        kubeconfig,
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
        kubeconfig,
        dry_run,
    )?;
    Ok(())
}

fn run_unit_helm_remote(
    dry_run: bool,
    helm_remote: &HelmRemote,
    kubeconfig: Option<&String>,
) -> Result<(), io::Error> {
    let already_installed = helm_release_exists(
        helm_remote.name.as_str(),
        helm_remote.namespace.as_str(),
        kubeconfig,
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

fn run_unit_manifest(
    dry_run: bool,
    manifest: &Manifest,
    kubeconfig: Option<&String>,
) -> Result<(), io::Error> {
    crate::utils::run_command_with_piped_stdio(
        "kubectl",
        &["apply", "-f", manifest.path.as_str()],
        kubeconfig,
        dry_run,
    )?;
    Ok(())
}

fn run_unit_shell(
    dry_run: bool,
    shell: &&Shell,
    kubeconfig: Option<&String>,
) -> Result<(), io::Error> {
    crate::utils::run_command_with_piped_stdio(
        "bash",
        &["-c", shell.input.as_str()],
        kubeconfig,
        dry_run,
    )?;
    Ok(())
}
