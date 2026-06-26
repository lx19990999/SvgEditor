use crate::svg_doc::SvgDoc;

/// Undo/redo history for SVG document.
pub struct History {
    states: Vec<SvgDoc>,
    current: usize,
    max_size: usize,
}

impl History {
    pub fn new(initial: SvgDoc) -> Self {
        Self {
            states: vec![initial],
            current: 0,
            max_size: 100,
        }
    }

    /// Push a new state. Discards any redo states.
    pub fn push(&mut self, doc: SvgDoc) {
        // Don't push if nothing changed
        if self.current < self.states.len()
            && doc_to_hash(&self.states[self.current]) == doc_to_hash(&doc)
        {
            return;
        }
        // Discard redo history
        self.states.truncate(self.current + 1);
        self.states.push(doc);
        self.current = self.states.len() - 1;
        // Limit size
        if self.states.len() > self.max_size {
            self.states.remove(0);
            self.current = self.current.saturating_sub(1);
        }
    }

    /// Undo: go back one step. Returns the document state.
    pub fn undo(&mut self) -> Option<&SvgDoc> {
        if self.current > 0 {
            self.current -= 1;
            Some(&self.states[self.current])
        } else {
            None
        }
    }

    /// Redo: go forward one step. Returns the document state.
    pub fn redo(&mut self) -> Option<&SvgDoc> {
        if self.current + 1 < self.states.len() {
            self.current += 1;
            Some(&self.states[self.current])
        } else {
            None
        }
    }

    /// Check if undo is possible.
    pub fn can_undo(&self) -> bool {
        self.current > 0
    }

    /// Check if redo is possible.
    pub fn can_redo(&self) -> bool {
        self.current + 1 < self.states.len()
    }
}

/// Simple hash for change detection (not cryptographically secure).
fn doc_to_hash(doc: &SvgDoc) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    doc.width.to_bits().hash(&mut hasher);
    doc.height.to_bits().hash(&mut hasher);
    doc.bg_color.hash(&mut hasher);
    for path in &doc.paths {
        path.id.hash(&mut hasher);
        path.fill_color.hash(&mut hasher);
        path.stroke_color.hash(&mut hasher);
        path.stroke_width.to_bits().hash(&mut hasher);
        path.translate_x.to_bits().hash(&mut hasher);
        path.translate_y.to_bits().hash(&mut hasher);
        path.scale_x.to_bits().hash(&mut hasher);
        path.scale_y.to_bits().hash(&mut hasher);
        path.rotation.to_bits().hash(&mut hasher);
        path.commands.len().hash(&mut hasher);
        // Hash first and last command for quick comparison
        if let Some(first) = path.commands.first() {
            hash_cmd(first, &mut hasher);
        }
        if let Some(last) = path.commands.last() {
            hash_cmd(last, &mut hasher);
        }
    }
    hasher.finish()
}

fn hash_cmd(cmd: &crate::svg_doc::PathCmd, hasher: &mut impl std::hash::Hasher) {
    use std::hash::Hash;
    match cmd {
        crate::svg_doc::PathCmd::MoveTo(x, y) => {
            0u8.hash(hasher);
            x.to_bits().hash(hasher);
            y.to_bits().hash(hasher);
        }
        crate::svg_doc::PathCmd::LineTo(x, y) => {
            1u8.hash(hasher);
            x.to_bits().hash(hasher);
            y.to_bits().hash(hasher);
        }
        crate::svg_doc::PathCmd::CurveTo(a, b, c, d, e, f) => {
            2u8.hash(hasher);
            a.to_bits().hash(hasher);
            b.to_bits().hash(hasher);
            c.to_bits().hash(hasher);
            d.to_bits().hash(hasher);
            e.to_bits().hash(hasher);
            f.to_bits().hash(hasher);
        }
        crate::svg_doc::PathCmd::QuadTo(a, b, c, d) => {
            3u8.hash(hasher);
            a.to_bits().hash(hasher);
            b.to_bits().hash(hasher);
            c.to_bits().hash(hasher);
            d.to_bits().hash(hasher);
        }
        crate::svg_doc::PathCmd::Close => {
            4u8.hash(hasher);
        }
    }
}
