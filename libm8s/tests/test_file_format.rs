use libm8s::file_format::{Config, Unit, UnitWithDependencies};

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
