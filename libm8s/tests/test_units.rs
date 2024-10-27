use libm8s::file_format::Config;
use libm8s::units::run_units;

#[test]
fn test_run_units_goes_through_units_to_run_them() {
    let test_file_yaml = include_str!("m8s_only_units.yaml");
    let config: Config = serde_yaml::from_str(test_file_yaml).unwrap();

    let skip_dependencies = false;
    let kubeconfig = None;
    let dry_run = true;
    assert_eq!(
        (),
        run_units(
            &config.units,
            None,
            vec![],
            skip_dependencies,
            kubeconfig,
            dry_run
        )
        .unwrap()
    );
}
