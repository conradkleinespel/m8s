use libm8s::file_format::{Config, HelmRepository, Unit, UnitWithDependencies};

#[test]
fn test_parse_only_units() {
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
fn test_parse_units_repositories_and_root() {
    let test_file_yaml = include_str!("m8s_units_repositories_and_root.yaml");

    let config: Config = serde_yaml::from_str(test_file_yaml).unwrap();
    assert_eq!(config.root, Some("path/to/root".to_string()));
    assert_eq!(config.helm_repositories.clone().unwrap().len(), 1usize);
    assert_eq!(
        config.helm_repositories.clone().unwrap()[0],
        HelmRepository {
            name: "argo".to_string(),
            url: "https://argoproj.github.io/argo-helm".to_string()
        }
    );
}
