#!/bin/sh
# Installs Sojourner like an app: the release binary, the desktop entry
# and the icon, and the voice's files where the app looks for them
# (src/voice/files.rs). Per user by default, under ~/.local; a prefix and
# a staging directory can be given, which is what a package build wants.
#
#     tools/install.sh                 # into ~/.local
#     tools/install.sh --remove        # take it out again
#     PREFIX=/usr DESTDIR=./pkg tools/install.sh
#
# Run from the repository root, with the voice model and ONNX Runtime in
# place under assets/ (see README.md). Builds the release binary first.
set -eu

PREFIX="${PREFIX:-$HOME/.local}"
DESTDIR="${DESTDIR:-}"
ID=io.github.hmrdsmoke.Sojourner
BIN="$DESTDIR$PREFIX/bin"
APPS="$DESTDIR$PREFIX/share/applications"
ICONS="$DESTDIR$PREFIX/share/icons/hicolor/scalable/apps"
# The voice's files: per user beside the app's other data, system-wide
# under share/ and lib/, matching where the app looks.
case "$PREFIX" in
    "$HOME"/*) DATA="$DESTDIR${XDG_DATA_HOME:-$HOME/.local/share}/sojourner"; LIB="$DATA/lib" ;;
    *) DATA="$DESTDIR$PREFIX/share/sojourner"; LIB="$DESTDIR$PREFIX/lib/sojourner" ;;
esac

if [ "${1:-}" = "--remove" ]; then
    rm -f "$BIN/sojourner" "$APPS/$ID.desktop" "$ICONS/$ID.svg"
    rm -rf "$DATA/voices" "$DATA/espeak-ng-data" "$LIB"
    rmdir "$DATA" 2>/dev/null || true
    echo "Sojourner removed from $PREFIX"
    exit 0
fi

[ -f Cargo.toml ] && grep -q '^name = "sojourner"' Cargo.toml || { echo "run this from the repository root" >&2; exit 1; }

# Cargo's target directory, wherever it is.
TARGET="${CARGO_TARGET_DIR:-target}"
echo "Building the release binary…"
cargo build --release

# The voice: the model beside its committed card and config, and ONNX
# Runtime's library (the real file, not the symlink), from assets/.
VOICE=assets/voices
ORT=$(ls assets/onnxruntime/*/lib/libonnxruntime.so 2>/dev/null | head -n 1 || true)
if [ ! -f "$VOICE/en_US-ljspeech-high.onnx" ] || [ -z "$ORT" ]; then
    echo "The voice model or ONNX Runtime is missing under assets/; the app will install without a voice." >&2
fi
# espeak-ng's data, as espeak-rs-sys built it for the release profile.
ESPEAK=$(ls -dt "$TARGET"/release/build/espeak-rs-sys-*/out/share/espeak-ng-data 2>/dev/null | head -n 1 || true)

mkdir -p "$BIN" "$APPS" "$ICONS" "$DATA/voices" "$LIB"
install -m 755 "$TARGET/release/sojourner" "$BIN/sojourner"
install -m 644 res/icons/hicolor/scalable/apps/$ID.svg "$ICONS/$ID.svg"
# The desktop entry runs the binary by its full path, so it works whether
# or not the bin directory is on PATH.
sed "s|^Exec=.*|Exec=$PREFIX/bin/sojourner|" res/$ID.desktop > "$APPS/$ID.desktop"
chmod 644 "$APPS/$ID.desktop"
if [ -f "$VOICE/en_US-ljspeech-high.onnx" ]; then
    install -m 644 "$VOICE/en_US-ljspeech-high.onnx" "$VOICE/en_US-ljspeech-high.onnx.json" "$VOICE/en_US-ljspeech-high.MODEL_CARD" "$DATA/voices/"
fi
if [ -n "$ORT" ]; then
    cp -L "$ORT" "$LIB/libonnxruntime.so"
    chmod 644 "$LIB/libonnxruntime.so"
fi
if [ -n "$ESPEAK" ]; then
    rm -rf "$DATA/espeak-ng-data"
    cp -r "$ESPEAK" "$DATA/espeak-ng-data"
else
    echo "No espeak-ng-data under $TARGET/release; the voice will not work from the installed app." >&2
fi

# Let the desktop notice the new entry and icon, where the tools exist.
if [ -z "$DESTDIR" ]; then
    command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPS" 2>/dev/null || true
    command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -q -t "$DESTDIR$PREFIX/share/icons/hicolor" 2>/dev/null || true
fi

echo "Sojourner installed under $PREFIX"
echo "  $BIN/sojourner"
echo "  $APPS/$ID.desktop"
echo "  $ICONS/$ID.svg"
echo "  $DATA  (voices, espeak-ng-data)"
echo "  $LIB/libonnxruntime.so"
