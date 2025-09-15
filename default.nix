{
  lib,
  rustPlatform,
  pkg-config,
  openssl,
  systemd,
  zlib,
  stdenv,
  makeWrapper,
}:

rustPlatform.buildRustPackage rec {
  pname = "rex";
  version = "1.0.0";

  src = ./.;

  cargoHash = "sha256-sFE4mnERzFv2DnsjLTJv64pfxaRNefz8b1njI1pAFow=";

  nativeBuildInputs = [
    pkg-config
    makeWrapper

  ];

  buildInputs = [
    openssl
    systemd
    zlib
    stdenv.cc.cc.lib

  ];

  postInstall = ''
    wrapProgram $out/bin/rex \
      --set LD_LIBRARY_PATH "${
        lib.makeLibraryPath [
          zlib
          stdenv.cc.cc.lib
        ]
      }"
  '';

  meta = with lib; {
    description = "Rex, the rust experiment manager";
    homepage = "https://github.com/yourusername/rex";
    license = licenses.gpl3;
    maintainers = with maintainers; [
      Jamin
      Martin
    ];
    platforms = platforms.linux;
  };
}
