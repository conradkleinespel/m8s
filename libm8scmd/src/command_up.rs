use crate::utils::CommandRunner;
use crate::{OptionDependencies, OptionHelmRepositories, OptionResources};
use libm8s::FileReader;
use std::path::Path;
use std::{fs, io};

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
            && self.resources.get_value()
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
        let config = libm8s::parse_deployment_file(
            NativeFileReader {},
            &Path::new(deployment_file_path.as_str()),
        )?;

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

struct NativeFileReader;

impl FileReader for NativeFileReader {
    fn read_to_string(&self, file_path: &Path) -> io::Result<String> {
        fs::read_to_string(file_path)
    }
}

#[cfg(test)]
mod test {
    use crate::command_up::CommandUp;
    use crate::utils::{with_directory, CommandRunner};
    use crate::{OptionDependencies, OptionHelmRepositories, OptionResources};

    #[test]
    fn test_command_up_reads_from_m8s_yaml_by_default() {
        with_directory(Some("tests".to_string()), || {
            let cmd = CommandUp {
                resources_args: vec![],
                file: None,
                kubeconfig: None,
                helm_repositories: OptionHelmRepositories {
                    helm_repositories: false,
                    no_helm_repositories: true,
                },
                resources: OptionResources {
                    resources: false,
                    no_resources: true,
                },
                dependencies: OptionDependencies {
                    dependencies: false,
                    no_dependencies: true,
                },
                dry_run: true,
            };
            cmd.run()
        })
        .unwrap();
    }

    #[test]
    fn test_command_up_handles_repositories() {
        let cmd = CommandUp {
            resources_args: vec![],
            file: Some("tests/m8s_with_repositories.yaml".to_string()),
            kubeconfig: None,
            helm_repositories: OptionHelmRepositories {
                helm_repositories: true,
                no_helm_repositories: false,
            },
            resources: OptionResources {
                resources: false,
                no_resources: true,
            },
            dependencies: OptionDependencies {
                dependencies: false,
                no_dependencies: true,
            },
            dry_run: true,
        };
        with_directory(None, || cmd.run()).unwrap();
    }
}
