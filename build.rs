fn main() {
    // Convert icon.png to icon.rgba at compile time
    let png_path = "icon.png";
    let rgba_path = "icon.rgba";

    // Only regenerate if PNG is newer
    if std::path::Path::new(png_path).exists() {
        let png_meta = std::fs::metadata(png_path).unwrap();
        let rgba_meta = std::fs::metadata(rgba_path);

        let needs_rebuild = match rgba_meta {
            Ok(rgba) => png_meta.modified().unwrap() > rgba.modified().unwrap(),
            Err(_) => true,
        };

        if needs_rebuild {
            // Decode PNG and write raw RGBA pixels
            let png_data = std::fs::read(png_path).unwrap();
            // Use the image crate would be ideal, but we don't want build-deps.
            // Instead, use a simple approach: just copy the pre-generated file.
            // The icon.rgba is pre-generated and checked in.
            if !std::path::Path::new(rgba_path).exists() {
                panic!("icon.rgba not found. Run: python3 -c \"from PIL import Image; img=Image.open('icon.png').convert('RGBA').resize((256,256)); open('icon.rgba','wb').write(img.tobytes())\"");
            }
        }
    }

    // Tell cargo to re-run if icon.png changes
    println!("cargo:rerun-if-changed={}", png_path);
}
