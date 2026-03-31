#!/usr/bin/env bash
# release.sh — Interactive release script for PWDManager.
#
# Usage: bash scripts/release.sh
#
# Flow:
#   1. Ask for version → update Cargo.toml + Dioxus.toml
#   2. Commit + push version bump to master
#   3. Build NSIS bundle
#   4. Code sign the installer (signtool — SmartScreen reputation)
#   5. Sign artifacts for updater (minisign)
#   6. Open editor for RELEASE_NOTES.md
#   7. Summary + confirmation
#   8. Create GitHub release

set -euo pipefail

# Keep window open on error so the user can read the message
cleanup() {
    local exit_code=$?
    if [[ $exit_code -ne 0 ]]; then
        echo ""
        error "Script exited with code $exit_code"
        echo ""
        read -rp "Press Enter to close..."
    fi
}
trap cleanup EXIT

# ── Colors ──────────────────────────────────────────────
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

step()  { echo -e "${GREEN}[✓]${NC} $1"; }
warn()  { echo -e "${YELLOW}[!]${NC} $1"; }
error() { echo -e "${RED}[✗]${NC} $1" >&2; }
info()  { echo -e "${CYAN}[→]${NC} $1"; }

die() { error "$1"; exit 1; }

# ── Project root ────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

CARGO_TOML="$PROJECT_ROOT/Cargo.toml"
DIOXUS_TOML="$PROJECT_ROOT/Dioxus.toml"
NOTES_FILE="$PROJECT_ROOT/RELEASE_NOTES.md"
BUNDLE_DIR="$PROJECT_ROOT/target/dx/PWDManager/bundle/windows/nsis"

# ── Pre-flight checks ───────────────────────────────────
if ! git rev-parse --is-inside-work-tree &>/dev/null; then
    die "Not inside a git repository"
fi

if ! command -v gh &>/dev/null; then
    die "gh CLI not found. Install it: https://cli.github.com/"
fi

if ! command -v signtool &>/dev/null; then
    die "signtool not found in PATH. Install Windows SDK."
fi

if ! gh auth status &>/dev/null 2>&1; then
    die "gh CLI not authenticated. Run: gh auth login"
fi

# Check for uncommitted changes
if ! git diff --quiet || ! git diff --cached --quiet; then
    die "Working tree is not clean. Commit or stash your changes first."
fi

CURRENT_BRANCH=$(git branch --show-current)

# ── Step 1: Ask for version ─────────────────────────────
CURRENT_VERSION=$(grep -m1 '^version' "$CARGO_TOML" | sed 's/.*"\(.*\)"/\1/')
echo -e "${CYAN}Current version: $CURRENT_VERSION${NC}"
read -rp "New version: " VERSION

if [[ -z "$VERSION" ]]; then
    die "Version cannot be empty"
fi

# ── Step 2: Switch to master first ───────────────────────
info "Committing version bump..."

ORIGINAL_BRANCH="$CURRENT_BRANCH"

if [[ "$CURRENT_BRANCH" != "master" ]]; then
    warn "You are on branch '$CURRENT_BRANCH', switching to master"
fi

git checkout master

# ── Step 3: Update version in Cargo.toml and Dioxus.toml ─
info "Updating version to $VERSION..."

sed -i "s/^version = \".*\"/version = \"$VERSION\"/" "$CARGO_TOML"
sed -i "s/^version = \".*\"/version = \"$VERSION\"/" "$DIOXUS_TOML"

step "Cargo.toml → $VERSION"
step "Dioxus.toml → $VERSION"

git add "$CARGO_TOML" "$DIOXUS_TOML"

# Check if there's actually a change (version might already be the same)
if git diff --cached --quiet; then
    warn "Version already at $VERSION on master, skipping commit"
else
    git commit -m "bump version to v$VERSION"
    git push origin master
    step "Version bump committed and pushed to master"
fi

# ── Step 4: Build NSIS bundle ───────────────────────────
info "Building NSIS bundle (release)..."
dx bundle --desktop --release --package-types "nsis"
step "NSIS bundle built"

# ── Step 5: Code sign the installer ─────────────────────
info "Code signing the installer for SmartScreen reputation..."
NSIS_EXE=$(find "$BUNDLE_DIR" -name "*.exe" -path "*nsis*" | head -1)
if [[ -z "$NSIS_EXE" ]]; then die "No NSIS .exe found in $BUNDLE_DIR"; fi

WIN_NSIS_EXE=$(cygpath -w "$NSIS_EXE")
MSYS_NO_PATHCONV=1 signtool sign /tr http://timestamp.digicert.com /td sha256 /fd sha256 /a "$WIN_NSIS_EXE"
step "Installer code-signed: $(basename "$NSIS_EXE")"

# ── Step 6: Open editor for RELEASE_NOTES.md ────────────
info "Opening editor for RELEASE_NOTES.md..."
${EDITOR:-notepad} "$NOTES_FILE"
step "Release notes saved"

# ── Step 7: Sign artifacts for updater ──────────────────
info "Signing artifacts for updater (reads RELEASE_NOTES.md for latest.json)..."
bash -x "$SCRIPT_DIR/build-updater-artifacts.sh" "$VERSION" "$BUNDLE_DIR"
step "Updater artifacts signed"

# ── Step 8: Find remaining artifact paths ───────────────
NSIS_ZIP=$(find "$BUNDLE_DIR" -name "*.nsis.zip" | head -1)
LATEST_JSON="$BUNDLE_DIR/latest.json"

if [[ -z "$NSIS_ZIP" ]]; then die "No NSIS .zip found"; fi
if [[ ! -f "$LATEST_JSON" ]]; then die "latest.json not found"; fi

# ── Step 9: Summary + confirmation ──────────────────────
echo ""
echo -e "${CYAN}════════════════════════════════════════${NC}"
echo -e "${CYAN}  Release Summary${NC}"
echo -e "${CYAN}════════════════════════════════════════${NC}"
echo "  Version:      $VERSION"
echo "  Installer:    $(basename "$NSIS_EXE")"
echo "  Update zip:   $(basename "$NSIS_ZIP")"
echo "  Manifest:     latest.json"
echo ""
echo -e "  Notes preview:"
head -5 "$NOTES_FILE" 2>/dev/null | sed 's/^/    /' || echo "    (empty)"
echo -e "${CYAN}════════════════════════════════════════${NC}"
echo ""

read -rp "Proceed with GitHub release? [y/N] " CONFIRM
if [[ "$CONFIRM" != "y" && "$CONFIRM" != "Y" ]]; then
    warn "Release cancelled by user."
    echo "Files are still built and signed in $BUNDLE_DIR"
    exit 0
fi

# ── Step 10: Create GitHub release ──────────────────────
info "Creating GitHub release v$VERSION..."
gh release create "v$VERSION" \
    --title "v$VERSION" \
    --notes-file "$NOTES_FILE" \
    "$NSIS_ZIP" \
    "$LATEST_JSON" \
    "$NSIS_EXE"

step "Release v$VERSION created!"
echo -e "${GREEN}https://github.com/LucioPg/PWDManager/releases/tag/v$VERSION${NC}"
