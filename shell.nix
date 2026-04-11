{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  packages = with pkgs; [
    kubectl
    kubernetes-helm
  ];

  shellHook = ''
    echo "Environment loaded."
    kubectl version --client
  '';
}
