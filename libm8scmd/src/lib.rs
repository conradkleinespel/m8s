use clap::{Args, Parser, Subcommand};
use std::io;

mod command_up;
pub mod utils;

#[derive(Parser)]
#[command(about = "Declarative k8s deployment using Kubectl, Helm and more", long_about = None)]
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

pub fn main_with_args(args: Vec<&str>, logging: bool) -> io::Result<()> {
    let args = Cli::parse_from(args);

    match args.command {
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
            if logging {
                utils::init_logging(global_options.verbose);
            }
            utils::with_directory(directory, || {
                command_up::execute_up_command(
                    units_args,
                    file.clone(),
                    kubeconfig.as_ref(),
                    helm_repositories,
                    units,
                    dependencies,
                    dry_run,
                )
            })
        }
    }
}
