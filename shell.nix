{pkgs ? import <nixpkgs> {}}:
pkgs.mkShell {
  packages = with pkgs; [
    rustup
    kubectl
    kubernetes-helm
  ];

  shellHook = ''
    rustup default stable
    rustup component add rust-src

    echo "Environment loaded."
    kubectl version --client
  '';
}
