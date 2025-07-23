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
  version = "0.9.5";

  src = ./.;

  cargoHash = "sha256-AGh1y89TpEWcz86KiaIRReBm7fLbG+sHBValhdsS3m4=";

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
