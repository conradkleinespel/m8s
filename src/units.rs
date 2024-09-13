use crate::file_format::{HelmLocal, HelmRemote, Manifest, Shell, Unit, UnitWithDependencies};
use indexmap::IndexMap;
use log::{debug, info};
use serde::Deserialize;
use std::io;
use std::path::{Path, PathBuf};

pub fn run_units(
    root: &Path,
    units: IndexMap<String, UnitWithDependencies>,
    dry_run: bool,
) -> io::Result<()> {
    info!("Running units...");

    let mut unit_keys_done: Vec<String> = Vec::new();
    while has_pending_units(&units, unit_keys_done.as_slice()) {
        debug!("Already done units: {:?}", unit_keys_done);

        for (unit_key, UnitWithDependencies { depends_on, unit }) in units.iter() {
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

            debug!("Running unit {:?}", unit);
            match unit {
                Unit::Noop { noop: _ } => {}
                Unit::Shell { shell } => {
                    run_unit_shell(dry_run, &shell)?;
                }
                Unit::Manifest { manifest } => {
                    run_unit_manifest(root, dry_run, manifest)?;
                }
                Unit::HelmRemote { helm_remote } => {
                    run_unit_helm_remote(root, dry_run, helm_remote)?;
                }
                Unit::HelmLocal { helm_local } => {
                    run_unit_helm_local(root, dry_run, helm_local)?;
                }
            }

            unit_keys_done.push(unit_key.to_string());
        }
    }

    Ok(())
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

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct HelmRelease {
    name: String,
    namespace: String,
}

fn helm_release_exists(name: &str, namespace: &str) -> io::Result<bool> {
    let output = std::process::Command::new("helm")
        .arg("list")
        .arg("--namespace")
        .arg(namespace)
        .arg("--output")
        .arg("yaml")
        .output()?;

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
) -> Result<(), io::Error> {
    let already_installed =
        helm_release_exists(helm_local.name.as_str(), helm_local.namespace.as_str())?;

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

    for values_file in helm_local.values.as_slice() {
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
        dry_run,
    )?;
    Ok(())
}

fn run_unit_helm_remote(
    root: &Path,
    dry_run: bool,
    helm_remote: &HelmRemote,
) -> Result<(), io::Error> {
    let already_installed =
        helm_release_exists(helm_remote.name.as_str(), helm_remote.namespace.as_str())?;

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

    for values_file in helm_remote.values.as_slice() {
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
        dry_run,
    )?;
    Ok(())
}

fn run_unit_manifest(root: &Path, dry_run: bool, manifest: &Manifest) -> Result<(), io::Error> {
    let mut path = PathBuf::new();
    path.push(root);
    path.push(manifest.path.as_str());
    crate::utils::run_command_with_piped_stdio(
        "kubectl",
        &["apply", "-f", path.to_str().unwrap()],
        dry_run,
    )?;
    Ok(())
}

pub fn run_unit_shell(dry_run: bool, shell: &&Shell) -> Result<(), io::Error> {
    crate::utils::run_command_with_piped_stdio("bash", &["-c", shell.input.as_str()], dry_run)?;
    Ok(())
}
