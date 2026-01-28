{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    python311
    python311Packages.torch
    python311Packages.numpy
    python311Packages.scipy
    python311Packages.matplotlib
    python311Packages.scikit-learn
  ];

  shellHook = ''
    echo "Learned HDC Research Environment"
    echo "Python: $(python --version)"
    echo "PyTorch: $(python -c 'import torch; print(torch.__version__)')"
  '';
}
