use libm8s::file_format::{
    check_dependency_cycles, check_invalid_unit_keys, check_unit_keys_format, Config,
    HelmRepository, Unit, UnitWithDependencies,
};

#[test]
fn test_parse_succeeds_with_only_units() {
    let test_file_yaml = include_str!("m8s_only_units.yaml");

    let config: Config = serde_yaml::from_str(test_file_yaml).unwrap();
    assert_eq!(
        config.units.get("foobarNoop").unwrap(),
        &UnitWithDependencies {
            unit: Unit::Noop {
                noop: "".to_string()
            },
            depends_on: None
        }
    )
}

#[test]
fn test_parse_succeeds_with_units_and_repositories() {
    let test_file_yaml = include_str!("m8s_units_and_repositories.yaml");

    let config: Config = serde_yaml::from_str(test_file_yaml).unwrap();
    assert_eq!(config.helm_repositories.clone().unwrap().len(), 1usize);
    assert_eq!(
        config.helm_repositories.clone().unwrap()[0],
        HelmRepository {
            name: "argo".to_string(),
            url: "https://argoproj.github.io/argo-helm".to_string()
        }
    );
}

#[test]
fn test_check_dependency_cycles_fails_with_a_single_unit_which_depends_on_itself() {
    let test_file_yaml = include_str!("m8s_cycle_on_self.yaml");

    let config: Config = serde_yaml::from_str(test_file_yaml).unwrap();
    assert_eq!(
        "Configuration is invalid, dependency cycle for \"foobarNoop\": foobarNoop -> foobarNoop",
        check_dependency_cycles(&config.units)
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
        check_dependency_cycles(&config.units)
            .err()
            .unwrap()
            .to_string()
    );
}

#[test]
fn test_check_invalid_unit_keys_fails_when_one_or_more_dependencies_do_not_exist() {
    let test_file_yaml = include_str!("m8s_depends_not_exists.yaml");

    let config: Config = serde_yaml::from_str(test_file_yaml).unwrap();
    assert_eq!(
        "Configuration is invalid, invalid dependencies: doesNotExist1, doesNotExist2",
        check_invalid_unit_keys(&config.units)
            .err()
            .unwrap()
            .to_string()
    );
}

#[test]
fn test_check_invalid_unit_keys_fails_when_a_dependency_refers_to_outside_group() {
    let test_file_yaml = include_str!("m8s_depends_outside_group.yaml");

    let config: Config = serde_yaml::from_str(test_file_yaml).unwrap();
    assert_eq!(
        "Configuration is invalid, invalid dependencies: foobarGroup, helloWorld",
        check_invalid_unit_keys(&config.units)
            .err()
            .unwrap()
            .to_string()
    );
}

#[test]
fn test_check_unit_keys_format_fails_when_key_is_not_alphanumeric() {
    let test_file_yaml = include_str!("m8s_invalid_key_format.yaml");

    let config: Config = serde_yaml::from_str(test_file_yaml).unwrap();
    assert_eq!(
        "Configuration is invalid, unit key can only contain [a-zA-Z0-9]: not:valid",
        check_unit_keys_format(&config.units)
            .err()
            .unwrap()
            .to_string()
    );
}
