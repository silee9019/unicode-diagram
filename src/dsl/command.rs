use crate::object::rect::{BorderStyle, ContentAlign, ContentOverflow, Side};
use crate::object::DrawObject;

/// Represents a parsed DSL command.
#[derive(Debug)]
pub enum DslCommand {
    /// Canvas size declaration with optional border and global defaults.
    Canvas {
        width: CanvasSize,
        height: CanvasSize,
        border: Option<BorderStyle>,
        content_overflow: Option<ContentOverflow>,
        content_align: Option<ContentAlign>,
    },
    /// Collision mode declaration.
    Collision(bool),
    /// A drawable object (rect, text, hline, vline — NOT arrow).
    Object(DrawObject),
    /// An unresolved arrow referencing object IDs (resolved later).
    Arrow {
        src_id: String,
        src_side: Side,
        dst_id: String,
        dst_side: Side,
        line: usize,
    },
}

/// Canvas dimension: explicit size or auto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasSize {
    Fixed(usize),
    Auto,
}
