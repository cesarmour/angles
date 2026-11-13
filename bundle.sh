#!/bin/sh
# Monta o aplicativo para Mac a partir da compilação otimizada: ./bundle.sh
set -e
DIR="$(cd "$(dirname "$0")" && pwd)"
APP="$DIR/dist/Angles.app"
# LOCAL=1 ./bundle.sh monta a versão de uso pessoal em dist/local/, com os modelos de assets/local/ (que não podem ser
# redistribuídos). A versão publicada sai sempre de dist/Angles.app, que nunca recebe essa pasta.
[ "$LOCAL" = "1" ] && APP="$DIR/dist/local/Angles.app"
# Os recursos (modelos, texturas) vão em Contents/Resources; em Contents/MacOS fica só o executável. Com arquivos de dados em
# Contents/MacOS a assinatura do pacote não fecha e o macOS recusa o app baixado como "danificado".
rm -rf "$APP/Contents/MacOS/assets" "$APP/Contents/_CodeSignature"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources/assets"
cp "$DIR/target/release/ponto-de-entrada" "$APP/Contents/MacOS/ponto-de-entrada"
# texturas, modelos e as falas do instrutor (só os .wav; o roteiro em texto já vai dentro do executável)
rsync -a --delete "$DIR/assets/tex" "$DIR/assets/models" "$APP/Contents/Resources/assets/"
rm -rf "$APP/Contents/Resources/assets/local"
[ "$LOCAL" = "1" ] && [ -d "$DIR/assets/local" ] && rsync -a "$DIR/assets/local" "$APP/Contents/Resources/assets/"
mkdir -p "$APP/Contents/Resources/assets/voice" && rsync -a --delete --include="*.wav" --exclude="*" "$DIR/assets/voice/" "$APP/Contents/Resources/assets/voice/"
cp "$DIR/assets/icon/AppIcon.icns" "$APP/Contents/Resources/AppIcon.icns"
cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Angles</string>
  <key>CFBundleDisplayName</key><string>Angles</string>
  <key>CFBundleIdentifier</key><string>local.pontodeentrada.cqb</string>
  <key>CFBundleExecutable</key><string>ponto-de-entrada</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.12.0</string>
  <key>CFBundleVersion</key><string>16</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSApplicationCategoryType</key><string>public.app-category.simulation-games</string>
</dict>
</plist>
PLIST
# assinatura local (sem conta de desenvolvedor da Apple): sela o pacote inteiro, para ele chegar íntegro a quem baixa
find "$APP" -name ".DS_Store" -delete
xattr -cr "$APP"
# Assina com o certificado Developer ID se houver um neste Mac (com "hardened runtime" e carimbo de tempo, que a
# notarização da Apple exige); sem certificado, assinatura ad hoc (só serve para rodar aqui).
SIGN_ID="$(security find-identity -v -p codesigning 2>/dev/null | grep -o '"Developer ID Application[^"]*"' | head -1 | tr -d '"')"
if [ -n "$SIGN_ID" ]; then
  codesign --force --deep --options runtime --timestamp --sign "$SIGN_ID" "$APP" && echo "Assinado com: $SIGN_ID"
else
  codesign --force --deep --sign - "$APP" 2>/dev/null
fi
codesign --verify --deep --strict "$APP" && echo "Assinatura do pacote: ok"
du -sh "$APP" | cut -f1
echo "Aplicativo em: $APP"
