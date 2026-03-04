use crate::canvas::cell::Cell;
use crate::error::UnidError;
use crate::width;

/// A 2D grid of cells addressed by (display-column, row).
pub struct Canvas {
    width: usize,
    height: usize,
    grid: Vec<Vec<Cell>>,
    /// Tracks which object index owns each cell (for collision reporting).
    owner: Vec<Vec<Option<usize>>>,
}

impl Canvas {
    /// Creates a new canvas filled with spaces.
    pub fn new(width: usize, height: usize) -> Self {
        let grid = (0..height)
            .map(|_| (0..width).map(|_| Cell::space()).collect())
            .collect();
        let owner = vec![vec![None; width]; height];
        Self {
            width,
            height,
            grid,
            owner,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// Returns the owner object index at (col, row), if any.
    pub fn owner_at(&self, col: usize, row: usize) -> Option<usize> {
        self.owner.get(row).and_then(|r| r.get(col)).copied().flatten()
    }

    /// Places a single character at (col, row) owned by `object_idx`.
    /// For wide characters (CJK), also places a continuation cell at col+1.
    pub fn put_char(
        &mut self,
        col: usize,
        row: usize,
        ch: char,
        collision: bool,
        object_idx: usize,
    ) -> Result<(), UnidError> {
        let w = width::char_width(ch);

        if col + w > self.width || row >= self.height {
            return Err(UnidError::OutOfBounds {
                col,
                row,
                canvas_width: self.width,
                canvas_height: self.height,
            });
        }

        if collision {
            let existing = &self.grid[row][col];
            if existing.ch != ' ' && !existing.is_continuation {
                return Err(self.collision_error(col, row, object_idx));
            }
            if w == 2 {
                let next = &self.grid[row][col + 1];
                if next.ch != ' ' && !next.is_continuation {
                    return Err(self.collision_error(col + 1, row, object_idx));
                }
            }
        }

        self.grid[row][col] = Cell::new(ch);
        self.owner[row][col] = Some(object_idx);
        if w == 2 {
            self.grid[row][col + 1] = Cell::continuation();
            self.owner[row][col + 1] = Some(object_idx);
        }
        Ok(())
    }

    /// Places a string starting at (col, row), advancing by each character's display width.
    pub fn put_str(
        &mut self,
        col: usize,
        row: usize,
        s: &str,
        collision: bool,
        object_idx: usize,
    ) -> Result<(), UnidError> {
        let mut current_col = col;
        for ch in s.chars() {
            self.put_char(current_col, row, ch, collision, object_idx)?;
            current_col += width::char_width(ch);
        }
        Ok(())
    }

    /// Renders the canvas to a string.
    pub fn render(&self) -> String {
        let mut lines = Vec::with_capacity(self.height);
        for row in &self.grid {
            let mut line = String::new();
            for cell in row {
                if cell.is_continuation {
                    continue;
                }
                line.push(cell.ch);
            }
            lines.push(line.trim_end().to_string());
        }
        // Remove trailing empty lines
        while lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }
        lines.join("\n")
    }

    /// Creates a partial collision error with cell-level info.
    /// The renderer will enrich this with object descriptions and overlap region.
    fn collision_error(&self, col: usize, row: usize, incoming_idx: usize) -> UnidError {
        let existing_idx = self.owner[row][col].unwrap_or(0);
        UnidError::Collision {
            incoming_idx,
            incoming_desc: String::new(),
            existing_idx,
            existing_desc: String::new(),
            overlap_col: col,
            overlap_row: row,
            overlap_end_col: col,
            overlap_end_row: row,
            overlap_w: 1,
            overlap_h: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn empty_canvas() {
        let canvas = Canvas::new(5, 3);
        assert_eq!(canvas.render(), "");
    }

    #[test]
    fn put_ascii_char() {
        let mut canvas = Canvas::new(5, 3);
        canvas.put_char(0, 0, 'A', false, 0).unwrap();
        assert_eq!(canvas.render(), "A");
    }

    #[test]
    fn put_cjk_char() {
        let mut canvas = Canvas::new(5, 3);
        canvas.put_char(0, 0, '한', false, 0).unwrap();
        assert_eq!(canvas.render(), "한");
    }

    #[test]
    fn put_str_ascii() {
        let mut canvas = Canvas::new(10, 1);
        canvas.put_str(0, 0, "Hello", false, 0).unwrap();
        assert_eq!(canvas.render(), "Hello");
    }

    #[test]
    fn put_str_cjk() {
        let mut canvas = Canvas::new(10, 1);
        canvas.put_str(0, 0, "한글", false, 0).unwrap();
        assert_eq!(canvas.render(), "한글");
    }

    #[test]
    fn put_str_mixed() {
        let mut canvas = Canvas::new(10, 1);
        canvas.put_str(0, 0, "A한B", false, 0).unwrap();
        assert_eq!(canvas.render(), "A한B");
    }

    #[test]
    fn cjk_continuation_cell() {
        let mut canvas = Canvas::new(4, 1);
        canvas.put_char(0, 0, '한', false, 0).unwrap();
        assert!(canvas.grid[0][1].is_continuation);
        canvas.put_char(2, 0, 'A', false, 0).unwrap();
        assert_eq!(canvas.render(), "한A");
    }

    #[test]
    fn out_of_bounds() {
        let mut canvas = Canvas::new(3, 1);
        let result = canvas.put_char(3, 0, 'A', false, 0);
        assert!(result.is_err());
    }

    #[test]
    fn cjk_out_of_bounds() {
        let mut canvas = Canvas::new(3, 1);
        let result = canvas.put_char(2, 0, '한', false, 0);
        assert!(result.is_err());
    }

    #[test]
    fn collision_detected() {
        let mut canvas = Canvas::new(5, 1);
        canvas.put_char(0, 0, 'A', true, 0).unwrap();
        let result = canvas.put_char(0, 0, 'B', true, 1);
        assert!(result.is_err());
    }

    #[test]
    fn collision_off_overwrites() {
        let mut canvas = Canvas::new(5, 1);
        canvas.put_char(0, 0, 'A', false, 0).unwrap();
        canvas.put_char(0, 0, 'B', false, 1).unwrap();
        assert_eq!(canvas.render(), "B");
    }

    #[test]
    fn multiline_render() {
        let mut canvas = Canvas::new(5, 3);
        canvas.put_str(0, 0, "Hello", false, 0).unwrap();
        canvas.put_str(0, 2, "World", false, 1).unwrap();
        assert_eq!(canvas.render(), "Hello\n\nWorld");
    }

    #[test]
    fn owner_tracking() {
        let mut canvas = Canvas::new(5, 1);
        canvas.put_char(0, 0, 'A', false, 42).unwrap();
        assert_eq!(canvas.owner_at(0, 0), Some(42));
        assert_eq!(canvas.owner_at(1, 0), None);
    }
}
