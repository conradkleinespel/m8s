use file_format::Config;
use log::{debug, info};
use std::path::{Path, PathBuf};
use std::{fs, io};

pub mod file_format;
pub mod helm_repositories;
pub mod units;
pub mod utils;

pub fn build_resources_root_from_config(
    deployment_file_path: &str,
    config: &Config,
) -> io::Result<PathBuf> {
    debug!("Building resources root from config: {:?}", config.root);
    let mut root = PathBuf::new();
    root.push(Path::new(deployment_file_path).parent().unwrap());
    // Need at least "." to prevent the root being empty, see Path::parent()
    root.push(config.root.clone().unwrap_or(".".to_string()));

    let absolute_path = fs::canonicalize(root.as_path())?
        .to_string_lossy()
        .to_string();
    debug!("Root is {}", absolute_path);

    Ok(root)
}

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
    let config: Config = match serde_yaml::from_str(&yaml_data) {
        Err(err) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to parse configuration file: {}", err),
            ));
        }
        Ok(c) => c,
    };

    debug!("Configuration: {:?}", config);

    Ok(config)
}
