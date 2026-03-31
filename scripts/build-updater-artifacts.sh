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

# Usa minisign locale se disponibile, altrimenti quello di sistema
MINISIGN="$PROJECT_ROOT/minisign.exe"
if [ ! -x "$MINISIGN" ]; then
    MINISIGN="minisign"
fi

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

# Crea lo zip per l'update (contiene solo l'installer .exe)
NSIS_ZIP="${NSIS_EXE%.*}.nsis.zip"
echo "==> Creating update zip: $NSIS_ZIP"

# Convert Unix paths to Windows paths for PowerShell on MSYS2/Git Bash
WIN_NSIS_EXE=$(cygpath -w "$NSIS_EXE")
WIN_NSIS_ZIP=$(cygpath -w "$NSIS_ZIP")
powershell -NoProfile -Command "Compress-Archive -Path '$WIN_NSIS_EXE' -DestinationPath '$WIN_NSIS_ZIP' -Force"

# Firma lo ZIP con minisign (il client scarica lo zip, quindi la firma deve essere sullo zip)
SIG_FILE="${NSIS_ZIP}.sig"
echo "==> Signing artifact..."

# Scrivi la chiave privata in un file temporaneo (minisign richiede un path, non stdin)
TMP_KEY=$(mktemp)
trap 'rm -f "$TMP_KEY"' EXIT
printf 'untrusted comment: minisign encrypted secret key\n%s\n' "$DIOXUS_SIGNING_PRIVATE_KEY" > "$TMP_KEY"

if [ -n "$DIOXUS_SIGNING_PRIVATE_KEY_PASSWORD" ]; then
    echo "$DIOXUS_SIGNING_PRIVATE_KEY_PASSWORD" | "$MINISIGN" \
        -Sm "$NSIS_ZIP" \
        -s "$TMP_KEY" \
        -t "PWDManager v$VERSION" \
        -x "$SIG_FILE"
else
    "$MINISIGN" -Sm "$NSIS_ZIP" -s "$TMP_KEY" -t "PWDManager v$VERSION" -x "$SIG_FILE"
fi

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

# Escape JSON: backslash, double quotes, newline -> \n
NOTES_JSON=$(printf '%s' "$NOTES" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' | awk '{printf "%s%s", (NR>1?"\\n":""), $0}')

cat > "$BUNDLE_DIR/latest.json" <<EOF
{
  "version": "$VERSION",
  "notes": "$NOTES_JSON",
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
