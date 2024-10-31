# Remote Helm chart

`m8s` can deploy Helm charts hosted on a remote repository, defined in the `helm.repositories` section.

```yaml
helm:
  repositories:
    # Structure
    - name: <repo_name>
      url: <repo_url>
    # Concrete example
    - name: ingress-nginx
      url: https://kubernetes.github.io/ingress-nginx

resources:
  # Structure
  <resource_name>:
    helmRemote:
      name: <helm_release_name>
      namespace: <kubernetes_namespace>
      chartName: <helm_repo_name>/<helm_chart_name>
      chartVersion: <helm_repo_version>
      values:
        - <path_to_values_file_a>
        - <path_to_values_file_b>

  # Concrete example
  myIngressNginxDeployment:
    helmRemote:
      name: ingress-nginx
      namespace: ingress-nginx
      chartName: ingress-nginx/ingress-nginx
      chartVersion: 4.11.3
      values:
        - path/to/values-ingress-nginx.yaml
```
