{ pkgs, ... }:
let cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
in {
  rink = pkgs.rustPlatform.buildRustPackage {
    pname = cargoToml.package.name;
    version = cargoToml.package.version;
    src = ./.;
    cargoBuildFlags = "";
    cargoLock = {
      lockFile = ./Cargo.lock;
      outputHashes = {
        "anyrun-interface-0.1.0" =
          "sha256-hI9+KBShsSfvWX7bmRa/1VI20WGat3lDXmbceMZzMS4=";
      };
    };
    nativeBuildInputs = with pkgs; [
      pkg-config
      openssl
      openssl.dev
    ];
    PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
  };
}
