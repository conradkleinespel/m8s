use clap::{Args, Parser, Subcommand};
use log::error;
use std::{env, io, process};

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

#[derive(Args)]
#[group(multiple = false)]
struct OptionHelmRepositories {
    /// Add and update Helm repositories (aka `helm repo add/update`)
    #[arg(long)]
    helm_repositories: bool,
    #[arg(long)]
    no_helm_repositories: bool,
}

impl OptionHelmRepositories {
    fn get_value(&self) -> bool {
        if self.helm_repositories {
            return true;
        }
        if self.no_helm_repositories {
            return false;
        }
        true
    }
}

#[derive(Args)]
#[group(multiple = false)]
struct OptionUnits {
    /// Update the Kubernetes resources (aka `kubectl apply`, `helm install`, etc)
    #[arg(long)]
    units: bool,
    #[arg(long)]
    no_units: bool,
}

impl OptionUnits {
    fn get_value(&self) -> bool {
        if self.units {
            return true;
        }
        if self.no_units {
            return false;
        }
        true
    }
}

#[derive(Args)]
#[group(multiple = false)]
struct OptionDependencies {
    /// Run units and their dependencies, requires UNITS
    #[arg(long)]
    dependencies: bool,
    #[arg(long)]
    no_dependencies: bool,
}

impl OptionDependencies {
    fn get_value(&self) -> bool {
        if self.dependencies {
            return true;
        }
        if self.no_dependencies {
            return false;
        }
        true
    }
}

#[derive(Subcommand)]
enum Command {
    /// Deploys resources using the current k8s config context
    Up {
        #[arg(name = "UNITS")]
        units_args: Vec<String>,
        /// Path to the deployment file in YAML format
        #[arg(short, long)]
        file: Option<String>,
        /// Change to DIRECTORY before doing anything
        #[arg(short = 'C', long)]
        directory: Option<String>,
        /// Path to the kubeconfig file to use for CLI requests
        #[arg(long)]
        kubeconfig: Option<String>,
        #[clap(flatten)]
        helm_repositories: OptionHelmRepositories,
        #[clap(flatten)]
        units: OptionUnits,
        #[clap(flatten)]
        dependencies: OptionDependencies,
        /// Show logs but do not actually apply changes
        #[arg(long)]
        dry_run: bool,
        #[clap(flatten)]
        global_options: GlobalConfigArgs,
    },
}

fn main() {
    let args = Cli::parse();

    let result = match args.command {
        Command::Up {
            units_args,
            global_options,
            file,
            directory,
            kubeconfig,
            helm_repositories,
            units,
            dependencies,
            dry_run,
        } => {
            init_logging(global_options.verbose);
            execute_up_command(
                units_args,
                file.clone(),
                directory,
                kubeconfig.as_ref(),
                helm_repositories,
                units,
                dependencies,
                dry_run,
            )
        }
    };

    if let Err(err) = result {
        error!("{}", err);
        process::exit(1);
    }
}

fn execute_up_command(
    units_args: Vec<String>,
    file: Option<String>,
    directory: Option<String>,
    kubeconfig: Option<&String>,
    helm_repositories: OptionHelmRepositories,
    units: OptionUnits,
    dependencies: OptionDependencies,
    dry_run: bool,
) -> io::Result<()> {
    if (dependencies.dependencies || dependencies.no_dependencies) && units_args.len() == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "option --dependencies/--no-dependencies only works when you pass argument UNITS too"
                .to_string(),
        ));
    }
    if units.no_units && units_args.len() > 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "option --no-units only works when you don't pass argument UNITS, you passed [{}]",
                units_args.join(", ")
            ),
        ));
    }

    if let Some(directory) = directory {
        env::set_current_dir(directory)?;
    }

    let deployment_file_path = file.unwrap_or("m8s.yaml".to_string());
    let config = libm8s::parse_deployment_file(deployment_file_path.as_str())?;

    libm8s::file_format::check_invalid_unit_keys(&config.units)?;
    libm8s::file_format::check_dependency_cycles(&config.units)?;
    libm8s::file_format::check_files_exist(&config.units)?;

    if helm_repositories.get_value() {
        libm8s::helm_repositories::handle_helm_repositories(
            config.helm_repositories.unwrap_or(Vec::new()).as_slice(),
            dry_run,
        )
        .map_err(|err| {
            io::Error::new(
                err.kind(),
                format!("Adding helm repositories failed: {}", err),
            )
        })?;
    }

    if units.get_value() {
        libm8s::units::run_units(
            config.units,
            units_args,
            dependencies.get_value(),
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
