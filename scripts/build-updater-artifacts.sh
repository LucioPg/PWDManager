#!/usr/bin/env bash
# build-updater-artifacts.sh
# Firma l'artefatto NSIS e genera latest.json per l'auto-update.
# Uso: ./scripts/build-updater-artifacts.sh <versione> <cartella_bundle_output>
#
# Prerequisiti:
#   - minisign installato (https://jedisct1.github.io/minisign/)
#   - .env nella root con DIOXUS_SIGNING_PRIVATE_KEY e DIOXUS_SIGNING_PRIVATE_KEY_PASSWORD
#   - Bundle NSIS gia compilato con: dx bundle --desktop --package-types "nsis" --release

set -euo pipefail

VERSION="${1:?Usage: $0 <version> <bundle_output_dir>}"
BUNDLE_DIR="${2:?Usage: $0 <version> <bundle_output_dir>}"

# Carica variabili d'ambiente dal .env
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

if [ -f "$PROJECT_ROOT/.env" ]; then
    set -a
    source "$PROJECT_ROOT/.env"
    set +a
else
    echo "ERROR: .env not found at $PROJECT_ROOT/.env"
    exit 1
fi

if [ -z "$DIOXUS_SIGNING_PRIVATE_KEY" ]; then
    echo "ERROR: DIOXUS_SIGNING_PRIVATE_KEY not set in .env"
    exit 1
fi

# Trova il file .exe NSIS nella cartella di output
NSIS_EXE=$(find "$BUNDLE_DIR" -name "*.exe" -path "*nsis*" | head -1)
if [ -z "$NSIS_EXE" ]; then
    echo "ERROR: No NSIS .exe found in $BUNDLE_DIR"
    exit 1
fi

echo "==> Found NSIS installer: $NSIS_EXE"

# Firma l'artefatto con minisign
SIG_FILE="${NSIS_EXE}.sig"
echo "==> Signing artifact..."
if [ -n "$DIOXUS_SIGNING_PRIVATE_KEY_PASSWORD" ]; then
    echo "$DIOXUS_SIGNING_PRIVATE_KEY_PASSWORD" | minisign \
        -Sm "$NSIS_EXE" \
        -s - \
        -t "PWDManager v$VERSION" \
        -x "$SIG_FILE"
else
    minisign -Sm "$NSIS_EXE" -t "PWDManager v$VERSION" -x "$SIG_FILE"
fi

# Crea lo zip per l'update (contiene solo l'installer .exe)
NSIS_ZIP="${NSIS_EXE%.*}.nsis.zip"
echo "==> Creating update zip: $NSIS_ZIP"
cp "$NSIS_EXE" "$(basename "$NSIS_EXE")"
zip -j "$NSIS_ZIP" "$(basename "$NSIS_EXE")"
rm "$(basename "$NSIS_EXE")"

# Legge la firma e la converte in base64 per latest.json
SIGNATURE_B64=$(base64 -w 0 "$SIG_FILE")

# Determina il nome file dell'exe per l'URL
EXE_BASENAME=$(basename "$NSIS_EXE")
ZIP_BASENAME=$(basename "$NSIS_ZIP")

# Genera latest.json
NOTES_FILE="$PROJECT_ROOT/RELEASE_NOTES.md"
NOTES="Release v$VERSION"
if [ -f "$NOTES_FILE" ]; then
    NOTES=$(cat "$NOTES_FILE")
fi

PUB_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

cat > "$BUNDLE_DIR/latest.json" <<EOF
{
  "version": "$VERSION",
  "notes": $(echo "$NOTES" | python3 -c 'import sys,json; print(json.dumps(sys.stdin.read().strip()))'),
  "pub_date": "$PUB_DATE",
  "platforms": {
    "windows-x86_64": {
      "signature": "$SIGNATURE_B64",
      "url": "https://github.com/LucioPg/PWDManager/releases/download/v$VERSION/$ZIP_BASENAME"
    }
  }
}
EOF

echo "==> Generated $BUNDLE_DIR/latest.json"
echo ""
echo "=== Artifacts ready for release ==="
echo "  Installer: $NSIS_EXE"
echo "  Signature: $SIG_FILE"
echo "  Update zip: $NSIS_ZIP"
echo "  Manifest: $BUNDLE_DIR/latest.json"
echo ""
echo "Upload these to GitHub Release v$VERSION:"
echo "  gh release create v$VERSION --title \"v$VERSION\" --notes-file \"$NOTES_FILE\" \\"
echo "    \"$NSIS_ZIP\" \"$BUNDLE_DIR/latest.json\""
