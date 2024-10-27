use crate::utils::CommandRunner;
use crate::{OptionDependencies, OptionHelmRepositories, OptionResources};
use std::io;

pub struct CommandUp {
    pub resources_args: Vec<String>,
    pub file: Option<String>,
    pub kubeconfig: Option<String>,
    pub helm_repositories: OptionHelmRepositories,
    pub resources: OptionResources,
    pub dependencies: OptionDependencies,
    pub dry_run: bool,
}

impl CommandRunner for CommandUp {
    fn run(&self) -> io::Result<()> {
        if (self.dependencies.dependencies || self.dependencies.no_dependencies)
            && self.resources_args.len() == 0
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "option --dependencies/--no-dependencies only works when you pass argument RESOURCES too"
                    .to_string(),
            ));
        }
        if self.resources.no_resources && self.resources_args.len() > 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "option --no-resources only works when you don't pass argument RESOURCES, you passed [{}]",
                    self.resources_args.join(", ")
                ),
            ));
        }

        let deployment_file_path = self.file.clone().unwrap_or("m8s.yaml".to_string());
        let config = libm8s::parse_deployment_file(deployment_file_path.as_str())?;

        libm8s::file_format::check_resource_keys_format(&config.resources)?;
        libm8s::file_format::check_invalid_resource_keys(&config.resources)?;
        libm8s::file_format::check_dependency_cycles(&config.resources)?;
        libm8s::file_format::check_files_exist(&config.resources)?;
        libm8s::file_format::check_helm_remote_repositories(
            &config.resources,
            &config
                .helm
                .as_ref()
                .and_then(|helm| helm.repositories.clone()),
        )?;

        if self.helm_repositories.get_value() {
            libm8s::helm_repositories::handle_helm_repositories(
                config
                    .helm
                    .as_ref()
                    .and_then(|helm| helm.repositories.clone())
                    .unwrap_or(Vec::new())
                    .as_slice(),
                self.dry_run,
            )
            .map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!("Adding helm repositories failed: {}", err),
                )
            })?;
        }

        if self.resources.get_value() {
            let resource_args = if self.resources_args.len() > 0 {
                self.resources_args.clone()
            } else {
                config.resources.keys().map(|k| k.to_string()).collect()
            };
            libm8s::resources::run_resources(
                &config.resources,
                None,
                resource_args,
                self.dependencies.get_value(),
                self.kubeconfig.clone(),
                self.dry_run,
            )
            .map_err(|err| {
                io::Error::new(err.kind(), format!("Running resources failed: {}", err))
            })?;
        }

        Ok(())
    }
}
