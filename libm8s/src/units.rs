use crate::file_format::{HelmLocal, HelmRemote, Manifest, Shell, Unit, UnitWithDependencies};
use indexmap::IndexMap;
use log::{debug, info};
use serde::Deserialize;
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

pub fn run_units(
    root: &Path,
    mut units: IndexMap<String, UnitWithDependencies>,
    units_filter_without_dependencies: Vec<String>,
    skip_dependencies: bool,
    kubeconfig: Option<&String>,
    dry_run: bool,
) -> io::Result<()> {
    info!("Running units...");

    debug!(
        "Unit filter without dependencies: {:?}",
        units_filter_without_dependencies
    );
    if units_filter_without_dependencies.len() > 0 {
        units = get_filtered_units(
            &units,
            &units_filter_without_dependencies,
            skip_dependencies,
            &mut HashSet::new(),
        );

        debug!(
            "Unit filter with dependencies: {:?}",
            units.keys().map(|i| i.to_string()).collect::<Vec<String>>()
        );
    }

    // return Ok(());

    let mut unit_keys_done: Vec<String> = Vec::new();
    while has_pending_units(&units, unit_keys_done.as_slice()) {
        debug!("Already done units: {:?}", unit_keys_done);

        for (unit_key, UnitWithDependencies { depends_on, unit }) in units.iter() {
            if !skip_dependencies {
                let depends_on: Vec<String> = depends_on.clone().unwrap_or(Vec::new());
                let missing_dependencies: Vec<String> = depends_on
                    .iter()
                    .filter(|item| !unit_keys_done.contains(item))
                    .map(|item| item.to_string())
                    .collect();
                if missing_dependencies.len() > 0 {
                    debug!(
                        "Skipping unit \"{}\", waiting for dependencies: {:?}",
                        unit_key, missing_dependencies
                    );
                    continue;
                }
            }

            debug!("Running unit {} = {:?}", unit_key, unit);
            match unit {
                Unit::Noop { noop: _ } => {}
                Unit::Shell { shell } => {
                    run_unit_shell(dry_run, &shell, kubeconfig)?;
                }
                Unit::Manifest { manifest } => {
                    run_unit_manifest(root, dry_run, manifest, kubeconfig)?;
                }
                Unit::HelmRemote { helm_remote } => {
                    run_unit_helm_remote(root, dry_run, helm_remote, kubeconfig)?;
                }
                Unit::HelmLocal { helm_local } => {
                    run_unit_helm_local(root, dry_run, helm_local, kubeconfig)?;
                }
            }

            unit_keys_done.push(unit_key.to_string());
        }
    }

    Ok(())
}

fn get_filtered_units(
    units: &IndexMap<String, UnitWithDependencies>,
    units_filter: &Vec<String>,
    skip_dependencies: bool,
    visited: &mut HashSet<String>,
) -> IndexMap<String, UnitWithDependencies> {
    let mut dependencies_by_unit_key = IndexMap::new();
    for (unit_key, unit) in units {
        dependencies_by_unit_key.insert(
            unit_key.clone(),
            unit.depends_on.clone().unwrap_or(Vec::new()),
        );
    }

    let mut filtered_units = IndexMap::new();

    let mut stack = units_filter.clone();
    while let Some(next_unit_to_visit) = stack.pop() {
        if visited.contains(&next_unit_to_visit) {
            continue;
        }
        filtered_units.insert(
            next_unit_to_visit.to_string(),
            units.get(&next_unit_to_visit).unwrap().clone(),
        );

        if skip_dependencies {
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
fn test_get_filtered_units() {
    let mut units = IndexMap::new();
    units.insert(
        "a".to_string(),
        UnitWithDependencies {
            unit: Unit::Noop {
                noop: "".to_string(),
            },
            depends_on: None,
        },
    );
    units.insert(
        "b".to_string(),
        UnitWithDependencies {
            unit: Unit::Noop {
                noop: "".to_string(),
            },
            depends_on: Some(vec!["a".to_string()]),
        },
    );
    units.insert(
        "c".to_string(),
        UnitWithDependencies {
            unit: Unit::Noop {
                noop: "".to_string(),
            },
            depends_on: None,
        },
    );

    let mut units_expected_with_dependencies = IndexMap::new();
    units_expected_with_dependencies.insert(
        "b".to_string(),
        UnitWithDependencies {
            unit: Unit::Noop {
                noop: "".to_string(),
            },
            depends_on: Some(vec!["a".to_string()]),
        },
    );
    units_expected_with_dependencies.insert(
        "a".to_string(),
        UnitWithDependencies {
            unit: Unit::Noop {
                noop: "".to_string(),
            },
            depends_on: None,
        },
    );

    let mut units_expected_without_dependencies = IndexMap::new();
    units_expected_without_dependencies.insert(
        "b".to_string(),
        UnitWithDependencies {
            unit: Unit::Noop {
                noop: "".to_string(),
            },
            depends_on: Some(vec!["a".to_string()]),
        },
    );

    assert_eq!(
        get_filtered_units(&units, &vec!["b".to_string()], false, &mut HashSet::new()),
        units_expected_with_dependencies
    );

    assert_eq!(
        get_filtered_units(&units, &vec!["b".to_string()], true, &mut HashSet::new()),
        units_expected_without_dependencies
    );
}

fn has_pending_units(
    units: &IndexMap<String, UnitWithDependencies>,
    unit_keys_done: &[String],
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
    root: &Path,
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
            "upgrade" // TODO: only upgrade when values/version/etc have changed
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
        let mut path = PathBuf::new();
        path.push(root);
        path.push(values_file.as_str());

        args.push("-f".to_string());
        args.push(path.to_str().unwrap().to_string());
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
    root: &Path,
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
            "upgrade" // TODO: only upgrade when values/version/etc have changed
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
        let mut path = PathBuf::new();
        path.push(root);
        path.push(values_file.as_str());

        args.push("-f".to_string());
        args.push(path.to_str().unwrap().to_string());
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
    root: &Path,
    dry_run: bool,
    manifest: &Manifest,
    kubeconfig: Option<&String>,
) -> Result<(), io::Error> {
    let mut path = PathBuf::new();
    path.push(root);
    path.push(manifest.path.as_str());
    crate::utils::run_command_with_piped_stdio(
        "kubectl",
        &["apply", "-f", path.to_str().unwrap()],
        kubeconfig,
        dry_run,
    )?;
    Ok(())
}

pub fn run_unit_shell(
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
