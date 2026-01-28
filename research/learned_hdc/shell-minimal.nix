{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    python311
    python311Packages.numpy
    python311Packages.scipy
    python311Packages.scikit-learn
  ];

  shellHook = ''
    echo "Learned HDC Research Environment (NumPy only)"
  '';
}
