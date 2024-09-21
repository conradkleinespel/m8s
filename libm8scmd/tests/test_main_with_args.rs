use libm8scmd::main_with_args;

#[test]
fn test_main_with_args_reads_file() {
    if let Err(err) = main_with_args(vec!["m8s", "up", "--dry-run", "-C", "tests"]) {
        assert_eq!("", err.to_string());
    }
}
