use std::collections::HashMap;

/// Supported languages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lang {
    En,
    ZhCn,
}

impl Lang {
    pub fn from_code(code: &str) -> Self {
        if code.starts_with("zh") {
            Lang::ZhCn
        } else {
            Lang::En
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::ZhCn => "zh-CN",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::ZhCn => "简体中文",
        }
    }

    pub fn all() -> &'static [Lang] {
        &[Lang::En, Lang::ZhCn]
    }
}

/// Look up a translation key for the given language.
/// Falls back to English if key not found, then to the key itself.
pub fn t(key: &str, lang: &Lang) -> &'static str {
    if let Some(translations) = TRANSLATIONS.get(key) {
        if let Some(text) = translations.get(lang) {
            return text;
        }
        if let Some(text) = translations.get(&Lang::En) {
            return text;
        }
    }
    // Key not found - return a static fallback
    "[missing]"
}

type Trans = HashMap<Lang, &'static str>;

fn tr(en: &'static str, zh: &'static str) -> Trans {
    let mut m = HashMap::new();
    m.insert(Lang::En, en);
    m.insert(Lang::ZhCn, zh);
    m
}

use std::sync::LazyLock;
static TRANSLATIONS: LazyLock<HashMap<&'static str, Trans>> = LazyLock::new(|| {
        let mut m: HashMap<&'static str, Trans> = HashMap::new();

        // File menu
        m.insert("menu.file", tr("File", "文件"));
        m.insert("menu.open", tr("Open...", "打开..."));
        m.insert("menu.save_as", tr("Save As...", "另存为..."));
        m.insert("menu.export_png", tr("Export as PNG...", "导出为 PNG..."));
        m.insert("menu.export_jpg", tr("Export as JPG...", "导出为 JPG..."));
        m.insert("menu.quit", tr("Quit", "退出"));

        // Toolbar
        m.insert("toolbar.open", tr("Open", "打开"));
        m.insert("toolbar.save", tr("Save", "保存"));
        m.insert("toolbar.zoom_in", tr("Zoom+", "放大"));
        m.insert("toolbar.zoom_out", tr("Zoom-", "缩小"));
        m.insert("toolbar.fit", tr("Fit", "适应"));
        m.insert("toolbar.undo", tr("Undo", "撤销"));
        m.insert("toolbar.redo", tr("Redo", "恢复"));
        m.insert("toolbar.draw", tr("Draw", "绘制"));
        m.insert("toolbar.drawing", tr("Drawing", "绘制中"));
        m.insert("toolbar.drawing_hint", tr("Click to add points, dbl-click/Enter to finish, Esc to cancel", "点击添加节点，双击/回车完成，Esc取消"));
        m.insert("toolbar.text", tr("Text", "文字"));
        m.insert("toolbar.texting", tr("Text input", "文字输入"));
        m.insert("toolbar.text_hint", tr("Click canvas to set position, then type and press Enter", "点击画板设置位置，输入文字后回车确认"));
        m.insert("props.text_content", tr("Text:", "文字:"));
        m.insert("props.text_size", tr("Size:", "字号:"));
        m.insert("props.text_font", tr("Font:", "字体:"));
        m.insert("props.text_style", tr("Style:", "样式:"));
        m.insert("props.text_font_count", tr("fonts available", "个可用字体"));
        m.insert("toolbar.dpi", tr("DPI", "DPI"));
        m.insert("toolbar.language", tr("Language", "语言"));
        m.insert("toolbar.theme", tr("Theme", "主题"));

        // Theme options
        m.insert("theme.system", tr("System", "跟随系统"));
        m.insert("theme.dark", tr("Dark", "深色"));
        m.insert("theme.light", tr("Light", "浅色"));

        // Path list panel
        m.insert("paths.heading", tr("Paths", "路径"));
        m.insert("paths.path_n", tr("Path", "路径"));

        // Properties panel
        m.insert("props.heading", tr("Properties", "属性"));
        m.insert("props.canvas", tr("Canvas", "画板"));
        m.insert("props.width", tr("Width", "宽度"));
        m.insert("props.height", tr("Height", "高度"));
        m.insert("props.background", tr("Background", "背景"));
        m.insert("props.clear_bg", tr("Clear background", "清除背景"));
        m.insert("props.fill", tr("Fill", "填充"));
        m.insert("props.stroke", tr("Stroke", "描边"));
        m.insert("props.stroke_width", tr("Width", "宽度"));
        m.insert("props.commands", tr("Commands", "命令数"));
        m.insert("props.source", tr("Path Source", "路径源码"));
        m.insert("props.delete_path", tr("Delete Path", "删除路径"));
        m.insert("props.transform", tr("Transform", "变换"));
        m.insert("props.translate", tr("Position", "位置"));
        m.insert("props.scale", tr("Scale", "缩放"));
        m.insert("props.rotation", tr("Rotation", "旋转"));
        m.insert("props.pivot", tr("Pivot", "锚点"));
        m.insert("props.locked", tr("Locked: X and Y scale together", "已锁定：XY 等比缩放"));
        m.insert("props.unlocked", tr("Unlocked: X and Y scale independently", "已解锁：XY 独立缩放"));
        m.insert("props.no_selection", tr("No path selected", "未选中路径"));
        m.insert("props.click_to_select", tr("Click a path in the canvas or list to select it.", "点击画板或列表中的路径以选中。"));
        m.insert("props.view", tr("View", "视图"));
        m.insert("props.zoom", tr("Zoom", "缩放"));
        m.insert("props.reset_view", tr("Reset View", "重置视图"));

        // Status bar
        m.insert("status.ready", tr("Ready. Use File > Open to load an SVG.", "就绪。使用 文件 > 打开 加载 SVG。"));
        m.insert("status.loading", tr("Loading…", "正在加载…"));
        m.insert("status.loaded", tr("Loaded", "已加载"));
        m.insert("status.selected", tr("Selected", "已选中"));
        m.insert("status.saved", tr("Saved", "已保存"));
        m.insert("status.parse_error", tr("Parse error", "解析错误"));
        m.insert("status.read_error", tr("Read error", "读取错误"));
        m.insert("status.failed_load", tr("Failed to load SVG.", "加载 SVG 失败。"));
        m.insert("status.failed_read", tr("Failed to read file.", "读取文件失败。"));
        m.insert("status.save_error", tr("Save error", "保存错误"));
        m.insert("status.undo", tr("Undo", "已撤销"));
        m.insert("status.redo", tr("Redo", "已恢复"));
        m.insert("status.new_path", tr("New path created", "已创建新路径"));

        // Welcome screen
        m.insert("welcome.title", tr("SVG Editor", "SVG 编辑器"));
        m.insert("welcome.no_file", tr("No file loaded.", "未加载文件。"));
        m.insert("welcome.open_svg", tr("Open SVG File...", "打开 SVG 文件..."));
        m.insert("welcome.drag_drop", tr("Drag & drop an SVG file or use File > Open", "拖放 SVG 文件或使用 文件 > 打开"));

        // Language names
        m.insert("lang.en", tr("English", "English"));
        m.insert("lang.zh_cn", tr("Chinese (Simplified)", "简体中文"));

        m
    });
