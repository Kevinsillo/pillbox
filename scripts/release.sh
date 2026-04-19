#!/usr/bin/env bash
set -e

BUMP="${1:-patch}"
CARGO="core/Cargo.toml"
CHANGELOG="CHANGELOG.md"
DATE=$(date +%Y-%m-%d)

if [[ "$BUMP" != "patch" && "$BUMP" != "minor" && "$BUMP" != "major" ]]; then
  echo "Uso: $0 [patch|minor|major]"
  exit 1
fi

# Leer versión actual
CURRENT=$(grep '^version' "$CARGO" | head -1 | sed 's/.*"\(.*\)"/\1/')
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT"

case "$BUMP" in
  patch) PATCH=$((PATCH + 1)) ;;
  minor) MINOR=$((MINOR + 1)); PATCH=0 ;;
  major) MAJOR=$((MAJOR + 1)); MINOR=0; PATCH=0 ;;
esac

NEXT="$MAJOR.$MINOR.$PATCH"

echo "Releasing $CURRENT → $NEXT"

# Actualizar Cargo.toml
sed -i "s/^version = \"$CURRENT\"/version = \"$NEXT\"/" "$CARGO"

# Mover [Unreleased] → [X.Y.Z] en CHANGELOG
sed -i "s/^## \[Unreleased\]/## [Unreleased]\n\n## [$NEXT] — $DATE/" "$CHANGELOG"

# Commit y tag
git add "$CARGO" "$CHANGELOG"
git commit -m "chore(release): v$NEXT"
git tag "v$NEXT"

echo ""
echo "✓ v$NEXT listo. Para publicar:"
echo "  git push && git push --tags"
