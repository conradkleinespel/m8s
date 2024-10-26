use crate::{OptionDependencies, OptionHelmRepositories, OptionUnits};
use std::io;

pub fn execute_up_command(
    units_args: Vec<String>,
    file: Option<String>,
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

    let deployment_file_path = file.unwrap_or("m8s.yaml".to_string());
    let config = libm8s::parse_deployment_file(deployment_file_path.as_str())?;

    libm8s::file_format::check_unit_keys_format(&config.units)?;
    libm8s::file_format::check_invalid_unit_keys(&config.units)?;
    libm8s::file_format::check_dependency_cycles(&config.units)?;
    libm8s::file_format::check_files_exist(&config.units)?;
    libm8s::file_format::check_helm_remote_repositories(&config.units, &config.helm_repositories)?;

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
        let unit_args = if units_args.len() > 0 {
            units_args
        } else {
            config.units.keys().map(|k| k.to_string()).collect()
        };
        libm8s::units::run_units(
            &config.units,
            unit_args,
            dependencies.get_value(),
            kubeconfig,
            dry_run,
        )
        .map_err(|err| io::Error::new(err.kind(), format!("Running units failed: {}", err)))?;
    }

    Ok(())
}
