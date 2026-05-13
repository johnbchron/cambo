{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    devshell.url = "github:numtide/devshell";
  };

  outputs = { nixpkgs, rust-overlay, devshell, flake-utils, ... }: 
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [
          (import rust-overlay)
          devshell.overlays.default
        ];
      };

      toolchain_fn = p: p.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
        extensions = [ "rust-src" "rust-analyzer" ];
      });

      packages = (with pkgs; [
        pkg-config clang mold

        alsa-lib udev

        libxkbcommon wayland
        # xorg.libX11 xorg.libXcursor xorg.libXi xorg.libXrandr

        vulkan-headers vulkan-loader
        vulkan-tools vulkan-tools-lunarg
        vulkan-extension-layer
        # vulkan-validation-layers

        fontconfig
      ]) ++ [ (toolchain_fn pkgs) ];
    in {
      devShell = pkgs.devshell.mkShell {
        inherit packages;
        motd = "\n  Welcome to the {2}$(basename $PRJ_ROOT){reset} shell.\n";
        env = [
          { name = "LD_LIBRARY_PATH"; value = pkgs.lib.makeLibraryPath packages; }
        ];
      };
  });
}
