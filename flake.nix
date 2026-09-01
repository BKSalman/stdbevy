{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    { nixpkgs, rust-overlay, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
        config.allowUnfree = true;
      };

      buildInputs = with pkgs; [
        cairo
        gdk-pixbuf
        gtk3
        pango
        expat
        pkg-config
        glib

        fontconfig
        freetype
        freetype.dev

        libGL
        vulkan-headers vulkan-loader
        vulkan-tools vulkan-tools-lunarg
        vulkan-extension-layer
        vulkan-validation-layers

        libxkbcommon
        # WINIT_UNIX_BACKEND=wayland
        wayland

        # WINIT_UNIX_BACKEND=x11
        libX11
        libXcursor
        libXi
        libXrandr
        # bzip2

        alsa-lib
        pipewire

        rustPlatform.bindgenHook

        openssl
        udev
      ];
    in
    with pkgs;
    {
      devShells.${system}.default = mkShell {

        packages = [
          (rust-bin.stable.latest.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
            ];
            targets = [ "wasm32-unknown-unknown" ];
          })

          cargo-watch
          # binaryen

          spacetimedb
        ];

        nativeBuildInputs = with pkgs; [
          pkg-config
          cmake
          mesa
        ];

        inherit buildInputs;

        LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";
      };

      formatter.x86_64-linux = legacyPackages.${system}.nixpkgs-fmt;
    };
}
