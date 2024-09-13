use crate::file_format::HelmRepository;
use log::{debug, info};
use std::io;

pub fn handle_helm_repositories(
    helm_repositories: &[HelmRepository],
    dry_run: bool,
) -> io::Result<()> {
    info!("Adding and updating Helm repositories...");

    for repository in helm_repositories {
        crate::utils::run_command_with_piped_stdio(
            "helm",
            &[
                "repo",
                "add",
                repository.name.as_str(),
                repository.url.as_str(),
            ],
            dry_run,
        )?;
        crate::utils::run_command_with_piped_stdio(
            "helm",
            &["repo", "update", repository.name.as_str()],
            dry_run,
        )?;

        debug!(
            "Repository {} = {} updated",
            repository.name, repository.url
        );
    }

    Ok(())
}
