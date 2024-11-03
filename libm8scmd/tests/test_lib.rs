use libm8scmd::Cli;

#[test]
fn test_main_with_args_for_command_up_changes_and_restores_directory() {
    Cli::main_with_args(
        vec!["m8s", "up", "--dry-run", "--directory", "tests"],
        false,
    )
    .unwrap();
}
#[test]
fn test_main_with_args_for_json_schema_runs() {
    Cli::main_with_args(vec!["m8s", "json-schema"], false).unwrap();
}
