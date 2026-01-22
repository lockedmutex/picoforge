{ pkgs ? import <nixpkgs> { } }:

let
  libraries = with pkgs; [
    at-spi2-atk
    atkmm
    cairo
    gdk-pixbuf
    glib
    gtk3
    harfbuzz
    librsvg
    libsoup_3
    pango
    webkitgtk_4_1
    pcsclite
    hidapi
    mesa 
  ];

  packages = with pkgs; [
    curl
    wget
    pkg-config
    dbus
    openssl_3
    librsvg
    git
    
    # Development tools
    rustc
    cargo
    deno
    nodejs_22
    
    # Tauri 2 dependencies
    at-spi2-atk
    atkmm
    cairo
    gdk-pixbuf
    glib
    gtk3
    harfbuzz
    librsvg
    libsoup_3
    pango
    webkitgtk_4_1
    
    # Hardware
    pcsclite
    hidapi
    udev
  ];
in
pkgs.mkShell {
  buildInputs = packages;

  shellHook = ''
    export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath libraries}:$LD_LIBRARY_PATH
    export XDG_DATA_DIRS=$GSETTINGS_SCHEMAS_PATH:$XDG_DATA_DIRS
    
    # Try to uncomment the following lines if you encounter EGL_BAD_PARAMETER errors:
    # export LIBGL_ALWAYS_SOFTWARE=1
    # export WEBKIT_DISABLE_COMPOSITING_MODE=1
    
    echo "Nix development environment loaded!"
    echo "Available tools: rustc, cargo, deno, node, tauri"
  '';
}
