{
  lib,
  rustPlatform,
  buildNpmPackage,
  fetchFromGitHub,
  makeDesktopItem,

  cargo-tauri,
  pkg-config,

  atkmm,
  eudev,
  gdk-pixbuf,
  glib,
  gtk3,
  libgudev,
  libsoup_3,
  pango,
  pcsclite,
  webkitgtk_4_1,
}:
rustPlatform.buildRustPackage (finalAttrs: {

  pname = "picoforge";
  version = "0.2.1";

  src = fetchFromGitHub {
    owner = "librekeys";
    repo = "picoforge";
    rev = "v${finalAttrs.version}";
    hash = "sha256-bVD8CXDDiXBPDCdspk9b4Y9hSfRDH4nHGF0IIZIMb9M=";
  };

  cargoRoot = "src-tauri";

  buildAndTestSubdir = finalAttrs.cargoRoot;

  cargoHash = "sha256-nLf8v4MIt2zAeA9YMVaoI3s/yut5/Jy2fGM3Sx33EJc=";

  npmDist = buildNpmPackage {
    name = "${finalAttrs.pname}-${finalAttrs.version}-dist";
    inherit (finalAttrs) src;

    npmDepsHash = "sha256-7DLooiGLzk3JRsKAftOxSf7HAgHBXCJDaAFp2p/pryc=";

    installPhase = ''
      runHook preInstall

      mkdir -p $out
      cp -r build/* $out

      runHook postInstall
    '';
  };

  preBuild = ''
    sed -i '/beforeBuildCommand/d' src-tauri/tauri.conf.json
    cp -r ${finalAttrs.npmDist} build
  '';

  nativeBuildInputs = [
    cargo-tauri.hook
    pkg-config
  ];

  buildInputs = [
    atkmm
    eudev
    gdk-pixbuf
    glib
    gtk3
    libgudev
    libsoup_3
    pango
    pcsclite
    webkitgtk_4_1
  ];

  postInstall = ''
    install -Dm644 ${finalAttrs.src}/static/in.suyogtandel.picoforge.svg $out/share/icons/hicolor/scalable/apps/picoforge.svg
  '';

  desktopItems = [
    (makeDesktopItem {
      name = "in.suyogtandel.picoforge";
      desktopName = "PicoForge";
      exec = "picoforge";
      terminal = false;
      icon = "picoforge";
      comment = finalAttrs.meta.description;
      categories = [ "Utility" ];
      dbusActivatable = true;
      keywords = [ "Config" ];
      startupNotify = true;
    })
  ];

  meta = {
    changelog = "https://github.com/librekeys/picoforge/releases/tag/v${finalAttrs.version}";
    description = "An open source commissioning tool for Pico FIDO security keys.";
    homepage = "https://github.com/librekeys/picoforge";
    license = lib.licenses.agpl3Only;
    mainProgram = "picoforge";
  };
})
