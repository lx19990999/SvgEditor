use std::fs;

#[test]
fn test_svg_parsing() {
    let bytes = fs::read("/home/flutter/source/SvgEditor/设备借还.svg").unwrap();
    let mut opts = resvg::usvg::Options::default();
    opts.resources_dir = Some(std::path::PathBuf::from("/home/flutter/source/SvgEditor/"));

    let tree = resvg::usvg::Tree::from_data(&bytes, &opts).unwrap();
    let size = tree.size();
    assert!(size.width() > 0.0);
    assert!(size.height() > 0.0);
    println!("SVG size: {}x{}", size.width(), size.height());

    // Dump the tree structure
    fn dump(group: &resvg::usvg::Group, depth: usize) -> usize {
        let mut count = 0;
        for node in group.children() {
            match node {
                resvg::usvg::Node::Path(p) => {
                    let indent = "  ".repeat(depth);
                    let seg_count = p.data().segments().count();
                    let fill = p.fill().map(|f| match f.paint() {
                        resvg::usvg::Paint::Color(c) => format!("#{:02x}{:02x}{:02x}", c.red, c.green, c.blue),
                        _ => "non-color".to_string(),
                    });
                    let bb = p.bounding_box();
                    println!("{}Path id='{}' segments={} fill={:?} bbox=[{},{},{},{}]",
                        indent, p.id(), seg_count, fill,
                        bb.left(), bb.top(), bb.right(), bb.bottom());
                    count += 1;
                }
                resvg::usvg::Node::Group(g) => {
                    let indent = "  ".repeat(depth);
                    println!("{}Group id='{}' children={}", indent, g.id(), g.children().len());
                    count += dump(g, depth + 1);
                }
                _ => {
                    let indent = "  ".repeat(depth);
                    println!("{}Other", indent);
                }
            }
        }
        count
    }
    println!("Root children: {}", tree.root().children().len());
    let root_transform = tree.root().abs_transform();
    println!("Root abs_transform: sx={} sy={} tx={} ty={}",
        root_transform.sx, root_transform.sy, root_transform.tx, root_transform.ty);
    let total = dump(tree.root(), 0);
    println!("Total paths found: {}", total);
    assert!(total > 0, "Expected paths");
}
