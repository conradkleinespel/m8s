mod file_format;
mod helm_repositories;
mod units;
mod utils;

use clap::Parser;
use file_format::Config;
use log::{debug, error, info};
use serde_yaml;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(about = "What if helm, kubectl and others were roommates", long_about = None)]
struct Cli {
    /// Path to the deployment file in YAML format
    deployment_file_path: String,
    #[arg(short, long)]
    verbose: bool,
    #[arg(long)]
    skip_helm_repositories: bool,
    #[arg(long)]
    skip_units: bool,
    #[arg(long)]
    upgrade: bool,
    #[arg(long)]
    dry_run: bool,
}

fn main() {
    let args = Cli::parse();

    if args.verbose {
        std::env::set_var("RUST_LOG", "debug");
    } else {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    info!("Deploying from {}...", args.deployment_file_path);

    let yaml_data = match fs::read_to_string(args.deployment_file_path.as_str()) {
        Err(err) => {
            error!("Unable to read configuration file: {}", err);
            return;
        }
        Ok(s) => s,
    };
    let config: Config = match serde_yaml::from_str(&yaml_data) {
        Err(err) => {
            error!("Unable to parse configuration file: {}", err);
            return;
        }
        Ok(c) => c,
    };

    debug!("Configuration: {:?}", config);

    let mut root = PathBuf::new();
    root.push(
        Path::new(args.deployment_file_path.as_str())
            .parent()
            .unwrap(),
    );
    root.push(config.root);

    debug!(
        "Root is {}",
        fs::canonicalize(root.as_path()).unwrap().to_str().unwrap()
    );

    if let Err(err) = file_format::check_invalid_unit_keys(&config.units)
        .and_then(|_| file_format::check_dependency_cycles(&config.units))
    {
        error!("Configuration is invalid: {}", err);
        return;
    }

    if !args.skip_helm_repositories {
        if let Err(err) = helm_repositories::handle_helm_repositories(
            config.helm_repositories.as_slice(),
            args.dry_run,
        ) {
            error!("Adding helm repositories failed: {}", err);
            return;
        }
    }

    if !args.skip_units {
        if let Err(err) = units::run_units(root.as_path(), config.units, args.upgrade, args.dry_run)
        {
            error!("Running units failed: {}", err);
            return;
        }
    }
}
