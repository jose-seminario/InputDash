use std::fs;
use std::path::Path;

/// Uncompressed 1x1 32-bit BGRA Windows ICO (70 bytes) used at compile time
/// so `tauri-build` always has a valid PE resource icon without tracking binary blobs.
const MINIMAL_ICO: [u8; 70] = [
    // ICONDIR (6 bytes): reserved=0, type=1 (ICO), count=1
    0x00, 0x00, 0x01, 0x00, 0x01, 0x00,
    // ICONDIRENTRY (16 bytes): 1x1, 0 palette, 1 plane, 32 bpp, size=48, offset=22
    0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x20, 0x00, 0x30, 0x00, 0x00, 0x00, 0x16, 0x00, 0x00, 0x00,
    // BITMAPINFOHEADER (40 bytes): size=40, width=1, height=2 (XOR+AND), planes=1, bpp=32
    0x28, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x20, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // Pixel XOR BGRA (4 bytes) + AND mask (4 bytes)
    0xFF, 0x88, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x00,
];

fn ensure_compile_icon() {
    let icon_path = Path::new("icons/icon.ico");
    if !icon_path.exists() {
        if let Some(parent) = icon_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(icon_path, MINIMAL_ICO);
    }
}

fn main() {
    ensure_compile_icon();

    let windows_attrs =
        tauri_build::WindowsAttributes::new().app_manifest(include_str!("app.manifest"));
    let attrs = tauri_build::Attributes::new().windows_attributes(windows_attrs);

    tauri_build::try_build(attrs).expect("failed to execute tauri-build");
}
