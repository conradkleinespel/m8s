use libm8s::file_format::Config;
use libm8s::file_format::Resource::{Group, HelmLocal, HelmRemote, Manifest};
use libm8s::resources::run_resources;
use libm8s::{parse_deployment_file, FileReader};
use std::fs;
use std::path::Path;

struct MockFileReader;

impl FileReader for MockFileReader {
    fn read_to_string(&self, _: &Path) -> std::io::Result<String> {
        let config_file = include_str!("m8s_only_resources.yaml");
        Ok(config_file.to_string())
    }
}

#[test]
fn test_parse_deployment_file_with_reader_returns_config() {
    let config =
        parse_deployment_file(MockFileReader {}, &Path::new("/my/m8s/dir/m8s.yaml")).unwrap();

    match &config.resources.get("foobarManifest").unwrap().resource {
        Manifest { manifest } => {
            assert_eq!("/my/m8s/dir/path/to/manifest.yaml", manifest.path.as_str());
        }
        _ => panic!("Expected Ressource::Manifest"),
    }

    match &config.resources.get("foobarHelmLocal").unwrap().resource {
        HelmLocal { helm_local } => {
            assert_eq!(
                "/my/m8s/dir/values-local.yaml",
                helm_local.values.as_ref().unwrap().get(0).unwrap().as_str()
            );
        }
        _ => panic!("Expected Ressource::HelmLocal"),
    }

    match &config.resources.get("foobarHelmRemote").unwrap().resource {
        HelmRemote { helm_remote } => {
            assert_eq!(
                "/my/m8s/dir/values-remote.yaml",
                helm_remote
                    .values
                    .as_ref()
                    .unwrap()
                    .get(0)
                    .unwrap()
                    .as_str()
            );
        }
        _ => panic!("Expected Ressource::HelmRemote"),
    }

    match &config.resources.get("foobarGroup").unwrap().resource {
        Group { group } => match &group.get("subFoobarManifest").unwrap().resource {
            Manifest { manifest } => {
                assert_eq!("/my/m8s/dir/sub-manifest.yaml", manifest.path.as_str());
            }
            _ => panic!("Expected Ressource::Manifest"),
        },
        _ => panic!("Expected Ressource::Group"),
    }
}

#[test]
fn test_no_dependencies_enables_from_leaf_group_in_resource_args() {
    let test_file_yaml = include_str!("m8s_no_dependencies_leaf_group_in_resource_args.yaml");
    let config: Config = serde_yaml::from_str(test_file_yaml).unwrap();

    if fs::exists("tests/m8s_no_dependencies_leaf_group_in_resource_args_test_output")
        .unwrap_or(false)
    {
        fs::remove_file("tests/m8s_no_dependencies_leaf_group_in_resource_args_test_output")
            .unwrap();
    }

    run_resources(
        &config.resources,
        None,
        vec!["a".to_string()],
        false,
        None,
        false,
    )
    .unwrap();

    let output =
        fs::read_to_string("tests/m8s_no_dependencies_leaf_group_in_resource_args_test_output")
            .unwrap();
    assert_eq!("b\nc\nd\n", output);
}

#[test]
fn test_no_dependencies_disables_from_leaf_resource_with_adjacent_resources_in_resource_args() {
    let test_file_yaml = include_str!(
        "m8s_no_dependencies_leaf_resource_with_adjacent_resources_in_resource_args.yaml"
    );
    let config: Config = serde_yaml::from_str(test_file_yaml).unwrap();

    if fs::exists("tests/m8s_no_dependencies_leaf_resource_with_adjacent_resources_in_resource_args_test_output").unwrap_or(false) {
        fs::remove_file("tests/m8s_no_dependencies_leaf_resource_with_adjacent_resources_in_resource_args_test_output").unwrap();
    }

    run_resources(
        &config.resources,
        None,
        vec!["a:d".to_string()],
        false,
        None,
        false,
    )
    .unwrap();

    let output = fs::read_to_string("tests/m8s_no_dependencies_leaf_resource_with_adjacent_resources_in_resource_args_test_output").unwrap();
    assert_eq!("d\n", output);
}
