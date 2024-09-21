use libm8scmd::main_with_args;
use libm8scmd::utils::with_directory;

#[test]
fn test_main_with_args_changes_and_restore_directory() {
    main_with_args(
        vec!["m8s", "up", "--dry-run", "--directory", "tests"],
        false,
    )
    .unwrap();
}

#[test]
fn test_main_with_args_reads_from_m8s_yaml_by_default() {
    let result = with_directory(Some("tests".to_string()), || {
        main_with_args(vec!["m8s", "up", "--dry-run"], false)
    });
    result.unwrap();
}

#[test]
fn test_main_with_args_reads_handles_repositories() {
    main_with_args(
        vec![
            "m8s",
            "up",
            "--dry-run",
            "--file",
            "tests/m8s_with_repositories.yaml",
        ],
        false,
    )
    .unwrap();
}
