use libm8s::file_format::{
    check_dependency_cycles, check_invalid_resource_keys, check_resource_keys_format, Config,
    HelmRepository, Resource, ResourceWithDepdencies,
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
