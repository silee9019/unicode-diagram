use thiserror::Error;

#[derive(Debug, Error)]
pub enum UnidError {
    #[error("canvas not defined")]
    NoCanvas,

    #[error("collision not defined")]
    NoCollision,

    #[error("position ({col}, {row}) is out of canvas bounds ({canvas_width}x{canvas_height})")]
    OutOfBounds {
        col: usize,
        row: usize,
        canvas_width: usize,
        canvas_height: usize,
    },

    #[error("collision: object #{incoming_idx} ({incoming_desc}) overlaps object #{existing_idx} ({existing_desc}) at ({overlap_col},{overlap_row})-({overlap_end_col},{overlap_end_row}) size {overlap_w}x{overlap_h}")]
    Collision {
        incoming_idx: usize,
        incoming_desc: String,
        existing_idx: usize,
        existing_desc: String,
        overlap_col: usize,
        overlap_row: usize,
        overlap_end_col: usize,
        overlap_end_row: usize,
        overlap_w: usize,
        overlap_h: usize,
    },

    #[error("content overflow: '{label}' ({label_width} cols) exceeds rect inner width ({inner_width} cols)")]
    LabelOverflow {
        label: String,
        label_width: usize,
        inner_width: usize,
    },

    #[error("parse error at line {line}: {message}")]
    Parse { line: usize, message: String },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
