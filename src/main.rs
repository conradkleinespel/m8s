mod file_format;
mod helm_repositories;
mod units;
mod utils;

use clap::{Parser, Subcommand};
use file_format::Config;
use log::{debug, error, info};
use serde_yaml;
use std::path::{Path, PathBuf};
use std::{env, fs, io, process};

#[derive(Parser)]
#[command(about = "What if helm, kubectl and others were roommates", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Default, Clone, clap::Args)]
struct GlobalConfigArgs {
    /// Show verbose logs
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Deploys resources using the current k8s config context
    Up {
        #[clap(flatten)]
        global_options: GlobalConfigArgs,
        /// Path to the deployment file in YAML format
        deployment_file_path: String,
        #[arg(short = 'C', long)]
        directory: Option<String>,
        /// Path to the kubeconfig file to use for CLI requests
        #[arg(long)]
        kubeconfig: Option<String>,
        /// Do not add or update Helm repositories (aka `helm repo add`)
        #[arg(long)]
        skip_helm_repositories: bool,
        /// Do not update the Kubernetes resources (aka `kubectl apply`, `helm install`, etc)
        #[arg(long)]
        skip_units: bool,
        /// Show logs but do not actually apply changes
        #[arg(long)]
        dry_run: bool,
    },
}

fn main() {
    let args = Cli::parse();

    let result = match args.command {
        Command::Up {
            global_options,
            deployment_file_path,
            directory,
            kubeconfig,
            skip_helm_repositories,
            skip_units,
            dry_run,
        } => execute_subcommand(
            global_options,
            deployment_file_path,
            directory,
            kubeconfig.as_ref(),
            skip_helm_repositories,
            skip_units,
            dry_run,
        ),
    };

    if let Err(err) = result {
        error!("{}", err);
        process::exit(1);
    }
}

fn execute_subcommand(
    global_options: GlobalConfigArgs,
    deployment_file_path: String,
    directory: Option<String>,
    kubeconfig: Option<&String>,
    skip_helm_repositories: bool,
    skip_units: bool,
    dry_run: bool,
) -> io::Result<()> {
    init_logging(global_options.verbose);

    if let Some(directory) = directory {
        env::set_current_dir(directory)?;
    }

    let config = parse_deployment_file(deployment_file_path.as_str())?;
    let root = build_resources_root_from_config(deployment_file_path.as_str(), &config)?;

    file_format::check_invalid_unit_keys(&config.units)?;
    file_format::check_dependency_cycles(&config.units)?;

    if !skip_helm_repositories {
        helm_repositories::handle_helm_repositories(config.helm_repositories.as_slice(), dry_run)
            .map_err(|err| {
            io::Error::new(
                err.kind(),
                format!("Adding helm repositories failed: {}", err),
            )
        })?;
    }

    if !skip_units {
        units::run_units(root.as_path(), config.units, kubeconfig, dry_run)
            .map_err(|err| io::Error::new(err.kind(), format!("Running units failed: {}", err)))?;
    }

    Ok(())
}

fn build_resources_root_from_config(
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

fn parse_deployment_file(deployment_file_path: &str) -> io::Result<Config> {
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

fn init_logging(verbose: bool) {
    if verbose {
        env::set_var("RUST_LOG", "debug");
    } else {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
}
