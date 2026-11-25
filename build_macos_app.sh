#!/usr/bin/env bash
set -euo pipefail

APP_NAME="RustTube"
BUNDLE_ID="com.rusttube.studio"
VERSION="1.0.0"
PROJECT_DIR="$(cd "$(dirname "$0")" && pwd)"
DIST_DIR="$PROJECT_DIR/dist"
APP_BUNDLE="$DIST_DIR/$APP_NAME.app"
DMG_PATH="$DIST_DIR/$APP_NAME-$VERSION-macOS.dmg"

echo "🔨 Building $APP_NAME (release)..."
cd "$PROJECT_DIR"
cargo build --release

echo "🎨 Generating macOS App Icon (.icns)..."
ICONSET_DIR="$(mktemp -d)/AppIcon.iconset"
mkdir -p "$ICONSET_DIR"
BASE_PNG="$ICONSET_DIR/base_1024.png"

python3 - "$BASE_PNG" <<'PYEOF'
import math, struct, sys, zlib

W = H = 1024
out_path = sys.argv[1]

# Rounded squircle icon with gradient red background and white play triangle
def clamp(v, lo, hi):
    return lo if v < lo else (hi if v > hi else v)

def sd_rounded_box(px, py, bx, by, r):
    qx = abs(px) - bx + r
    qy = abs(py) - by + r
    return math.hypot(max(qx, 0.0), max(qy, 0.0)) + min(max(qx, qy), 0.0) - r

def point_in_triangle(px, py, ax, ay, bx, by, cx, cy):
    d1 = (px - bx) * (ay - by) - (ax - bx) * (py - by)
    d2 = (px - cx) * (by - cy) - (bx - cx) * (py - cy)
    d3 = (px - ax) * (cy - ay) - (cx - ax) * (py - ay)
    has_neg = (d1 < 0) or (d2 < 0) or (d3 < 0)
    has_pos = (d1 > 0) or (d2 > 0) or (d3 > 0)
    return not (has_neg and has_pos)

rows = []
cx0, cy0 = W * 0.5, H * 0.5
half_box = 420.0
corner_r = 188.0

# Play triangle vertices (slightly shifted right for optical centering)
ax, ay = cx0 - 115.0, cy0 - 185.0
bx, by = cx0 - 115.0, cy0 + 185.0
tx, ty = cx0 + 185.0, cy0

for y in range(H):
    row = bytearray([0])  # filter type 0
    t_grad = y / float(H - 1)
    # Vibrant YouTube-studio red/crimson vertical gradient
    r_bg = int(255 * (1.0 - t_grad) + 210 * t_grad)
    g_bg = int(68 * (1.0 - t_grad) + 24 * t_grad)
    b_bg = int(102 * (1.0 - t_grad) + 58 * t_grad)
    for x in range(W):
        d = sd_rounded_box(x - cx0, y - cy0, half_box, half_box, corner_r)
        if d > 1.5:
            row.extend((0, 0, 0, 0))
            continue
        alpha = int(clamp((1.0 - d) * 255.0, 0.0, 255.0))
        if point_in_triangle(x, y, ax, ay, bx, by, tx, ty):
            row.extend((255, 255, 255, alpha))
        else:
            row.extend((r_bg, g_bg, b_bg, alpha))
    rows.append(bytes(row))

raw = b"".join(rows)

def png_chunk(tag, data):
    return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

ihdr = struct.pack(">IIBBBBB", W, H, 8, 6, 0, 0, 0)
idat = zlib.compress(raw, 6)
png = b"\x89PNG\r\n\x1a\n" + png_chunk(b"IHDR", ihdr) + png_chunk(b"IDAT", idat) + png_chunk(b"IEND", b"")
with open(out_path, "wb") as f:
    f.write(png)
PYEOF

for size in 16 32 128 256 512; do
    sips -z $size $size "$BASE_PNG" --out "$ICONSET_DIR/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    sips -z $double $double "$BASE_PNG" --out "$ICONSET_DIR/icon_${size}x${size}@2x.png" >/dev/null
done
rm -f "$BASE_PNG"

echo "📦 Assembling $APP_BUNDLE..."
rm -rf "$APP_BUNDLE"
mkdir -p "$APP_BUNDLE/Contents/MacOS" "$APP_BUNDLE/Contents/Resources"

cp "$PROJECT_DIR/target/release/rusttube" "$APP_BUNDLE/Contents/MacOS/$APP_NAME"
chmod +x "$APP_BUNDLE/Contents/MacOS/$APP_NAME"

iconutil -c icns "$ICONSET_DIR" -o "$APP_BUNDLE/Contents/Resources/AppIcon.icns"
rm -rf "$(dirname "$ICONSET_DIR")"

cat > "$APP_BUNDLE/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundleDisplayName</key>
    <string>$APP_NAME</string>
    <key>CFBundleIdentifier</key>
    <string>$BUNDLE_ID</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>$APP_NAME</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
EOF

echo "🔏 Ad-hoc code signing $APP_BUNDLE..."
codesign --force --deep --sign - "$APP_BUNDLE"

echo "💿 Creating DMG installer ($DMG_PATH)..."
rm -f "$DMG_PATH"
STAGING_DIR="$(mktemp -d)"
cp -R "$APP_BUNDLE" "$STAGING_DIR/"
ln -s /Applications "$STAGING_DIR/Applications"
hdiutil create -volname "$APP_NAME" -srcfolder "$STAGING_DIR" -ov -format UDZO "$DMG_PATH" >/dev/null
rm -rf "$STAGING_DIR"

echo "✅ Done!"
echo "   App Bundle: $APP_BUNDLE"
echo "   DMG File:   $DMG_PATH"
