use libm8s::file_format::HelmRepository;
use libm8s::helm_repositories::handle_helm_repositories;

#[test]
fn test_handle_helm_repositories_runs_helm_add_and_update_for_each_repository() {
    let dry_run = true;
    assert_eq!(
        (),
        handle_helm_repositories(
            &[
                HelmRepository {
                    name: "a".to_string(),
                    url: "https://a.example.com".to_string()
                },
                HelmRepository {
                    name: "b".to_string(),
                    url: "https://b.example.com".to_string()
                }
            ],
            dry_run
        )
        .unwrap()
    );
}
