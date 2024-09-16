use crate::file_format::{Unit, UnitWithDependencies};
use file_format::Config;
use log::{debug, info};
use std::path::Path;
use std::{fs, io};

pub mod file_format;
pub mod helm_repositories;
pub mod units;
pub mod utils;

pub fn parse_deployment_file(deployment_file_path: &str) -> io::Result<Config> {
    info!("Deploying from {}...", deployment_file_path);

    let yaml_data = match fs::read_to_string(deployment_file_path) {
        Err(err) => {
            return Err(io::Error::new(
                err.kind(),
                format!("Unable to read configuration file: {}", err),
            ));
        }
        Ok(s) => s,
    };
    let mut config: Config = match serde_yaml::from_str(&yaml_data) {
        Err(err) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to parse configuration file: {}", err),
            ));
        }
        Ok(c) => c,
    };

    let deployment_file_dir = Path::new(deployment_file_path)
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();
    for (_, UnitWithDependencies { unit, .. }) in config.units.as_mut_slice() {
        match unit {
            Unit::Shell { .. } => {}
            Unit::Manifest { ref mut manifest } => {
                let mut new_path = deployment_file_dir.clone();
                new_path.push(&manifest.path);

                manifest.path = new_path.to_string_lossy().to_string();
            }
            Unit::HelmRemote {
                ref mut helm_remote,
            } => {
                if let Some(ref mut values) = helm_remote.values {
                    for value in values.iter_mut() {
                        let mut new_path = deployment_file_dir.clone();
                        new_path.push(value.clone());

                        *value = new_path.to_string_lossy().to_string();
                    }
                }
            }
            Unit::HelmLocal { ref mut helm_local } => {
                if let Some(ref mut values) = helm_local.values {
                    for value in values.iter_mut() {
                        let mut new_path = deployment_file_dir.clone();
                        new_path.push(value.clone());

                        *value = new_path.to_string_lossy().to_string();
                    }
                }

                let mut new_chart_path = deployment_file_dir.clone();
                new_chart_path.push(helm_local.chart_path.clone());
                helm_local.chart_path = new_chart_path.to_string_lossy().to_string();
            }
            Unit::Noop { .. } => {}
        }
    }

    debug!("Configuration: {:?}", config);

    Ok(config)
}
