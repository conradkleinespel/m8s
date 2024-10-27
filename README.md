# `m8s`

[![Rust](https://github.com/conradkleinespel/m8s/actions/workflows/rust.yaml/badge.svg)](https://github.com/conradkleinespel/m8s/actions/workflows/rust.yaml)

`m8s`, pronounced "mates", is a tool for declarative k8s deployment using Kubectl, Helm and more.

## Why use `m8s`?

`m8s` is helpful in the following situations :

- you want a complete and declarative representation of your k8s resources, 
- you want to deploy from a terminal,
- you don't have something like ArgoCD,
- you have a tool like ArgoCD that depends on other resources being deployed,
- you want to a tool that helps you prevent dependency cycles in your infrastructure.

## Usage

`m8s` needs a configuration file like the one below, we'll call it `m8s.yaml`:

```yaml
---
helm:
  repositories:
    - name: ingress-nginx
      url: https://kubernetes.github.io/ingress-nginx

resources:
  myCustomManifest:
    manifest:
      path: path/to/my-custom-manifest.yaml

  myIngressNginxDeployment:
    helmRemote:
      name: ingress-nginx
      namespace: ingress-nginx
      chartName: ingress-nginx/ingress-nginx
      chartVersion: 4.11.3
      values:
        - path/to/values-ingress-nginx.yaml
    dependsOn:
      - myCustomManifest
```

Then, call `m8s up` and `m8s` will run `kubectl`, `helm`, and other tools as needed 🚀

## In-depth documentation

The `m8s.yaml` file is the configuration file for `m8s`. It defines the Helm chart, Kubernetes manifests and other resources to be deployed. It has two main parts: Helm configuration and resources to be deployed to Kubernetes.

### Helm repositories

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

### Resources

You can define resources to be deployed in your Kubernetes cluster in the `resources` section of `m8s.yaml`:

```yaml
resources:
  <resource_name>:
    <resource_type>:
      <resource_option_a>: <value>
      <resource_option_b>: <value>
      <resource_option_c>: <value>
```

There are various resource types. You can ask `m8s` to deploy Helm charts, to run shell scripts and more. Below is the documentation for each of those resource types.

#### Kubernetes manifest

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

#### Shell script

`m8s` can run shell scripts, which can be useful when creating resources which cannot safely be checked into Git, such as secrets.

The shell script is passed to `bash -c "..."`.

```yaml
resources:
  # Structure
  <resource_name>:
    shell:
      input: <shell_script_as_string>
      
  # Concrete example
  myCustomSecret:
    shell:
      input: |
        set -eu

        if kubectl get secret my-custom-secret; then
          echo "Secret exists"
          exit
        fi

        password="$(tr -dc A-Za-z0-9 < /dev/urandom | head -c64)"
        kubectl create secret generic my-custom-secret \
          --from-literal=MY_CUSTOM_SECRET="$password"
```

#### Remote Helm chart

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

#### Local Helm chart

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

#### Resource group

`m8s` allows grouping resources into groups. This is particularly handy when you need to define dependencies on complex resources.

```yaml
resources:
  # Structure
  <resource_name_for_group>:
    group:
      <resource_name_a>:
        <resource_options>:
      <resource_name_b>:
        <resource_options>:

  # Concrete example
  webApplicationBase:
    group:
      certManager:
        ...
      ingressNginx:
        ...

  myCustomWebApplication:
    manifest:
      path: path/to/my-custom-web-application.yaml
    dependsOn:
      - webApplicationBase

  anotherWebApplication:
    manifest:
      path: path/to/my-other-web-application.yaml
    dependsOn:
      - webApplicationBase
```

#### Noop

`m8s` allows defining resources that are no-ops. This can sometimes come in handy. It is there just in case.

```yaml
resources:
  # Structure
  <resource_name>:
    noop: ""

  # Concrete example
  myNoop:
    noop: ""
```
