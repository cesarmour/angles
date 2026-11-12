#!/bin/sh
# Monta o aplicativo para Mac a partir da compilação otimizada: ./bundle.sh
set -e
DIR="$(cd "$(dirname "$0")" && pwd)"
APP="$DIR/dist/Point of Entry.app"
mkdir -p "$APP/Contents/MacOS/assets" "$APP/Contents/Resources"
cp "$DIR/target/release/ponto-de-entrada" "$APP/Contents/MacOS/ponto-de-entrada"
rsync -a --delete "$DIR/assets/tex" "$DIR/assets/models" "$APP/Contents/MacOS/assets/"
cp "$DIR/assets/icon/AppIcon.icns" "$APP/Contents/Resources/AppIcon.icns"
cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Point of Entry</string>
  <key>CFBundleDisplayName</key><string>Point of Entry</string>
  <key>CFBundleIdentifier</key><string>local.pontodeentrada.cqb</string>
  <key>CFBundleExecutable</key><string>ponto-de-entrada</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.9.0</string>
  <key>CFBundleVersion</key><string>10</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSApplicationCategoryType</key><string>public.app-category.simulation-games</string>
</dict>
</plist>
PLIST
du -sh "$APP" | cut -f1
echo "Aplicativo em: $APP"
