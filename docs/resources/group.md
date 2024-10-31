# Resource group

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
