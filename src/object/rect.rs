/// Border style for rectangles.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BorderStyle {
    #[default]
    Light,
    Heavy,
    Double,
    Rounded,
}

/// Content overflow mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ContentOverflow {
    #[default]
    Ellipsis,
    Overflow,
    Hidden,
    Error,
}

/// Content alignment within rect.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ContentAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Side of a rect for anchor-based arrows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

/// A rectangle with optional content.
#[derive(Debug, Clone)]
pub struct Rect {
    pub col: usize,
    pub row: usize,
    /// Inner width (excluding borders).
    pub width: usize,
    /// Inner height (excluding borders).
    pub height: usize,
    pub id: Option<String>,
    pub content: Option<String>,
    pub style: BorderStyle,
    pub content_overflow: ContentOverflow,
    pub content_align: ContentAlign,
    pub legend: Option<super::Legend>,
}

impl Rect {
    pub fn new(col: usize, row: usize, width: usize, height: usize) -> Self {
        Self {
            col,
            row,
            width,
            height,
            id: None,
            content: None,
            style: BorderStyle::default(),
            content_overflow: ContentOverflow::default(),
            content_align: ContentAlign::default(),
            legend: None,
        }
    }

    /// Total width including borders.
    pub fn outer_width(&self) -> usize {
        self.width + 2
    }

    /// Total height including borders.
    pub fn outer_height(&self) -> usize {
        self.height + 2
    }

    /// Center column (display-column of the inner center).
    fn center_col(&self) -> usize {
        self.col + 1 + self.width / 2
    }

    /// Center row (row of the inner center).
    fn center_row(&self) -> usize {
        self.row + 1 + self.height / 2
    }

    /// Source anchor: 1 cell OUTSIDE the border (arrow starts here).
    pub fn src_anchor(&self, side: Side) -> (usize, usize) {
        match side {
            Side::Top => (self.center_col(), self.row.saturating_sub(1)),
            Side::Right => (self.col + self.width + 2, self.center_row()),
            Side::Bottom => (self.center_col(), self.row + self.height + 2),
            Side::Left => (self.col.saturating_sub(1), self.center_row()),
        }
    }

    /// Dest anchor: ON the border (arrowhead replaces border character).
    pub fn dst_anchor(&self, side: Side) -> (usize, usize) {
        match side {
            Side::Top => (self.center_col(), self.row),
            Side::Right => (self.col + self.width + 1, self.center_row()),
            Side::Bottom => (self.center_col(), self.row + self.height + 1),
            Side::Left => (self.col, self.center_row()),
        }
    }
}
