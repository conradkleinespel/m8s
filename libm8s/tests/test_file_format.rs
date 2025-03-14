use indexmap::indexmap;
use libm8s::file_format::Resource::HelmLocal as HelmLocalResource;
use libm8s::file_format::Resource::HelmRemote as HelmRemoteResource;
use libm8s::file_format::Resource::Manifest as ManifestResource;
use libm8s::file_format::{
    check_dependency_cycles, check_files_exist, check_helm_remote_repositories,
    check_invalid_resource_keys, check_resource_keys_format, Config, HelmLocal, HelmRemote,
    HelmRepository, Manifest, Resource, ResourceWithDepdencies,
};

#[test]
fn test_parse_succeeds_with_only_resources() {
    let test_file_yaml = include_str!("m8s_only_resources.yaml");

    let config: Config = serde_yaml::from_str(test_file_yaml).unwrap();
    assert_eq!(
        config.resources.get("foobarNoop").unwrap(),
        &ResourceWithDepdencies {
            resource: Resource::Noop {
                noop: "".to_string()
            },
            depends_on: None
        }
    )
}

#[test]
fn test_parse_succeeds_with_resources_and_repositories() {
    let test_file_yaml = include_str!("m8s_resources_and_repositories.yaml");

    let config: Config = serde_yaml::from_str(test_file_yaml).unwrap();
    assert_eq!(
        config
            .helm
            .as_ref()
            .and_then(|helm| helm.repositories.clone())
            .unwrap()
            .len(),
        1usize
    );
    assert_eq!(
        config
            .helm
            .as_ref()
            .and_then(|helm| helm.repositories.clone())
            .unwrap()[0],
        HelmRepository {
            name: "argo".to_string(),
            url: "https://argoproj.github.io/argo-helm".to_string()
        }
    );
}

#[test]
fn test_parse_fails_with_unknown_key_in_resource() {
    let test_file_yaml = include_str!("m8s_unknown_key_in_resource_throws_error.yaml");

    let config = serde_yaml::from_str::<Config>(test_file_yaml);
    assert!(config.is_err());
}

#[test]
fn test_parse_fails_with_unknown_resource_type() {
    let test_file_yaml = include_str!("m8s_unknown_resource_type_throws_error.yaml");

    let config = serde_yaml::from_str::<Config>(test_file_yaml);
    assert!(config.is_err());
}

#[test]
fn test_check_dependency_cycles_fails_with_a_single_resource_which_depends_on_itself() {
    let test_file_yaml = include_str!("m8s_cycle_on_self.yaml");

    let config: Config = serde_yaml::from_str(test_file_yaml).unwrap();
    assert_eq!(
        "Configuration is invalid, dependency cycle for \"foobarNoop\": foobarNoop -> foobarNoop",
        check_dependency_cycles(&config.resources)
            .err()
            .unwrap()
            .to_string()
    );
}

#[test]
fn test_check_dependency_cycles_fails_with_two_dependencies_that_depends_on_each_other() {
    let test_file_yaml = include_str!("m8s_cycle_on_one.yaml");

    let config: Config = serde_yaml::from_str(test_file_yaml).unwrap();
    assert_eq!(
        "Configuration is invalid, dependency cycle for \"foobarNoop\": foobarNoop -> foobazNoop -> foobarNoop",
        check_dependency_cycles(&config.resources)
            .err()
            .unwrap()
            .to_string()
    );
}

#[test]
fn test_check_invalid_resource_keys_fails_when_one_or_more_dependencies_do_not_exist() {
    let test_file_yaml = include_str!("m8s_depends_not_exists.yaml");

    let config: Config = serde_yaml::from_str(test_file_yaml).unwrap();
    assert_eq!(
        "Configuration is invalid, invalid dependencies: doesNotExist1, doesNotExist2",
        check_invalid_resource_keys(&config.resources)
            .err()
            .unwrap()
            .to_string()
    );
}

#[test]
fn test_check_invalid_resource_keys_fails_when_a_dependency_refers_to_outside_group() {
    let test_file_yaml = include_str!("m8s_depends_outside_group.yaml");

    let config: Config = serde_yaml::from_str(test_file_yaml).unwrap();
    assert_eq!(
        "Configuration is invalid, invalid dependencies: foobarGroup, helloWorld",
        check_invalid_resource_keys(&config.resources)
            .err()
            .unwrap()
            .to_string()
    );
}

#[test]
fn test_check_resource_keys_format_fails_when_key_is_not_alphanumeric() {
    let test_file_yaml = include_str!("m8s_invalid_key_format.yaml");

    let config: Config = serde_yaml::from_str(test_file_yaml).unwrap();
    assert_eq!(
        "Configuration is invalid, resource key can only contain [a-zA-Z0-9]: not:valid",
        check_resource_keys_format(&config.resources)
            .err()
            .unwrap()
            .to_string()
    );
}

#[test]
fn test_check_helm_remote_repositories_with_no_repositories_passes() {
    let resources = indexmap! {};
    let helm_repositories = None;
    check_helm_remote_repositories(&resources, &helm_repositories).unwrap();

    let resources = indexmap! {};
    let helm_repositories = Some(vec![]);
    check_helm_remote_repositories(&resources, &helm_repositories).unwrap();
}

#[test]
fn test_check_helm_remote_repositories_with_valid_repository_passes() {
    let resources = indexmap! {
        "foobar".to_string() => ResourceWithDepdencies {
            resource: HelmRemoteResource {
                helm_remote: HelmRemote {
                    name: "my-release-name".to_string(),
                    namespace: "default".to_string(),
                    chart_name: "example/example".to_string(),
                    chart_version: "1.2.3".to_string(),
                    values: None,
                }
            },
            depends_on: None,
        }
    };
    let helm_repositories = Some(vec![HelmRepository {
        name: "example".to_string(),
        url: "https://example.com".to_string(),
    }]);
    check_helm_remote_repositories(&resources, &helm_repositories).unwrap();
}

#[test]
fn test_check_helm_remote_repositories_with_invalid_chart_name_fails() {
    let resources = indexmap! {
        "foobar".to_string() => ResourceWithDepdencies {
            resource: HelmRemoteResource {
                helm_remote: HelmRemote {
                    name: "my-release-name".to_string(),
                    namespace: "default".to_string(),
                    chart_name: "invalid".to_string(),
                    chart_version: "1.2.3".to_string(),
                    values: None,
                }
            },
            depends_on: None,
        }
    };
    let helm_repositories = Some(vec![HelmRepository {
        name: "example".to_string(),
        url: "https://example.com".to_string(),
    }]);
    let err = check_helm_remote_repositories(&resources, &helm_repositories)
        .err()
        .unwrap();
    assert!(err
        .to_string()
        .contains("chart name \"invalid\" doesn't start with a repository name"));
}

#[test]
fn test_check_helm_remote_repositories_with_invalid_repo_in_chart_name_fails() {
    let resources = indexmap! {
        "foobar".to_string() => ResourceWithDepdencies {
            resource: HelmRemoteResource {
                helm_remote: HelmRemote {
                    name: "my-release-name".to_string(),
                    namespace: "default".to_string(),
                    chart_name: "invalid/example".to_string(),
                    chart_version: "1.2.3".to_string(),
                    values: None,
                }
            },
            depends_on: None,
        }
    };
    let helm_repositories = Some(vec![]);
    let err = check_helm_remote_repositories(&resources, &helm_repositories)
        .err()
        .unwrap();
    assert!(err
        .to_string()
        .contains("repository with name \"invalid\" doesn't exist"));
}

#[test]
fn test_check_helm_remote_repositories_with_undefined_repository_fails() {
    let resources = indexmap! {
        "foobar".to_string() => ResourceWithDepdencies {
            resource: HelmRemoteResource {
                helm_remote: HelmRemote {
                    name: "my-release-name".to_string(),
                    namespace: "default".to_string(),
                    chart_name: "example/example".to_string(),
                    chart_version: "1.2.3".to_string(),
                    values: None,
                }
            },
            depends_on: None,
        }
    };
    let helm_repositories = None;
    let err = check_helm_remote_repositories(&resources, &helm_repositories)
        .err()
        .unwrap();
    assert!(err.to_string().contains("no repositories configured"))
}

#[test]
fn test_check_files_exist_with_no_files_passes() {
    let resources = indexmap! {};
    check_files_exist(&resources).unwrap()
}

#[test]
fn test_check_files_exist_with_valid_manifest_files_passes() {
    let resources = indexmap! {
        "foobar".to_string() => ResourceWithDepdencies {
            resource: ManifestResource {
                manifest: Manifest {
                    path: "tests/manifest.yaml".to_string()
                }
            },
            depends_on: None,
        }
    };
    check_files_exist(&resources).unwrap()
}

#[test]
fn test_check_files_exist_with_invalid_manifest_files_fails() {
    let resources = indexmap! {
        "foobar".to_string() => ResourceWithDepdencies {
            resource: ManifestResource {
                manifest: Manifest {
                    path: "invalid-manifest.yaml".to_string()
                }
            },
            depends_on: None,
        }
    };
    let err = check_files_exist(&resources).err().unwrap();
    assert!(err
        .to_string()
        .contains("references file that doesn't exist"))
}

#[test]
fn test_check_files_exist_with_valid_helm_remote_files_passes() {
    let resources = indexmap! {
        "foobar".to_string() => ResourceWithDepdencies {
            resource: HelmRemoteResource {
                helm_remote: HelmRemote {
                    name: "my-release-name".to_string(),
                    namespace: "default".to_string(),
                    chart_name: "example/example".to_string(),
                    chart_version: "1.2.3".to_string(),
                    values: Some(vec![
                        "tests/chart/values.yaml".to_string()
                    ]),
                }
            },
            depends_on: None,
        }
    };
    check_files_exist(&resources).unwrap();
}

#[test]
fn test_check_files_exist_with_invalid_helm_remote_files_fails() {
    let resources = indexmap! {
        "foobar".to_string() => ResourceWithDepdencies {
            resource: HelmRemoteResource {
                helm_remote: HelmRemote {
                    name: "my-release-name".to_string(),
                    namespace: "default".to_string(),
                    chart_name: "example/example".to_string(),
                    chart_version: "1.2.3".to_string(),
                    values: Some(vec![
                        "invalid-values.yaml".to_string()
                    ]),
                }
            },
            depends_on: None,
        }
    };
    let err = check_files_exist(&resources).err().unwrap();
    assert!(err
        .to_string()
        .contains("references file that doesn't exist"))
}

#[test]
fn test_check_files_exist_with_valid_helm_local_files_passes() {
    let resources = indexmap! {
        "foobar".to_string() => ResourceWithDepdencies {
            resource: HelmLocalResource {
                helm_local: HelmLocal {
                    name: "my-release-name".to_string(),
                    namespace: "default".to_string(),
                    values: Some(vec![
                        "tests/chart/values.yaml".to_string()
                    ]),
                    chart_path: "tests/chart".to_string()
                }
            },
            depends_on: None,
        }
    };
    check_files_exist(&resources).unwrap();
}

#[test]
fn test_check_files_exist_with_invalid_helm_local_values_fails() {
    let resources = indexmap! {
        "foobar".to_string() => ResourceWithDepdencies {
            resource: HelmLocalResource {
                helm_local: HelmLocal {
                    name: "my-release-name".to_string(),
                    namespace: "default".to_string(),
                    values: Some(vec![
                        "invalid-values.yaml".to_string()
                    ]),
                    chart_path: "tests/chart".to_string()
                }
            },
            depends_on: None,
        }
    };
    let err = check_files_exist(&resources).err().unwrap();
    assert!(err
        .to_string()
        .contains("references file that doesn't exist"))
}

#[test]
fn test_check_files_exist_with_invalid_helm_local_chart_path_fails() {
    let resources = indexmap! {
        "foobar".to_string() => ResourceWithDepdencies {
            resource: HelmLocalResource {
                helm_local: HelmLocal {
                    name: "my-release-name".to_string(),
                    namespace: "default".to_string(),
                    values: Some(vec![
                        "tests/chart/values.yaml".to_string()
                    ]),
                    chart_path: "invalid".to_string()
                }
            },
            depends_on: None,
        }
    };
    let err = check_files_exist(&resources).err().unwrap();
    assert!(err
        .to_string()
        .contains("references directory that doesn't exist"))
}
