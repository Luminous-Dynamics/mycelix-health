# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
# Commercial licensing: see COMMERCIAL_LICENSE.md at repository root{ pkgs ? import <nixpkgs> {} }:

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
