{
  lib,
  stdenvNoCC,
  makeWrapper,
  src,
  rev,
  # Core dependencies
  quickshell,
  qt6Packages,
  # Python with required packages
  python3,
  # Runtime dependencies
  wl-clipboard,
  cliphist,
  fd,
  fzf,
  xdg-utils,
  libnotify,
  gtk3,
  libpulseaudio,
  jq,
  libqalculate,
  gnome-desktop,
  procps,
  coreutils,
  bash,
  # Optional plugin dependencies
  zoxide,
  tesseract,
  imagemagick,
  slurp,
  wf-recorder,
  bitwarden-cli,
  ydotool,
  # Fonts
  material-symbols,
  nerd-fonts,
}: let
  # Font packages for hamr UI
  fonts = [
    material-symbols
    nerd-fonts.jetbrains-mono
  ];
  pythonEnv = python3.withPackages (ps:
    with ps; [
      click
      loguru
      tqdm
      pygobject3
    ]);

  # Extract base version from source, append commit hash
  baseVersion = builtins.elemAt (builtins.match ".*VERSION=\"([0-9.]+)\".*" (builtins.readFile "${src}/hamr")) 0;
  version = "${baseVersion}+${rev}";
in
  stdenvNoCC.mkDerivation {
    pname = "hamr";
    inherit src version;

    nativeBuildInputs = [makeWrapper];

    # Runtime dependencies that need to be in PATH
    runtimeDeps = [
      quickshell
      qt6Packages.qt5compat
      pythonEnv
      wl-clipboard
      cliphist
      fd
      fzf
      xdg-utils
      libnotify
      gtk3
      libpulseaudio
      jq
      libqalculate
      gnome-desktop
      procps
      coreutils
      bash
      # Plugin dependencies
      zoxide
      tesseract
      imagemagick
      slurp
      wf-recorder
      bitwarden-cli
      ydotool
    ];

    installPhase = ''
      runHook preInstall

      # Install to /etc/xdg/quickshell/hamr (system-wide quickshell config)
      mkdir -p $out/etc/xdg/quickshell/$pname
      cp -r modules services plugins scripts assets defaults $out/etc/xdg/quickshell/$pname/
      cp *.qml $out/etc/xdg/quickshell/$pname/

      # Install hamr command with wrapper
      mkdir -p $out/bin
      cp hamr $out/bin/$pname
      chmod +x $out/bin/$pname

      wrapProgram $out/bin/$pname \
        --prefix PATH : ${lib.makeBinPath runtimeDeps} \
        --prefix QML2_IMPORT_PATH : "${qt6Packages.qt5compat}/lib/qt-6/qml:${qt6Packages.qtmultimedia}/lib/qt-6/qml" \
        --prefix XDG_DATA_DIRS : "${lib.concatMapStringsSep ":" (f: "${f}/share") fonts}" \
        --set XDG_CONFIG_DIRS "$out/etc/xdg''${XDG_CONFIG_DIRS:+:$XDG_CONFIG_DIRS}"

      # Install systemd user service
      mkdir -p $out/lib/systemd/user
      cp hamr.service $out/lib/systemd/user/hamr.service

      runHook postInstall
    '';

    meta = with lib; {
      description = "Extensible launcher for Hyprland and Niri built with Quickshell";
      homepage = "https://github.com/Stewart86/hamr";
      license = licenses.gpl3Plus;
      maintainers = [];
      platforms = platforms.linux;
      mainProgram = "hamr";
    };
  }
