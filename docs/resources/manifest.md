# Kubernetes manifest

`m8s` can apply Kubernetes manifests using `kubectl apply -f`.

```yaml
resources:
  # Structure
  <resource_name>:
    manifest:
      path: <manifest_path>

  # Concrete example
  myCustomManifest:
    manifest:
      path: path/to/my-custom-manifest.yaml
```
