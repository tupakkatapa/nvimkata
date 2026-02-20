{ pkgs
, lib
}:
let
  manifest = (lib.importTOML ./Cargo.toml).package;
in
pkgs.rustPlatform.buildRustPackage rec {
  pname = manifest.name;
  inherit (manifest) version;

  nativeBuildInputs = with pkgs; [
    pkg-config
    makeWrapper
  ];

  postInstall = ''
    wrapProgram $out/bin/${pname} \
      --suffix PATH : ${lib.makeBinPath [ pkgs.neovim ]}

    mkdir -p $out/share/${pname}
    cp -r challenges $out/share/${pname}/
  '';

  src = lib.sourceByRegex ./. [
    "^Cargo.toml$"
    "^Cargo.lock$"
    "^src.*$"
    "^tests.*$"
    "^challenges.*$"
  ];

  doCheck = false;
  cargoLock.lockFile = ./Cargo.lock;
}
