use std::sync::{Arc, OnceLock};

static FONTDB: OnceLock<Arc<resvg::usvg::fontdb::Database>> = OnceLock::new();

/// Shared system font database, loaded once per process.
pub fn shared_fontdb() -> Arc<resvg::usvg::fontdb::Database> {
    FONTDB
        .get_or_init(|| {
            let mut fontdb = resvg::usvg::fontdb::Database::new();
            let count_before = fontdb.len();
            fontdb.load_system_fonts();
            log::info!(
                "fontdb: before={}, after={}",
                count_before,
                fontdb.len()
            );
            Arc::new(fontdb)
        })
        .clone()
}

/// usvg options with the shared font database.
pub fn usvg_opts() -> resvg::usvg::Options<'static> {
    let mut opts = resvg::usvg::Options::default();
    opts.fontdb = shared_fontdb();
    opts
}

/// Sorted list of system font family names.
pub fn font_family_names() -> Vec<String> {
    let fontdb = shared_fontdb();
    let mut families = std::collections::BTreeSet::new();
    for face in fontdb.faces() {
        for (name, _lang) in &face.families {
            families.insert(name.clone());
        }
    }
    let result: Vec<String> = families.into_iter().collect();
    log::info!("Font families found: {}", result.len());
    if result.is_empty() {
        log::warn!("No font families found!");
    }
    result
}
