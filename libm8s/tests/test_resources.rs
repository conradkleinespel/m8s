use libm8s::file_format::Config;
use libm8s::resources::run_resources;

#[test]
fn test_run_resources_goes_through_resources_to_run_them() {
    let test_file_yaml = include_str!("m8s_only_resources.yaml");
    let config: Config = serde_yaml::from_str(test_file_yaml).unwrap();

    let skip_dependencies = false;
    let kubeconfig = None;
    let dry_run = true;
    assert_eq!(
        (),
        run_resources(
            &config.resources,
            None,
            vec![],
            skip_dependencies,
            kubeconfig,
            dry_run
        )
        .unwrap()
    );
}
