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

/// A rectangle with optional content.
#[derive(Debug, Clone)]
pub struct Rect {
    pub col: usize,
    pub row: usize,
    /// Inner width (excluding borders).
    pub width: usize,
    /// Inner height (excluding borders).
    pub height: usize,
    pub content: Option<String>,
    pub style: BorderStyle,
    pub content_overflow: ContentOverflow,
    pub content_align: ContentAlign,
}

impl Rect {
    pub fn new(col: usize, row: usize, width: usize, height: usize) -> Self {
        Self {
            col,
            row,
            width,
            height,
            content: None,
            style: BorderStyle::default(),
            content_overflow: ContentOverflow::default(),
            content_align: ContentAlign::default(),
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
}
