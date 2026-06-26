fn main() {
    // Convert icon.png to icon.rgba at compile time (for Linux/macOS runtime icon)
    let png_path = "icon.png";
    let rgba_path = "icon.rgba";

    if std::path::Path::new(png_path).exists() {
        let png_meta = std::fs::metadata(png_path).unwrap();
        let needs_rebuild = match std::fs::metadata(rgba_path) {
            Ok(rgba) => png_meta.modified().unwrap() > rgba.modified().unwrap(),
            Err(_) => true,
        };

        if needs_rebuild && !std::path::Path::new(rgba_path).exists() {
            panic!(
                "icon.rgba not found. Run: python3 -c \"\
                from PIL import Image; \
                img=Image.open('icon.png').convert('RGBA').resize((256,256)); \
                open('icon.rgba','wb').write(img.tobytes())\""
            );
        }
    }

    println!("cargo:rerun-if-changed={}", png_path);
    println!("cargo:rerun-if-changed=icon.ico");
    println!("cargo:rerun-if-changed=icon.rc");

    // Embed Windows icon resource
    #[cfg(target_os = "windows")]
    {
        if std::path::Path::new("icon.rc").exists() {
            let mut res = winres::WindowsResource::new();
            res.set_icon("icon.ico");
            res.set("FileDescription", "SVG Editor");
            res.set("ProductName", "SVG Editor");
            res.set("LegalCopyright", "MIT");
            res.compile().expect("Failed to compile Windows resource");
        }
    }
}
