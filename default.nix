{
  lib,
  rustPlatform,
  pkg-config,
  openssl,
  systemd,
  zlib,
  stdenv,
}:

rustPlatform.buildRustPackage rec {
  pname = "rex";
  version = "0.9.5"; # Update as needed

  src = ./.;

  cargoHash = ""; # Run nix build first, it will tell you the correct hash

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    openssl
    systemd
    zlib
    stdenv.cc.cc.lib
  ];

  # Set up runtime environment
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
    homepage = "https://github.com/yourusername/rex"; # Update this
    license = licenses.gpl3; # Update as appropriate
    maintainers = with maintainers; [
      Jamin
      Martin
    ];
    platforms = platforms.linux;
  };
}
