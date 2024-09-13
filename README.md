# `m8s`

[![Rust](https://github.com/conradkleinespel/m8s/actions/workflows/rust.yaml/badge.svg)](https://github.com/conradkleinespel/m8s/actions/workflows/rust.yaml)

`m8s`, pronounced "mates", is a tool that let's you define your Kubernetes (manifests, helm charts, etc) in a declarative way and deploy them from the command-line.

## Why use `m8s`?

`m8s` is helpful in the following situations :

- when you want a complete and declarative representation of your k8s resources, 
- when you want to deploy from a terminal,
- when you don't have something like ArgoCD,
- when you have a tool like ArgoCD that depends on other resources being deployed,
- when you want to prevent dependency cycles.

## Usage

`m8s` needs a configuration file like the one below, we'll call it `my-deployment.yaml`:

```yaml
---
helmRepositories:
  - name: ingress-nginx
    url: https://kubernetes.github.io/ingress-nginx

units:
  ingressNginxNamespace:
    manifest:
      path: manifests/ingress-nginx-namespace.yaml

  helmIngressNginx:
    helmRemote:
      name: ingress-nginx
      namespace: ingress-nginx
      chartName: ingress-nginx/ingress-nginx
      chartVersion: 4.11.2
      values:
        - values/ingress-nginx.yaml
    depends_on:
      - ingressNginxNamespace

  myWebsite:
    manifest:
      path: manifests/my-website.yaml
    depends_on:
      - helmIngressNginx
```

Then, call `m8s up my-deployment.yaml` and `m8s` will run `kubectl`, `helm`, and other tools as needed 🚀
