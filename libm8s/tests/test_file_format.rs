use libm8s::file_format::{Config, Unit, UnitWithDependencies};

#[test]
fn test_parse_only_units() {
    let test_file_yaml = include_str!("file_most_basic.yaml");

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
