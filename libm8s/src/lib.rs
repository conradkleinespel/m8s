use crate::file_format::{Resource, ResourceWithDependencies};
use file_format::Config;
use indexmap::IndexMap;
use log::{debug, info};
use std::io;
use std::path::{Path, PathBuf};

pub mod file_format;
pub mod helm_repositories;
pub mod resources;
pub mod utils;

pub trait FileReader {
    fn read_to_string(&self, file_path: &Path) -> io::Result<String>;
}

pub fn parse_deployment_file(
    file_reader: impl FileReader,
    deployment_file_path: &Path,
) -> io::Result<Config> {
    info!("Deploying from {:?}...", deployment_file_path);

    let yaml_data = match file_reader.read_to_string(deployment_file_path) {
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

    integrate_deployment_file_dir_into_paths(&mut config.resources, deployment_file_dir);

    debug!("Configuration: {:?}", config);

    Ok(config)
}

fn integrate_deployment_file_dir_into_paths(
    resources: &mut IndexMap<String, ResourceWithDependencies>,
    deployment_file_dir: PathBuf,
) {
    for (_, ResourceWithDependencies { resource, .. }) in resources {
        match resource {
            Resource::Shell { .. } => {}
            Resource::Manifest { ref mut manifest } => {
                let mut new_path = deployment_file_dir.clone();
                new_path.push(&manifest.path);

                manifest.path = new_path.to_string_lossy().to_string();
            }
            Resource::HelmRemote {
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
            Resource::HelmLocal { ref mut helm_local } => {
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
            Resource::Group { ref mut group } => {
                integrate_deployment_file_dir_into_paths(group, deployment_file_dir.clone());
            }
            Resource::Noop { .. } => {}
        }
    }
}
