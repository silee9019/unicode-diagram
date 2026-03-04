pub mod arrow;
pub mod line;
pub mod rect;
pub mod text;

pub use arrow::{Arrow, Dir, ResolvedArrow};
pub use line::{HLine, LineStyle, VLine};
pub use rect::{BorderStyle, ContentAlign, ContentOverflow, Rect, Side};
pub use text::Text;

use crate::width;

/// Legend position (absolute direction relative to the object).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LegendPos {
    Top,
    Bottom,
    Left,
    Right,
    #[default]
    Auto,
}

/// Legend (external label) for rect, hline, vline, arrow.
#[derive(Debug, Clone)]
pub struct Legend {
    pub text: String,
    pub pos: LegendPos,
    pub overflow: ContentOverflow,
    pub align: ContentAlign,
}

/// Unified enum for all drawable objects.
#[derive(Debug, Clone)]
pub enum DrawObject {
    Rect(Rect),
    Text(Text),
    HLine(HLine),
    VLine(VLine),
    Arrow(ResolvedArrow),
}

impl DrawObject {
    /// Returns (max_col_exclusive, max_row_exclusive) — the minimum canvas size to contain this object.
    pub fn bounds(&self) -> (usize, usize) {
        match self {
            DrawObject::Rect(r) => (r.col + r.outer_width(), r.row + r.outer_height()),
            DrawObject::Text(t) => {
                let max_w = t.content.lines().map(width::str_width).max().unwrap_or(0);
                let line_count = t.content.lines().count().max(1);
                (t.col + max_w, t.row + line_count)
            }
            DrawObject::HLine(h) => (h.col + h.length, h.row + 1),
            DrawObject::VLine(v) => (v.col + 1, v.row + v.length),
            DrawObject::Arrow(a) => {
                let (mut max_col, mut max_row) = (0, 0);
                for &(c, r) in &a.waypoints {
                    max_col = max_col.max(c);
                    max_row = max_row.max(r);
                }
                (max_col + 1, max_row + 1)
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
            DrawObject::Arrow(a) => {
                let (mut min_col, mut min_row) = (usize::MAX, usize::MAX);
                for &(c, r) in &a.waypoints {
                    min_col = min_col.min(c);
                    min_row = min_row.min(r);
                }
                (min_col, min_row)
            }
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
                let pts: Vec<String> = a.waypoints.iter().map(|(c, r)| format!("({c},{r})")).collect();
                format!("arrow {}", pts.join("->"))
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
            DrawObject::Arrow(a) => {
                let pts: Vec<String> = a.waypoints.iter().map(|(c, r)| format!("({c},{r})")).collect();
                format!("arrow {}", pts.join(" -> "))
            }
        }
    }
}
