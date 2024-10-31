# Local Helm chart

`m8s` can deploy Helm charts that live in a directory on your disk.

```yaml
resources:
  # Structure
  <resource_name>:
    helmLocal:
      name: <helm_release_name>
      namespace: <kubernetes_namespace>
      chartPath: <path_to_chart>
      values:
        - <path_to_values_file_a>
        - <path_to_values_file_b>

  # Concrete example
  myHelmChart:
    helmLocal:
      name: my-helm-chart
      namespace: my-custom-namespace
      chartPath: ./charts/my-helm-chart/
      values:
        - path/to/values-ingress-nginx.yaml
```
