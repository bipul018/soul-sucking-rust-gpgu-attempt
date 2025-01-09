# { pkgs ? import <nixpkgs>{} }:
{ pkgs ? import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/394571358ce82dff7411395829aa6a3aad45b907.tar.gz") {} }:
let
in
pkgs.mkShell {
  buildInputs = [
    #pkgs.clang
    #pkgs.gdb
    #pkgs.glibc
    #pkgs.criterion # test framework library for C/C++

    #pkgs.libffi

    #pkgs.vulkan-volk
    pkgs.shaderc
    pkgs.vulkan-loader
    #pkgs.vulkan-tools
    pkgs.vulkan-headers
    #pkgs.vulkan-caps-viewer
    #pkgs.vulkan-tools-lunarg
    pkgs.vulkan-validation-layers
    #pkgs.vulkan-utility-libraries
    #pkgs.vk-bootstrap
    #pkgs.gfxreconstruct # this is a 'command replayer'

    pkgs.rustfmt
    pkgs.rustc
    pkgs.cargo
    pkgs.libclang
    pkgs.clang_18
  ];

  propagatedBuildInputs = [
  ];

  # LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
  VK_LAYER_PATH = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";
  XDG_DATA_DIRS = builtins.getEnv "XDG_DATA_DIRS";
  XDG_RUNTIME_DIR = builtins.getEnv "XDG_RUNTIME_DIR";

  
  
  # TODO:: Need to fix this having to hardcode /clang/18
  shellHook = ''
    # Set up any environment variables or paths needed for your application
    export LIBRARY_PATH=$LIBRARY_PATH
    export LD_LIBRARY_PATH=$LD_LIBRARY_PATH
    export PATH=$PATH
    export C_INCLUDE_PATH=$C_INCLUDE_PATH
    export C_INCLUDE_PATH=${pkgs.vulkan-headers}/include:$C_INCLUDE_PATH
    export INCLUDE=${pkgs.vulkan-headers}/include:$INCLUDE
    export C_INCLUDE_PATH=${pkgs.libclang.lib}/lib/clang/18/include:$C_INCLUDE_PATH
    export INCLUDE=${pkgs.libclang.lib}/lib/clang/18/include:$INCLUDE
    export LIBRARY_PATH=${pkgs.vulkan-loader}/lib:$LIBRARY_PATH
    export LIBRARY_PATH=${pkgs.vulkan-validation-layers}/lib:$LD_LIBRARY_PATH
    export VK_LAYER_PATH="${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d"; 
    export LD_LIBRARY_PATH=${pkgs.vulkan-validation-layers}/lib:$LD_LIBRARY_PATH
    export LD_LIBRARY_PATH=${pkgs.vulkan-loader}/lib:$LD_LIBRARY_PATH
    export LIBCLANG_PATH=${pkgs.libclang.lib}/lib
    export TMPDIR=$XDG_RUNIME_DIR
    unset WAYLAND_DISPLAY 
  '';
}
