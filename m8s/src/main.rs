use clap::{Parser, Subcommand};
use libm8s::file_format::Config;
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
    /// Path to the deployment file in YAML format
    #[arg(short, long)]
    file: Option<String>,
}

#[derive(Subcommand)]
enum Command {
    /// Deploys resources using the current k8s config context
    Up {
        units: Vec<String>,
        #[clap(flatten)]
        global_options: GlobalConfigArgs,
        /// Change to DIRECTORY before doing anything
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
        /// Do not run dependencies, only run the units passed as arguments (requires UNITS)
        #[arg(long)]
        skip_dependencies: bool,
        /// Show logs but do not actually apply changes
        #[arg(long)]
        dry_run: bool,
    },
}

fn main() {
    let args = Cli::parse();

    let result = match args.command {
        Command::Up {
            units,
            global_options,
            directory,
            kubeconfig,
            skip_helm_repositories,
            skip_units,
            skip_dependencies,
            dry_run,
        } => execute_subcommand(
            units,
            global_options,
            directory,
            kubeconfig.as_ref(),
            skip_helm_repositories,
            skip_units,
            skip_dependencies,
            dry_run,
        ),
    };

    if let Err(err) = result {
        error!("{}", err);
        process::exit(1);
    }
}

fn execute_subcommand(
    units_filter_without_dependencies: Vec<String>,
    global_options: GlobalConfigArgs,
    directory: Option<String>,
    kubeconfig: Option<&String>,
    skip_helm_repositories: bool,
    skip_units: bool,
    skip_dependencies: bool,
    dry_run: bool,
) -> io::Result<()> {
    init_logging(global_options.verbose);

    if skip_dependencies && units_filter_without_dependencies.len() == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "option --skip-dependencies only works when you passe UNITS too",
        ));
    }

    if let Some(directory) = directory {
        env::set_current_dir(directory)?;
    }

    let deployment_file_path = global_options.file.unwrap_or("m8s.yaml".to_string());

    let config = libm8s::parse_deployment_file(deployment_file_path.as_str())?;
    let root = libm8s::build_resources_root_from_config(deployment_file_path.as_str(), &config)?;

    libm8s::file_format::check_invalid_unit_keys(&config.units)?;
    libm8s::file_format::check_dependency_cycles(&config.units)?;

    if !skip_helm_repositories {
        libm8s::helm_repositories::handle_helm_repositories(
            config.helm_repositories.as_slice(),
            dry_run,
        )
        .map_err(|err| {
            io::Error::new(
                err.kind(),
                format!("Adding helm repositories failed: {}", err),
            )
        })?;
    }

    if !skip_units {
        libm8s::units::run_units(
            root.as_path(),
            config.units,
            units_filter_without_dependencies,
            skip_dependencies,
            kubeconfig,
            dry_run,
        )
        .map_err(|err| io::Error::new(err.kind(), format!("Running units failed: {}", err)))?;
    }

    Ok(())
}

fn init_logging(verbose: bool) {
    if verbose {
        env::set_var("RUST_LOG", "debug");
    } else {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
}
