# Documentation

The `m8s.yaml` file is the configuration file for `m8s`. It defines the Helm chart, Kubernetes manifests and other resources to be deployed. It has two main parts:

- Helm configuration;
- resources to be deployed to Kubernetes.

## Helm repositories

To define Helm repositories, add a `helm.repositories` section in `m8s.yaml`:

```yaml
helm:
  repositories:
    - name: <repository_name>
      url: <repository_url>
```

A more concrete example would be:

```yaml
helm:
  repositories:
    - name: ingress-nginx
      url: https://kubernetes.github.io/ingress-nginx
```

## Resources

You can define resources to be deployed in your Kubernetes cluster in the `resources` section of `m8s.yaml`:

```yaml
resources:
  <resource_name>:
    <resource_type>:
      <resource_option_a>: <value>
      <resource_option_b>: <value>
      <resource_option_c>: <value>
```

There are various resource types. You can ask `m8s` to deploy resources using a variety of sources:

- [Kubernetes manifests](./resources/manifest.md);
- [shell scripts](./resources/shell.md);
- [local Helm charts](./resources/helm-local.md);
- [remote Helm charts](./resources/helm-remote.md).

You can also [group resources into groups](./resources/group.md).

Finally, in some cases you may want to use [noops](./resources/noop.md).
