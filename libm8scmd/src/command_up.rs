use crate::utils::CommandRunner;
use crate::{OptionDependencies, OptionHelmRepositories, OptionUnits};
use std::io;

pub struct CommandUp {
    pub units_args: Vec<String>,
    pub file: Option<String>,
    pub kubeconfig: Option<String>,
    pub helm_repositories: OptionHelmRepositories,
    pub units: OptionUnits,
    pub dependencies: OptionDependencies,
    pub dry_run: bool,
}

impl CommandRunner for CommandUp {
    fn run(&self) -> io::Result<()> {
        if (self.dependencies.dependencies || self.dependencies.no_dependencies)
            && self.units_args.len() == 0
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "option --dependencies/--no-dependencies only works when you pass argument UNITS too"
                    .to_string(),
            ));
        }
        if self.units.no_units && self.units_args.len() > 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "option --no-units only works when you don't pass argument UNITS, you passed [{}]",
                    self.units_args.join(", ")
                ),
            ));
        }

        let deployment_file_path = self.file.clone().unwrap_or("m8s.yaml".to_string());
        let config = libm8s::parse_deployment_file(deployment_file_path.as_str())?;

        libm8s::file_format::check_unit_keys_format(&config.units)?;
        libm8s::file_format::check_invalid_unit_keys(&config.units)?;
        libm8s::file_format::check_dependency_cycles(&config.units)?;
        libm8s::file_format::check_files_exist(&config.units)?;
        libm8s::file_format::check_helm_remote_repositories(
            &config.units,
            &config.helm_repositories,
        )?;

        if self.helm_repositories.get_value() {
            libm8s::helm_repositories::handle_helm_repositories(
                config.helm_repositories.unwrap_or(Vec::new()).as_slice(),
                self.dry_run,
            )
            .map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!("Adding helm repositories failed: {}", err),
                )
            })?;
        }

        if self.units.get_value() {
            let unit_args = if self.units_args.len() > 0 {
                self.units_args.clone()
            } else {
                config.units.keys().map(|k| k.to_string()).collect()
            };
            libm8s::units::run_units(
                &config.units,
                None,
                unit_args,
                self.dependencies.get_value(),
                self.kubeconfig.clone(),
                self.dry_run,
            )
            .map_err(|err| io::Error::new(err.kind(), format!("Running units failed: {}", err)))?;
        }

        Ok(())
    }
}
