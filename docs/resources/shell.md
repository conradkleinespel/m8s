# Shell script

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
