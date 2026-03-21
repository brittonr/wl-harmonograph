{
  description = "Animated mathematical wallpaper for Sway/Wayland";

  inputs.nixpkgs.url = "nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      lastModifiedDate = self.lastModifiedDate or self.lastModified or "19700101";
      version = builtins.substring 0 8 lastModifiedDate;
      forAllSystems = nixpkgs.lib.genAttrs nixpkgs.lib.systems.flakeExposed;
      nixpkgsFor = forAllSystems (system: import nixpkgs { inherit system; });
    in {
      packages = forAllSystems (system:
        let
          pkgs = nixpkgsFor.${system};
        in rec {
          wl-walls = pkgs.rustPlatform.buildRustPackage {
            pname = "wl-walls";
            inherit version;

            src = ./.;

            # Update after changing dependencies: nix build 2>&1 | grep 'got:'
            cargoHash = "sha256-5QoBXDfiPNRUVIj9f+YLCr3r+e17c15A8VCZPXFnTZE=";

            nativeBuildInputs = with pkgs; [
              pkg-config
            ];

            buildInputs = with pkgs; [
              wayland
              libGL
              libglvnd
            ];

            # khronos-egl with "static" feature links EGL statically,
            # but still needs the GL driver at runtime.
            postFixup = ''
              patchelf --add-rpath ${pkgs.lib.makeLibraryPath [
                pkgs.libglvnd
                pkgs.mesa
              ]} $out/bin/wl-walls
              # wl-walls-ctl doesn't need GL rpath
            '';

            meta = with pkgs.lib; {
              description = "Animated mathematical wallpaper for Sway/Wayland";
              license = licenses.mit;
              platforms = platforms.linux;
              mainProgram = "wl-walls";
            };
          };

          wl-walls-ctl = wl-walls.overrideAttrs (_: {
            meta.mainProgram = "wl-walls-ctl";
          });

          noctalia-plugin = pkgs.stdenvNoCC.mkDerivation {
            pname = "wl-walls-noctalia-plugin";
            inherit version;
            src = ./noctalia-plugin;
            dontBuild = true;
            installPhase = ''
              mkdir -p $out/share/noctalia/plugins/wl-walls
              cp -r $src/* $out/share/noctalia/plugins/wl-walls/
            '';
            meta = with pkgs.lib; {
              description = "Noctalia settings plugin for wl-walls";
              license = licenses.mit;
              platforms = platforms.linux;
            };
          };

          default = wl-walls;
        });
    };
}
