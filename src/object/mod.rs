pub mod arrow;
pub mod line;
pub mod rect;
pub mod text;

pub use arrow::Arrow;
pub use line::{HLine, LineStyle, VLine};
pub use rect::{BorderStyle, ContentAlign, ContentOverflow, Rect};
pub use text::Text;

use crate::width;

/// Unified enum for all drawable objects.
#[derive(Debug, Clone)]
pub enum DrawObject {
    Rect(Rect),
    Text(Text),
    HLine(HLine),
    VLine(VLine),
    Arrow(Arrow),
}

impl DrawObject {
    /// Returns (max_col_exclusive, max_row_exclusive) — the minimum canvas size to contain this object.
    pub fn bounds(&self) -> (usize, usize) {
        match self {
            DrawObject::Rect(r) => (r.col + r.outer_width(), r.row + r.outer_height()),
            DrawObject::Text(t) => (t.col + width::str_width(&t.content), t.row + 1),
            DrawObject::HLine(h) => (h.col + h.length, h.row + 1),
            DrawObject::VLine(v) => (v.col + 1, v.row + v.length),
            DrawObject::Arrow(a) => {
                let max_col = a.from_col.max(a.to_col) + 1;
                let max_row = a.from_row.max(a.to_row) + 1;
                (max_col, max_row)
            }
        }
    }

    /// Returns (col, row) for sorting: top-left position of the object.
    pub fn position(&self) -> (usize, usize) {
        match self {
            DrawObject::Rect(r) => (r.col, r.row),
            DrawObject::Text(t) => (t.col, t.row),
            DrawObject::HLine(h) => (h.col, h.row),
            DrawObject::VLine(v) => (v.col, v.row),
            DrawObject::Arrow(a) => (a.from_col.min(a.to_col), a.from_row.min(a.to_row)),
        }
    }

    /// Returns a display name for the object type.
    pub fn type_name(&self) -> &'static str {
        match self {
            DrawObject::Rect(_) => "rect",
            DrawObject::Text(_) => "text",
            DrawObject::HLine(_) => "hline",
            DrawObject::VLine(_) => "vline",
            DrawObject::Arrow(_) => "arrow",
        }
    }

    /// Short description for collision error messages.
    pub fn collision_desc(&self) -> String {
        match self {
            DrawObject::Rect(r) => {
                format!("rect at ({},{}) {}x{}", r.col, r.row, r.outer_width(), r.outer_height())
            }
            DrawObject::Text(t) => {
                format!("text at ({},{}) w={}", t.col, t.row, width::str_width(&t.content))
            }
            DrawObject::HLine(h) => format!("hline at ({},{}) len={}", h.col, h.row, h.length),
            DrawObject::VLine(v) => format!("vline at ({},{}) len={}", v.col, v.row, v.length),
            DrawObject::Arrow(a) => {
                format!("arrow ({},{})->({},{})", a.from_col, a.from_row, a.to_col, a.to_row)
            }
        }
    }

    /// Returns a summary string for list display.
    pub fn summary(&self) -> String {
        match self {
            DrawObject::Rect(r) => {
                let content = r.content.as_deref().unwrap_or("");
                format!(
                    "rect ({},{}) {}x{} {:?}{}",
                    r.col,
                    r.row,
                    r.width,
                    r.height,
                    r.style,
                    if content.is_empty() {
                        String::new()
                    } else {
                        format!(" \"{}\"", content)
                    }
                )
            }
            DrawObject::Text(t) => format!("text ({},{}) \"{}\"", t.col, t.row, t.content),
            DrawObject::HLine(h) => {
                format!("hline ({},{}) len={} {:?}", h.col, h.row, h.length, h.style)
            }
            DrawObject::VLine(v) => {
                format!("vline ({},{}) len={} {:?}", v.col, v.row, v.length, v.style)
            }
            DrawObject::Arrow(a) => format!(
                "arrow ({},{}) -> ({},{})",
                a.from_col, a.from_row, a.to_col, a.to_row
            ),
        }
    }
}
