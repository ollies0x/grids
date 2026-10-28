#!/bin/zsh
# Build Grids.app: the Rust daemon plus the bundled SwiftUI Settings app.
#   scripts/build.sh            build into build/Grids.app
#   scripts/build.sh --install  also replace /Applications/Grids.app and relaunch it
set -euo pipefail
cd "${0:A:h}/.."
ROOT=$PWD
BUILD=$ROOT/build
APP=$BUILD/Grids.app
# The Command Line Tools ship no SwiftUI macro plugin for the macOS 27 SDK; 26.5 needs none.
SDK=/Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk
[[ -d $SDK ]] || SDK=$(xcrun --show-sdk-path)
IDENTITY=${GRIDS_SIGN:-$(security find-identity -v -p codesigning | awk -F'"' '/Apple Development/{print $2; exit}')}
VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)

cargo build --release
mkdir -p $BUILD/obj
swiftc -parse-as-library -swift-version 5 -Osize -target arm64-apple-macos14.0 -sdk $SDK apple/Settings/*.swift -o $BUILD/obj/GridsSettings
strip -x $BUILD/obj/GridsSettings
if [[ ! -f $BUILD/obj/AppIcon.icns || scripts/make-icon.swift -nt $BUILD/obj/AppIcon.icns ]]; then
  swiftc -O -sdk $SDK scripts/make-icon.swift -o $BUILD/obj/make-icon
  rm -rf $BUILD/obj/AppIcon.iconset
  $BUILD/obj/make-icon $BUILD/obj/AppIcon.iconset
  # Quantised and packed verbatim the icon is ~200 KB instead of 1.25 MB.
  if command -v pngquant >/dev/null && command -v oxipng >/dev/null; then
    pngquant --force --ext .png --quality 90-100 --speed 1 $(ls $BUILD/obj/AppIcon.iconset/*.png | grep -v "16x16.png" | grep -v "32x32.png")
    oxipng -q -o 4 --strip safe $BUILD/obj/AppIcon.iconset/*.png
    python3 scripts/pack-icns.py $BUILD/obj/AppIcon.iconset $BUILD/obj/AppIcon.icns
  else
    iconutil -c icns $BUILD/obj/AppIcon.iconset -o $BUILD/obj/AppIcon.icns
  fi
fi

# Throwaway window the self-test moves around (never shipped).
T=$BUILD/obj/TestWindow.app
if [[ ! -x $T/Contents/MacOS/TestWindow || tests/TestWindow.swift -nt $T/Contents/MacOS/TestWindow ]]; then
  mkdir -p $T/Contents/MacOS
  swiftc -O -sdk $SDK tests/TestWindow.swift -o $T/Contents/MacOS/TestWindow
  rm -f $T/Contents/Info.plist
  /usr/libexec/PlistBuddy -c 'Add :CFBundleIdentifier string app.grids.testwindow' -c 'Add :CFBundleExecutable string TestWindow' -c 'Add :CFBundlePackageType string APPL' -c 'Add :CFBundleName string Grids Test Window' $T/Contents/Info.plist >/dev/null
  codesign --force --sign - $T
fi

rm -rf $APP
SET=$APP/Contents/Helpers/"Grids Settings.app"
mkdir -p $APP/Contents/{MacOS,Resources} "$SET/Contents/"{MacOS,Resources}
cp target/release/grids $APP/Contents/MacOS/Grids
cp $BUILD/obj/AppIcon.icns $APP/Contents/Resources/AppIcon.icns
cp $BUILD/obj/AppIcon.icns "$SET/Contents/Resources/AppIcon.icns"
cp $BUILD/obj/GridsSettings "$SET/Contents/MacOS/Grids Settings"

cat > $APP/Contents/Info.plist <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleIdentifier</key><string>app.grids</string>
  <key>CFBundleName</key><string>Grids</string>
  <key>CFBundleDisplayName</key><string>Grids</string>
  <key>CFBundleExecutable</key><string>Grids</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>LSMinimumSystemVersion</key><string>14.0</string>
  <key>LSUIElement</key><true/>
  <key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
  <key>NSHumanReadableCopyright</key><string>Grids, a Rectangle-compatible window manager.</string>
  <key>CFBundleURLTypes</key><array><dict>
    <key>CFBundleURLName</key><string>app.grids</string>
    <key>CFBundleURLSchemes</key><array><string>grids</string><string>rectangle-pro</string></array>
  </dict></array>
</dict></plist>
PLIST

cat > "$SET/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleIdentifier</key><string>app.grids.settings</string>
  <key>CFBundleName</key><string>Grids Settings</string>
  <key>CFBundleExecutable</key><string>Grids Settings</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>LSMinimumSystemVersion</key><string>14.0</string>
  <key>NSPrincipalClass</key><string>NSApplication</string>
</dict></plist>
PLIST

if [[ -n $IDENTITY ]]; then
  codesign --force --options runtime --sign "$IDENTITY" "$SET"
  codesign --force --options runtime --sign "$IDENTITY" $APP
else
  codesign --force --sign - "$SET"
  codesign --force --sign - $APP
fi
echo "built $APP ($(du -sh $APP | cut -f1))"

if [[ ${1:-} == --install || ${1:-} == --install-only ]]; then
  pkill -x Grids 2>/dev/null || true
  pkill -x "Grids Settings" 2>/dev/null || true
  rm -rf /Applications/Grids.app
  cp -R $APP /Applications/
  [[ $1 == --install ]] && open /Applications/Grids.app
  echo "installed /Applications/Grids.app"
fi
