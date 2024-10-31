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

Read the [full documentation](./docs/README.md). Or import the [JSON schema](./schema.json) into your IDE for autocomplete and validation.

## License

The source code is released under the Apache 2.0 license.
