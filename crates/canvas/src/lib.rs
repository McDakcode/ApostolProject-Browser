// Made by MrDuck && Ox-Alpha
//! apb-canvas
//!
//! Infinite canvas document model (design doc §15). Documents are stored as
//! plain JSON files (`.canvas.json`) in the profile's `canvas/` directory —
//! no proprietary lock-in: users can open them in any text editor and diff
//! them in git. This crate owns the data model, undo/redo, geometry helpers
//! and SVG export; the UI editor renders the same JSON live.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum CanvasError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("document not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, CanvasError>;

// ---------------------------------------------------------------------------
// Geometry primitives
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x <= self.x + self.w && p.y >= self.y && p.y <= self.y + self.h
    }

    /// Union of two rects (used for bounding-box computation).
    pub fn union(&self, other: &Rect) -> Rect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let x2 = (self.x + self.w).max(other.x + other.w);
        let y2 = (self.y + self.h).max(other.y + other.h);
        Rect::new(x, y, x2 - x, y2 - y)
    }

    /// Inflate by a margin on all sides (hit-test tolerance).
    pub fn inflate(&self, m: f64) -> Rect {
        Rect::new(self.x - m, self.y - m, self.w + 2.0 * m, self.h + 2.0 * m)
    }
}

/// Shortest distance from `p` to segment `a-b`.
fn point_segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let abx = b.x - a.x;
    let aby = b.y - a.y;
    let len_sq = abx * abx + aby * aby;
    if len_sq == 0.0 {
        return ((p.x - a.x).powi(2) + (p.y - a.y).powi(2)).sqrt();
    }
    let t = (((p.x - a.x) * abx + (p.y - a.y) * aby) / len_sq).clamp(0.0, 1.0);
    let cx = a.x + t * abx;
    let cy = a.y + t * aby;
    ((p.x - cx).powi(2) + (p.y - cy).powi(2)).sqrt()
}

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ElementKind {
    Rectangle { fill: String },
    Ellipse { fill: String },
    Line { points: Vec<Point>, stroke_width: f64 },
    Arrow { points: Vec<Point>, stroke_width: f64 },
    Freehand { points: Vec<Point>, stroke_width: f64 },
    Text { content: String, font_size: f64 },
    Sticky { content: String, color: String },
    /// Reference to an image file in the profile's canvas assets dir —
    /// never inline big base64 blobs into the document (keeps it diffable).
    Image { asset_path: String },
    LinkCard { url: String, title: String },
    Frame { title: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Element {
    pub id: Uuid,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub rotation_deg: f64,
    pub stroke: String,
    pub z: i64,
    #[serde(flatten)]
    pub kind: ElementKind,
}

impl Element {
    pub fn new(x: f64, y: f64, w: f64, h: f64, kind: ElementKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            x,
            y,
            w,
            h,
            rotation_deg: 0.0,
            stroke: "#1f2933".to_string(),
            z: 0,
            kind,
        }
    }

    /// Bounding box in world space. Rotation is ignored here — an axis-
    /// aligned approximation is good enough for hit-testing and exports.
    pub fn bounds(&self) -> Rect {
        Rect::new(self.x, self.y, self.w, self.h)
    }

    /// True if `p` hits this element (bounds for shapes, stroke distance
    /// for freeform paths).
    pub fn hit_test(&self, p: Point) -> bool {
        match &self.kind {
            ElementKind::Rectangle { .. }
            | ElementKind::Text { .. }
            | ElementKind::Sticky { .. }
            | ElementKind::Image { .. }
            | ElementKind::LinkCard { .. }
            | ElementKind::Frame { .. } => self.bounds().contains(p),
            ElementKind::Ellipse { .. } => {
                let b = self.bounds();
                if b.w <= 0.0 || b.h <= 0.0 {
                    return false;
                }
                let cx = b.x + b.w / 2.0;
                let cy = b.y + b.h / 2.0;
                let dx = (p.x - cx) / (b.w / 2.0);
                let dy = (p.y - cy) / (b.h / 2.0);
                dx * dx + dy * dy <= 1.0
            }
            ElementKind::Line { points, stroke_width }
            | ElementKind::Arrow { points, stroke_width }
            | ElementKind::Freehand { points, stroke_width } => {
                let tol = stroke_width.max(4.0) / 2.0 + 3.0;
                points.windows(2).any(|seg| point_segment_distance(p, seg[0], seg[1]) <= tol)
                    // single click-point paths still hittable around their anchor
                    || (points.len() == 1 && self.bounds().inflate(tol).contains(p))
            }
        }
    }
}

/// A directed connection between two elements (mind-map edges).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Connector {
    pub id: Uuid,
    pub from: Uuid,
    pub to: Uuid,
}

/// Viewport maps world space <-> screen space (pan + zoom).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Viewport {
    pub pan_x: f64,
    pub pan_y: f64,
    pub zoom: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self { pan_x: 0.0, pan_y: 0.0, zoom: 1.0 }
    }
}

impl Viewport {
    pub fn world_to_screen(&self, p: Point) -> Point {
        Point::new((p.x + self.pan_x) * self.zoom, (p.y + self.pan_y) * self.zoom)
    }

// Made by MrDuck && Ox-Alpha
    pub fn screen_to_world(&self, p: Point) -> Point {
        Point::new(p.x / self.zoom - self.pan_x, p.y / self.zoom - self.pan_y)
    }
}

/// One infinite canvas document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub name: String,
    pub elements: Vec<Element>,
    pub connectors: Vec<Connector>,
    pub viewport: Viewport,
}

impl Document {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            elements: Vec::new(),
            connectors: Vec::new(),
            viewport: Viewport::default(),
        }
    }

    pub fn add_element(&mut self, mut el: Element) -> &mut Element {
        el.z = self.elements.iter().map(|e| e.z).max().unwrap_or(0) + 1;
        self.elements.push(el);
        self.elements.last_mut().expect("just pushed")
    }

    pub fn remove_element(&mut self, id: Uuid) -> bool {
        let before = self.elements.len();
        self.elements.retain(|e| e.id != id);
        // Drop dangling connectors too.
        self.connectors.retain(|c| c.from != id && c.to != id);
        before != self.elements.len()
    }

    /// Topmost element under `p`, if any.
    pub fn hit_test(&self, p: Point) -> Option<&Element> {
        self.elements
            .iter()
            .filter(|e| e.hit_test(p))
            .max_by_key(|e| e.z)
    }

    pub fn connect(&mut self, from: Uuid, to: Uuid) -> Connector {
        let c = Connector { id: Uuid::new_v4(), from, to };
        self.connectors.push(c.clone());
        c
    }

    /// Union bounds of everything, or None for an empty document.
    pub fn content_bounds(&self) -> Option<Rect> {
        let mut acc: Option<Rect> = None;
        for el in &self.elements {
            let b = el.bounds();
            acc = Some(match acc {
                Some(r) => r.union(&b),
                None => b,
            });
        }
        acc
    }
}

// ---------------------------------------------------------------------------
// Undo / redo
// ---------------------------------------------------------------------------

/// An atomic edit operation. Storing both sides lets us invert in place.
#[derive(Debug, Clone)]
pub enum Op {
    Add(Element),
    Remove(Element),
    Move { id: Uuid, from: (f64, f64), to: (f64, f64) },
    Resize { id: Uuid, from: (f64, f64), to: (f64, f64) },
    SetText { id: Uuid, from: String, to: String },
}

fn set_text_target(el: &mut Element, text: String) -> Option<String> {
    match &mut el.kind {
        ElementKind::Text { content, .. } => Some(std::mem::replace(content, text)),
        ElementKind::Sticky { content, .. } => Some(std::mem::replace(content, text)),
        _ => None,
    }
}

/// Unbounded-but-practical undo history (caller may cap depth).
#[derive(Debug, Default)]
pub struct UndoStack {
    undo: Vec<Op>,
    redo: Vec<Op>,
}

impl UndoStack {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an already-applied change without touching the document.
    pub fn push(&mut self, op: Op) {
        self.undo.push(op);
        self.redo.clear();
    }

    /// Apply `op` to `doc` AND record it — use this instead of mutating the
    /// document yourself so history stays consistent.
    pub fn commit(&mut self, doc: &mut Document, op: Op) -> bool {
        if apply_op(doc, &op) {
            self.push(op);
            true
        } else {
            false
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self, doc: &mut Document) -> bool {
        let Some(op) = self.undo.pop() else {
            return false;
        };
        let inv = invert_op(&op);
        if apply_op(doc, &inv) {
            self.redo.push(op);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self, doc: &mut Document) -> bool {
        let Some(op) = self.redo.pop() else {
            return false;
        };
        if apply_op(doc, &op) {
            self.undo.push(op);
            true
        } else {
            false
        }
    }
}

fn invert_op(op: &Op) -> Op {
    match op {
        Op::Add(el) => Op::Remove(el.clone()),
        Op::Remove(el) => Op::Add(el.clone()),
        Op::Move { id, from, to } => Op::Move { id: *id, from: *to, to: *from },
        Op::Resize { id, from, to } => Op::Resize { id: *id, from: *to, to: *from },
        Op::SetText { id, from, to } => Op::SetText { id: *id, from: to.clone(), to: from.clone() },
    }
}

/// Apply an operation directly. Returns false when it no longer applies
/// (e.g. the target element was deleted out-of-band).
fn apply_op(doc: &mut Document, op: &Op) -> bool {
    match op {
        Op::Add(el) => {
            let mut el = el.clone();
            el.z = doc.elements.iter().map(|e| e.z).max().unwrap_or(0) + 1;
            doc.elements.push(el);
            true
        }
        Op::Remove(el) => doc.remove_element(el.id),
        Op::Move { id, to, .. } => match doc.elements.iter_mut().find(|e| e.id == *id) {
            Some(el) => {
                el.x = to.0;
                el.y = to.1;
                true
            }
            None => false,
        },
        Op::Resize { id, to, .. } => match doc.elements.iter_mut().find(|e| e.id == *id) {
            Some(el) => {
                el.w = to.0;
                el.h = to.1;
                true
            }
            None => false,
        },
        Op::SetText { id, to, .. } => match doc.elements.iter_mut().find(|e| e.id == *id) {
            Some(el) => set_text_target(el, to.clone()).is_some(),
            None => false,
        },
    }
}

// ---------------------------------------------------------------------------
// SVG export
// ---------------------------------------------------------------------------

// Made by MrDuck && Ox-Alpha
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn polyline(points: &[Point], stroke: &str, stroke_width: f64) -> String {
    let pts: Vec<String> = points.iter().map(|p| format!("{:.1},{:.1}", p.x, p.y)).collect();
    format!(
        r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="{:.1}" stroke-linecap="round" stroke-linejoin="round"/>"#,
        pts.join(" "),
        xml_escape(stroke),
        stroke_width.max(1.0)
    )
}

fn element_to_svg(el: &Element) -> String {
    match &el.kind {
        ElementKind::Rectangle { fill } => format!(
            r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{}" stroke="{}"/>"#,
            el.x, el.y, el.w, el.h, xml_escape(fill), xml_escape(&el.stroke)
        ),
        ElementKind::Ellipse { fill } => format!(
            r#"<ellipse cx="{:.1}" cy="{:.1}" rx="{:.1}" ry="{:.1}" fill="{}" stroke="{}"/>"#,
            el.x + el.w / 2.0,
            el.y + el.h / 2.0,
            el.w / 2.0,
            el.h / 2.0,
            xml_escape(fill),
            xml_escape(&el.stroke)
        ),
        ElementKind::Line { points, stroke_width } => polyline(points, &el.stroke, *stroke_width),
        ElementKind::Arrow { points, stroke_width } => {
            let mut out = polyline(points, &el.stroke, *stroke_width);
            if let (Some(tail), Some(prev)) = (points.last(), points.len().checked_sub(2).and_then(|i| points.get(i))) {
                let ang = (tail.y - prev.y).atan2(tail.x - prev.x);
                let len = 10.0_f64.max(*stroke_width * 3.0);
                let spread = 0.42;
                let p1 = Point::new(
                    tail.x - len * (ang - spread).cos(),
                    tail.y - len * (ang - spread).sin(),
                );
                let p2 = Point::new(
                    tail.x - len * (ang + spread).cos(),
                    tail.y - len * (ang + spread).sin(),
                );
                out.push_str(&format!(
                    r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="{}"/>"#,
                    tail.x, tail.y, p1.x, p1.y, p2.x, p2.y, xml_escape(&el.stroke)
                ));
            }
            out
        }
        ElementKind::Freehand { points, stroke_width } => polyline(points, &el.stroke, *stroke_width),
        ElementKind::Text { content, font_size } => format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="{:.1}" font-family="system-ui, sans-serif" fill="{}">{}</text>"#,
            el.x,
            el.y + font_size,
            font_size,
            xml_escape(&el.stroke),
            xml_escape(content)
        ),
        ElementKind::Sticky { content, color } => format!(
            r##"<g><rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{}" stroke="#c9a227"/><text x="{:.1}" y="{:.1}" font-size="13" font-family="system-ui, sans-serif" fill="#222222" xml:space="preserve">{}</text></g>"##,
            el.x, el.y, el.w, el.h, xml_escape(color), el.x + 8.0, el.y + 20.0, xml_escape(content)
        ),
        ElementKind::Image { asset_path } => format!(
            r#"<image x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" href="{}"/>"#,
            el.x, el.y, el.w, el.h, xml_escape(asset_path)
        ),
        ElementKind::LinkCard { url, title } => format!(
            r##"<g><rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="8" fill="#ffffff" stroke="{}"/><text x="{:.1}" y="{:.1}" font-size="14" font-weight="bold" font-family="system-ui, sans-serif" fill="#1f2933">{}</text><text x="{:.1}" y="{:.1}" font-size="11" font-family="system-ui, sans-serif" fill="#52606d">{}</text></g>"##,
            el.x, el.y, el.w, el.h, xml_escape(&el.stroke),
            el.x + 12.0, el.y + 24.0, xml_escape(title),
            el.x + 12.0, el.y + 44.0, xml_escape(url)
        ),
        ElementKind::Frame { title } => format!(
            r##"<g><rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="none" stroke="{}" stroke-dasharray="6 4"/><text x="{:.1}" y="{:.1}" font-size="12" font-family="system-ui, sans-serif" fill="#8a8f98">{}</text></g>"##,
            el.x, el.y, el.w, el.h, xml_escape(&el.stroke), el.x + 6.0, el.y - 6.0, xml_escape(title)
        ),
    }
}

/// Export the document as a standalone SVG file content (world coordinates,
/// viewBox fitted to content).
pub fn to_svg(doc: &Document) -> String {
    const MARGIN: f64 = 24.0;
    let bounds = doc.content_bounds().unwrap_or(Rect::new(0.0, 0.0, 800.0, 600.0));
    let vb_x = bounds.x - MARGIN;
    let vb_y = bounds.y - MARGIN;
    let vb_w = bounds.w.max(1.0) + 2.0 * MARGIN;
    let vb_h = bounds.h.max(1.0) + 2.0 * MARGIN;

    let mut body = String::new();
    let mut ordered: Vec<&Element> = doc.elements.iter().collect();
    ordered.sort_by_key(|e| e.z);
    for el in ordered {
        body.push_str(&element_to_svg(el));
        body.push('\n');
    }

    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="{vb_x:.1} {vb_y:.1} {vb_w:.1} {vb_h:.1}" width="{vb_w:.0}" height="{vb_h:.0}">
<title>{name}</title>
<g>
{body}</g>
</svg>
"##,
        vb_x = vb_x,
        vb_y = vb_y,
        vb_w = vb_w,
        vb_h = vb_h,
        name = xml_escape(&doc.name),
        body = body
    )
}

impl Document {
    pub fn to_svg(&self) -> String {
        to_svg(self)
    }
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

fn sanitize_file_stem(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() || trimmed.chars().all(|c| c == '-') {
        "untitled".to_string()
    } else {
        trimmed.to_string()
    }
}

/// File-backed store rooted at a directory (one `.canvas.json` per doc).
pub struct CanvasStore {
    dir: PathBuf,
}

impl CanvasStore {
    pub fn open(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    fn path_for(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{}.canvas.json", sanitize_file_stem(name)))
    }

    pub fn save(&self, name: &str, doc: &Document) -> Result<()> {
        let path = self.path_for(name);
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(doc)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn load(&self, name: &str) -> Result<Document> {
        let path = self.path_for(name);
        if !path.exists() {
            return Err(CanvasError::NotFound(name.to_string()));
        }
        Ok(serde_json::from_str(&std::fs::read_to_string(&path)?)?)
    }

    pub fn list(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Some(name) = stem.strip_suffix(".canvas") {
                        names.push(name.to_string());
                    }
                }
            }
        }
        names.sort();
        Ok(names)
    }

    pub fn delete(&self, name: &str) -> Result<()> {
        let path = self.path_for(name);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Convenience used by the UI: append a link card to a saved document.
// Made by MrDuck && Ox-Alpha
    pub fn add_link_card(&self, name: &str, url: &str, title: &str) -> Result<Element> {
        let mut doc = self.load(name)?;
        let next_slot = doc.content_bounds().map(|b| b.y + b.h).unwrap_or(0.0) + 40.0;
        let el = Element::new(40.0, next_slot, 240.0, 64.0, ElementKind::LinkCard {
            url: url.to_string(),
            title: title.to_string(),
        });
        doc.add_element(el.clone());
        self.save(name, &doc)?;
        Ok(el)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_json_roundtrip() {
        let mut doc = Document::new("plan");
        doc.add_element(Element::new(10.0, 20.0, 100.0, 50.0, ElementKind::Sticky {
            content: "hello".into(),
            color: "#fff3bf".into(),
        }));
        let json = serde_json::to_string(&doc).unwrap();
        let back: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(back, doc);
    }

    #[test]
    fn hit_test_picks_topmost_and_respects_shapes() {
        let mut doc = Document::new("t");
        let bottom = doc.add_element(Element::new(0.0, 0.0, 100.0, 100.0, ElementKind::Rectangle {
            fill: "#dbe4ff".into(),
        })).clone();
        let top = doc.add_element(Element::new(40.0, 40.0, 60.0, 60.0, ElementKind::Rectangle {
            fill: "#ffe3e3".into(),
        })).clone();

        assert_eq!(doc.hit_test(Point::new(50.0, 50.0)), Some(&top));
        assert_eq!(doc.hit_test(Point::new(10.0, 10.0)), Some(&bottom));
        assert_eq!(doc.hit_test(Point::new(-5.0, -5.0)), None);

        // Ellipse corners are empty space.
        let ell = Element::new(200.0, 200.0, 100.0, 100.0, ElementKind::Ellipse { fill: "#fff".into() });
        assert!(ell.hit_test(Point::new(250.0, 250.0)));
        assert!(!ell.hit_test(Point::new(205.0, 205.0)));
    }

    #[test]
    fn line_hit_uses_distance_not_bbox() {
        let line = Element::new(0.0, 0.0, 0.0, 0.0, ElementKind::Line {
            points: vec![Point::new(0.0, 0.0), Point::new(100.0, 0.0)],
            stroke_width: 2.0,
        });
        assert!(line.hit_test(Point::new(50.0, 3.0)));
        assert!(!line.hit_test(Point::new(50.0, 30.0)));
    }

    #[test]
    fn undo_redo_roundtrip() {
        let mut doc = Document::new("u");
        let mut undo = UndoStack::new();

        let el = Element::new(0.0, 0.0, 10.0, 10.0, ElementKind::Text {
            content: "v1".into(),
            font_size: 14.0,
        });
        let id = el.id;
        assert!(undo.commit(&mut doc, Op::Add(el)));
        assert_eq!(doc.elements.len(), 1);

        assert!(undo.undo(&mut doc));
        assert_eq!(doc.elements.len(), 0);
        assert!(undo.redo(&mut doc));
        assert_eq!(doc.elements.len(), 1);

        assert!(undo.commit(&mut doc, Op::SetText { id, from: "v1".into(), to: "v2".into() }));
        match &doc.elements[0].kind {
            ElementKind::Text { content, .. } => assert_eq!(content, "v2"),
            other => panic!("unexpected kind: {:?}", other),
        }
        assert!(undo.undo(&mut doc));
        match &doc.elements[0].kind {
            ElementKind::Text { content, .. } => assert_eq!(content, "v1"),
            other => panic!("unexpected kind: {:?}", other),
        }
        assert!(undo.redo(&mut doc));
        match &doc.elements[0].kind {
            ElementKind::Text { content, .. } => assert_eq!(content, "v2"),
            other => panic!("unexpected kind: {:?}", other),
        }

        assert!(undo.commit(&mut doc, Op::Move { id, from: (0.0, 0.0), to: (7.0, 9.0) }));
        assert_eq!((doc.elements[0].x, doc.elements[0].y), (7.0, 9.0));
        undo.undo(&mut doc);
        assert_eq!((doc.elements[0].x, doc.elements[0].y), (0.0, 0.0));
    }

    #[test]
    fn removing_element_drops_connectors() {
        let mut doc = Document::new("c");
        let a = doc.add_element(Element::new(0.0, 0.0, 10.0, 10.0, ElementKind::Rectangle { fill: "#fff".into() })).clone();
        let b = doc.add_element(Element::new(50.0, 0.0, 10.0, 10.0, ElementKind::Rectangle { fill: "#fff".into() })).clone();
        doc.connect(a.id, b.id);
        assert_eq!(doc.connectors.len(), 1);
        doc.remove_element(a.id);
        assert!(doc.connectors.is_empty());
    }

    #[test]
    fn svg_export_is_self_contained_and_escapes() {
        let mut doc = Document::new("Export <Test>");
        doc.add_element(Element::new(0.0, 0.0, 120.0, 40.0, ElementKind::Text {
            content: "a<b> & c".into(),
            font_size: 14.0,
        }));
        let sticky = doc
            .add_element(Element::new(150.0, 0.0, 120.0, 80.0, ElementKind::Sticky {
                content: "note".into(),
                color: "#fff3bf".into(),
            }))
            .clone();
        let card = doc
            .add_element(Element::new(300.0, 0.0, 240.0, 64.0, ElementKind::LinkCard {
                url: "https://example.com/a?x=1&y=2".into(),
                title: "Article".into(),
            }))
            .clone();
        doc.add_element(Element::new(0.0, 120.0, 0.0, 0.0, ElementKind::Arrow {
            points: vec![Point::new(0.0, 120.0), Point::new(80.0, 160.0)],
            stroke_width: 2.0,
        }));
        doc.connect(sticky.id, card.id);

        let svg = doc.to_svg();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("&lt;b&gt; &amp; c"));
        assert!(svg.contains(r##"fill="#fff3bf""##)); // sticky background survives
        assert!(svg.contains("<polyline"));
        assert!(svg.contains("<polygon")); // arrow head
        assert!(svg.contains("https://example.com/a?x=1&amp;y=2"));
    }

    #[test]
    fn viewport_transforms_invert() {
        let vp = Viewport { pan_x: 10.0, pan_y: -5.0, zoom: 1.5 };
        let w = Point::new(3.0, 4.0);
        let s = vp.world_to_screen(w);
        let back = vp.screen_to_world(s);
        assert!((back.x - w.x).abs() < 1e-9);
        assert!((back.y - w.y).abs() < 1e-9);
    }

    #[test]
    fn store_save_load_list_delete_and_link_card() {
        let dir = std::env::temp_dir().join(format!("apb-canvas-{}", Uuid::new_v4()));
        let store = CanvasStore::open(&dir).unwrap();

        let mut doc = Document::new("Mind map");
        doc.add_element(Element::new(0.0, 0.0, 100.0, 40.0, ElementKind::Text {
            content: "root".into(),
            font_size: 16.0,
        }));
        store.save("Mind map", &doc).unwrap();

        assert_eq!(store.list().unwrap(), vec!["Mind map".to_string()]);
        let loaded = store.load("Mind map").unwrap();
        assert_eq!(loaded, doc);
        assert!(loaded.to_svg().contains("<svg"));

        let card = store.add_link_card("Mind map", "https://example.com/article", "Article").unwrap();
        let updated = store.load("Mind map").unwrap();
        assert_eq!(updated.elements.len(), 2);
        match updated.elements.iter().find(|e| e.id == card.id).unwrap().kind {
            ElementKind::LinkCard { ref url, .. } => assert_eq!(url, "https://example.com/article"),
            ref other => panic!("unexpected kind: {:?}", other),
        }

        store.delete("Mind map").unwrap();
        assert!(store.load("Mind map").is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn filename_sanitization_is_portable() {
        assert_eq!(sanitize_file_stem("my plan: v2?"), "my plan- v2-");
        assert_eq!(sanitize_file_stem("///"), "untitled");
        assert_eq!(sanitize_file_stem(""), "untitled");
    }
}

// Made by MrDuck && Ox-Alpha