use crate::canvas::Canvas;
use crate::error::UnidError;
use crate::object::arrow::{corner_char, default_arrowhead, resolve_arrowhead, segment_dir, Dir, ResolvedArrow};
use crate::object::{
    BorderStyle, ContentAlign, ContentOverflow, DrawObject, HLine, Legend, LegendPos, LineStyle,
    Rect, Text, VLine,
};
use crate::width;

/// Renders DrawObjects onto a Canvas.
pub struct Renderer {
    pub canvas: Canvas,
    pub collision: bool,
    /// Objects tracked for collision reporting.
    objects: Vec<DrawObject>,
    /// Global content overflow default from canvas settings.
    pub global_overflow: ContentOverflow,
    /// Global content align default from canvas settings.
    pub global_align: ContentAlign,
}

impl Renderer {
    pub fn new(canvas: Canvas, collision: bool) -> Self {
        Self {
            canvas,
            collision,
            objects: Vec::new(),
            global_overflow: ContentOverflow::default(),
            global_align: ContentAlign::default(),
        }
    }

    /// Draws a single object onto the canvas (single-pass, used for lint/collision).
    pub fn draw(&mut self, object: &DrawObject) -> Result<(), UnidError> {
        let idx = self.objects.len();
        self.objects.push(object.clone());
        let result = match object {
            DrawObject::Rect(r) => self.draw_rect(r, idx),
            DrawObject::Text(t) => self.draw_text(t, idx),
            DrawObject::HLine(h) => self.draw_hline(h, idx),
            DrawObject::VLine(v) => self.draw_vline(v, idx),
            DrawObject::Arrow(a) => self.draw_arrow(a, idx),
        };
        // Enrich collision errors with object descriptions and overlap region
        result.map_err(|e| self.enrich_error(e, object, idx))
    }

    /// Draws all objects with 2-pass rendering:
    /// Pass 1: Structure (borders, lines, arrow lines/corners)
    /// Pass 2: Text content (c=, lg=, text objects) — overwrites structure
    pub fn draw_all(&mut self, objects: &[DrawObject]) -> Result<(), UnidError> {
        // Register all objects first for collision tracking
        for obj in objects {
            self.objects.push(obj.clone());
        }

        // Pass 1: Draw structural elements
        for (idx, obj) in objects.iter().enumerate() {
            let result = match obj {
                DrawObject::Rect(r) => self.draw_rect_structure(r, idx),
                DrawObject::HLine(h) => self.draw_hline_structure(h, idx),
                DrawObject::VLine(v) => self.draw_vline_structure(v, idx),
                DrawObject::Arrow(a) => self.draw_arrow_structure(a, idx),
                DrawObject::Text(_) => Ok(()), // Text is content-only
            };
            result.map_err(|e| self.enrich_error(e, obj, idx))?;
        }

        // Pass 2: Draw text content (overwrites structure, no collision check)
        for (idx, obj) in objects.iter().enumerate() {
            let result = match obj {
                DrawObject::Rect(r) => self.draw_rect_content(r, idx),
                DrawObject::Text(t) => self.draw_text(t, idx),
                DrawObject::HLine(h) => self.draw_hline_content(h, idx),
                DrawObject::VLine(v) => self.draw_vline_content(v, idx),
                DrawObject::Arrow(a) => self.draw_arrow_content(a, idx),
            };
            result.map_err(|e| self.enrich_error(e, obj, idx))?;
        }

        Ok(())
    }

    /// Renders the canvas to string output.
    pub fn render(&self) -> String {
        self.canvas.render()
    }

    /// Draws canvas border if specified.
    pub fn draw_border(&mut self, style: BorderStyle) -> Result<(), UnidError> {
        let (tl, tr, bl, br, h, v) = border_chars(style);
        let w = self.canvas.width();
        let ht = self.canvas.height();
        let idx = usize::MAX; // Special index for border

        // Top
        self.canvas.put_char(0, 0, tl, false, idx)?;
        for c in 1..w - 1 {
            self.canvas.put_char(c, 0, h, false, idx)?;
        }
        self.canvas.put_char(w - 1, 0, tr, false, idx)?;

        // Sides
        for r in 1..ht - 1 {
            self.canvas.put_char(0, r, v, false, idx)?;
            self.canvas.put_char(w - 1, r, v, false, idx)?;
        }

        // Bottom
        self.canvas.put_char(0, ht - 1, bl, false, idx)?;
        for c in 1..w - 1 {
            self.canvas.put_char(c, ht - 1, h, false, idx)?;
        }
        self.canvas.put_char(w - 1, ht - 1, br, false, idx)?;

        Ok(())
    }

    fn enrich_error(&self, err: UnidError, _object: &DrawObject, idx: usize) -> UnidError {
        match err {
            UnidError::Collision {
                existing_idx,
                overlap_col,
                overlap_row,
                ..
            } => {
                let incoming_desc = if idx < self.objects.len() {
                    self.objects[idx].collision_desc()
                } else {
                    "unknown".to_string()
                };
                let existing_desc = if existing_idx < self.objects.len() {
                    self.objects[existing_idx].collision_desc()
                } else {
                    "border".to_string()
                };

                // Calculate overlap region between two objects
                let (oc, or, oec, oer, ow, oh) = if idx < self.objects.len()
                    && existing_idx < self.objects.len()
                {
                    self.compute_overlap(idx, existing_idx)
                } else {
                    (overlap_col, overlap_row, overlap_col, overlap_row, 1, 1)
                };

                UnidError::Collision {
                    incoming_idx: idx + 1,
                    incoming_desc,
                    existing_idx: existing_idx + 1,
                    existing_desc,
                    overlap_col: oc,
                    overlap_row: or,
                    overlap_end_col: oec,
                    overlap_end_row: oer,
                    overlap_w: ow,
                    overlap_h: oh,
                }
            }
            other => other,
        }
    }

    fn compute_overlap(
        &self,
        idx_a: usize,
        idx_b: usize,
    ) -> (usize, usize, usize, usize, usize, usize) {
        let (a_pos, a_bounds) = (self.objects[idx_a].position(), self.objects[idx_a].bounds());
        let (b_pos, b_bounds) = (self.objects[idx_b].position(), self.objects[idx_b].bounds());

        let start_col = a_pos.0.max(b_pos.0);
        let start_row = a_pos.1.max(b_pos.1);
        let end_col = a_bounds.0.min(b_bounds.0);
        let end_row = a_bounds.1.min(b_bounds.1);

        if end_col > start_col && end_row > start_row {
            (
                start_col,
                start_row,
                end_col - 1,
                end_row - 1,
                end_col - start_col,
                end_row - start_row,
            )
        } else {
            (start_col, start_row, start_col, start_row, 1, 1)
        }
    }

    fn draw_rect(&mut self, rect: &Rect, idx: usize) -> Result<(), UnidError> {
        self.draw_rect_structure(rect, idx)?;
        self.draw_rect_content(rect, idx)
    }

    /// Draws rect structural elements (borders only).
    fn draw_rect_structure(&mut self, rect: &Rect, idx: usize) -> Result<(), UnidError> {
        let (tl, tr, bl, br, h, v) = border_chars(rect.style);
        let col = rect.col;
        let row = rect.row;
        let inner_w = rect.width;
        let inner_h = rect.height;

        // Top border
        self.canvas.put_char(col, row, tl, self.collision, idx)?;
        for c in 1..=inner_w {
            self.canvas
                .put_char(col + c, row, h, self.collision, idx)?;
        }
        self.canvas
            .put_char(col + inner_w + 1, row, tr, self.collision, idx)?;

        // Side borders
        for r in 1..=inner_h {
            self.canvas
                .put_char(col, row + r, v, self.collision, idx)?;
            self.canvas
                .put_char(col + inner_w + 1, row + r, v, self.collision, idx)?;
        }

        // Bottom border
        self.canvas
            .put_char(col, row + inner_h + 1, bl, self.collision, idx)?;
        for c in 1..=inner_w {
            self.canvas
                .put_char(col + c, row + inner_h + 1, h, self.collision, idx)?;
        }
        self.canvas
            .put_char(col + inner_w + 1, row + inner_h + 1, br, self.collision, idx)?;

        Ok(())
    }

    /// Draws rect text content and legend (Pass 2).
    fn draw_rect_content(&mut self, rect: &Rect, idx: usize) -> Result<(), UnidError> {
        let col = rect.col;
        let row = rect.row;
        let inner_w = rect.width;
        let inner_h = rect.height;

        if let Some(content) = &rect.content {
            let lines: Vec<&str> = content.lines().collect();
            let line_count = lines.len();
            let overflow = rect.content_overflow;
            let align = rect.content_align;

            let start_row = if line_count <= inner_h {
                row + 1 + (inner_h - line_count) / 2
            } else {
                row + 1
            };

            for (i, line) in lines.iter().enumerate() {
                let r = start_row + i;
                if r > row + inner_h {
                    break;
                }
                self.render_content_line(col, r, line, inner_w, overflow, align, idx)?;
            }
        }

        if let Some(legend) = &rect.legend {
            let effective_pos = match legend.pos {
                LegendPos::Auto | LegendPos::Top => LegendPos::Top,
                LegendPos::Bottom => LegendPos::Bottom,
                _ => LegendPos::Top,
            };
            let lg_col = col;
            let lg_row = match effective_pos {
                LegendPos::Top => row.saturating_sub(1),
                LegendPos::Bottom => row + inner_h + 2,
                _ => unreachable!(),
            };
            self.draw_legend_text(lg_col, lg_row, legend, idx)?;
        }

        Ok(())
    }

    /// Renders a single line of content into the canvas at (col+1..col+inner_w, row).
    /// Handles overflow (Ellipsis/Hidden/Error) and alignment (Left/Center/Right).
    #[allow(clippy::too_many_arguments)]
    fn render_content_line(
        &mut self,
        col: usize,
        row: usize,
        line: &str,
        inner_w: usize,
        overflow: ContentOverflow,
        align: ContentAlign,
        idx: usize,
    ) -> Result<(), UnidError> {
        let content_w = width::str_width(line);

        let display = if content_w <= inner_w {
            line.to_string()
        } else {
            match overflow {
                ContentOverflow::Ellipsis => {
                    ellipsis_truncate(line, content_w, inner_w, align)
                }
                ContentOverflow::Overflow => line.to_string(),
                ContentOverflow::Hidden => {
                    hidden_truncate(line, inner_w, align)
                }
                ContentOverflow::Error => {
                    return Err(UnidError::LabelOverflow {
                        label: line.to_string(),
                        label_width: content_w,
                        inner_width: inner_w,
                    });
                }
            }
        };

        let display_w = width::str_width(&display);
        let pad_left = match align {
            ContentAlign::Left => 0,
            ContentAlign::Center => (inner_w.saturating_sub(display_w)) / 2,
            ContentAlign::Right => inner_w.saturating_sub(display_w),
        };

        self.canvas.put_str(
            col + 1 + pad_left,
            row,
            &display,
            false,
            idx,
        )
    }

    /// Draws legend text at the given position (multiline supported).
    /// Legend text is rendered without collision check (text always wins over structure).
    fn draw_legend_text(
        &mut self,
        col: usize,
        row: usize,
        legend: &Legend,
        idx: usize,
    ) -> Result<(), UnidError> {
        for (i, line) in legend.text.lines().enumerate() {
            self.canvas.put_str(col, row + i, line, false, idx)?;
        }
        Ok(())
    }

    fn draw_text(&mut self, text: &Text, idx: usize) -> Result<(), UnidError> {
        for (i, line) in text.content.lines().enumerate() {
            self.canvas
                .put_str(text.col, text.row + i, line, self.collision, idx)?;
        }
        Ok(())
    }

    fn draw_hline(&mut self, hline: &HLine, idx: usize) -> Result<(), UnidError> {
        self.draw_hline_structure(hline, idx)?;
        self.draw_hline_content(hline, idx)
    }

    fn draw_hline_structure(&mut self, hline: &HLine, idx: usize) -> Result<(), UnidError> {
        let ch = hline_char(hline.style);
        for c in 0..hline.length {
            self.canvas
                .put_char(hline.col + c, hline.row, ch, self.collision, idx)?;
        }
        Ok(())
    }

    fn draw_hline_content(&mut self, hline: &HLine, idx: usize) -> Result<(), UnidError> {
        if let Some(legend) = &hline.legend {
            let effective_pos = match legend.pos {
                LegendPos::Auto | LegendPos::Top => LegendPos::Top,
                other => other,
            };
            let (lg_col, lg_row) = match effective_pos {
                LegendPos::Top => (hline.col, hline.row.saturating_sub(1)),
                LegendPos::Bottom => (hline.col, hline.row + 1),
                LegendPos::Left => {
                    let tw = width::str_width(&legend.text);
                    (hline.col.saturating_sub(tw + 1), hline.row)
                }
                LegendPos::Right => (hline.col + hline.length + 1, hline.row),
                LegendPos::Auto => unreachable!(),
            };
            self.draw_legend_text(lg_col, lg_row, legend, idx)?;
        }
        Ok(())
    }

    fn draw_vline(&mut self, vline: &VLine, idx: usize) -> Result<(), UnidError> {
        self.draw_vline_structure(vline, idx)?;
        self.draw_vline_content(vline, idx)
    }

    fn draw_vline_structure(&mut self, vline: &VLine, idx: usize) -> Result<(), UnidError> {
        let ch = vline_char(vline.style);
        for r in 0..vline.length {
            self.canvas
                .put_char(vline.col, vline.row + r, ch, self.collision, idx)?;
        }
        Ok(())
    }

    fn draw_vline_content(&mut self, vline: &VLine, idx: usize) -> Result<(), UnidError> {
        if let Some(legend) = &vline.legend {
            let effective_pos = match legend.pos {
                LegendPos::Auto | LegendPos::Right => LegendPos::Right,
                other => other,
            };
            let mid_row = vline.row + vline.length / 2;
            let (lg_col, lg_row) = match effective_pos {
                LegendPos::Top => (vline.col, vline.row.saturating_sub(1)),
                LegendPos::Bottom => (vline.col, vline.row + vline.length),
                LegendPos::Left => {
                    let tw = width::str_width(&legend.text);
                    (vline.col.saturating_sub(tw + 1), mid_row)
                }
                LegendPos::Right => (vline.col + 2, mid_row),
                LegendPos::Auto => unreachable!(),
            };
            self.draw_legend_text(lg_col, lg_row, legend, idx)?;
        }
        Ok(())
    }

    fn draw_arrow(&mut self, arrow: &ResolvedArrow, idx: usize) -> Result<(), UnidError> {
        self.draw_arrow_structure(arrow, idx)?;
        self.draw_arrow_content(arrow, idx)
    }

    /// Draws arrow structural elements (lines, corners, arrowheads).
    fn draw_arrow_structure(&mut self, arrow: &ResolvedArrow, idx: usize) -> Result<(), UnidError> {
        let wp = &arrow.waypoints;
        if wp.len() < 2 {
            return Ok(());
        }

        // Draw each segment between consecutive waypoints
        for i in 0..wp.len() - 1 {
            let (fc, fr) = wp[i];
            let (tc, tr) = wp[i + 1];
            let is_last = i == wp.len() - 2;

            self.draw_straight_segment(fc, fr, tc, tr, is_last, arrow.head, idx)?;
        }

        // Draw corner characters at intermediate waypoints
        for i in 1..wp.len() - 1 {
            let (pc, pr) = wp[i - 1];
            let (cc, cr) = wp[i];
            let (nc, nr) = wp[i + 1];
            let incoming = segment_dir(pc, pr, cc, cr);
            let outgoing = segment_dir(cc, cr, nc, nr);
            let corner = corner_char(incoming, outgoing);
            self.canvas.put_char(cc, cr, corner, false, idx)?;
        }

        // Bidirectional: add arrowhead at the start point (reverse direction).
        // Resolves to the correct direction variant from the same arrowhead family.
        if arrow.both && wp.len() >= 2 {
            let (sc, sr) = wp[0];
            let (nc, nr) = wp[1];
            let dir = segment_dir(nc, nr, sc, sr);
            let tip = match arrow.head {
                Some(ch) => resolve_arrowhead(ch, dir),
                None => default_arrowhead(dir),
            };
            self.canvas.put_char(sc, sr, tip, false, idx)?;
        }

        Ok(())
    }

    /// Draws arrow text content (legend).
    fn draw_arrow_content(&mut self, arrow: &ResolvedArrow, idx: usize) -> Result<(), UnidError> {
        if let Some(legend) = &arrow.legend {
            self.draw_arrow_legend(&arrow.waypoints, legend, idx)?;
        }
        Ok(())
    }

    /// Draws arrow legend text near the midpoint of a segment.
    /// For bent arrows (3+ waypoints), uses the longest segment from the second onwards.
    /// For straight arrows (2 waypoints), uses the only segment.
    fn draw_arrow_legend(
        &mut self,
        wp: &[(usize, usize)],
        legend: &Legend,
        idx: usize,
    ) -> Result<(), UnidError> {
        if wp.len() < 2 {
            return Ok(());
        }

        // For bent arrows, pick the longest segment from the second one onwards.
        let seg_idx = if wp.len() >= 3 {
            (1..wp.len() - 1)
                .max_by_key(|&i| {
                    let (fc, fr) = wp[i];
                    let (tc, tr) = wp[i + 1];
                    fc.abs_diff(tc) + fr.abs_diff(tr)
                })
                .unwrap_or(1)
        } else {
            0
        };

        let (fc, fr) = wp[seg_idx];
        let (tc, tr) = wp[seg_idx + 1];
        let mid_c = (fc + tc) / 2;
        let mid_r = (fr + tr) / 2;
        let dir = segment_dir(fc, fr, tc, tr);
        let is_horizontal = matches!(dir, Dir::Left | Dir::Right);

        let effective_pos = match legend.pos {
            LegendPos::Auto => {
                if is_horizontal { LegendPos::Top } else { LegendPos::Right }
            }
            other => other,
        };

        let text_w = width::str_width(&legend.text);
        let (lg_col, lg_row) = match effective_pos {
            LegendPos::Top => (mid_c.saturating_sub(text_w / 2), mid_r.saturating_sub(1)),
            LegendPos::Bottom => (mid_c.saturating_sub(text_w / 2), mid_r + 1),
            LegendPos::Left => (mid_c.saturating_sub(text_w + 1), mid_r),
            LegendPos::Right => (mid_c + 1, mid_r),
            LegendPos::Auto => unreachable!(),
        };

        self.draw_legend_text(lg_col, lg_row, legend, idx)
    }

    /// Draws a straight line segment (horizontal or vertical).
    /// If `with_tip`, the endpoint gets an arrowhead (custom `tip_char` or default).
    #[allow(clippy::too_many_arguments)]
    fn draw_straight_segment(
        &mut self,
        fc: usize,
        fr: usize,
        tc: usize,
        tr: usize,
        with_tip: bool,
        tip_char: Option<char>,
        idx: usize,
    ) -> Result<(), UnidError> {
        if fr == tr {
            // Horizontal
            let (min, max) = if fc < tc { (fc, tc) } else { (tc, fc) };
            for c in min..=max {
                self.canvas.put_char(c, fr, '─', self.collision, idx)?;
            }
            if with_tip {
                let dir = if tc > fc { Dir::Right } else { Dir::Left };
                let tip = match tip_char {
                    Some(ch) => resolve_arrowhead(ch, dir),
                    None => default_arrowhead(dir),
                };
                self.canvas.put_char(tc, fr, tip, self.collision, idx)?;
            }
        } else if fc == tc {
            // Vertical
            let (min, max) = if fr < tr { (fr, tr) } else { (tr, fr) };
            for r in min..=max {
                self.canvas.put_char(fc, r, '│', self.collision, idx)?;
            }
            if with_tip {
                let dir = if tr > fr { Dir::Down } else { Dir::Up };
                let tip = match tip_char {
                    Some(ch) => resolve_arrowhead(ch, dir),
                    None => default_arrowhead(dir),
                };
                self.canvas.put_char(fc, tr, tip, self.collision, idx)?;
            }
        }
        Ok(())
    }
}

/// Truncates content to fit inner_w using "prefix..{N}" or "{N}..suffix" format.
fn ellipsis_truncate(
    content: &str,
    content_w: usize,
    inner_w: usize,
    align: ContentAlign,
) -> String {
    if inner_w < 2 {
        return "..".to_string()[..inner_w.min(2)].to_string();
    }

    match align {
        ContentAlign::Left | ContentAlign::Center => {
            // "prefix..{N}" format
            // Try different digit lengths
            for digits in 1..=5 {
                let suffix_len = 2 + digits; // ".." + digits
                if suffix_len > inner_w {
                    continue;
                }
                let prefix_space = inner_w - suffix_len;
                let prefix = truncate_to_width(content, prefix_space);
                let prefix_w = width::str_width(&prefix);
                let truncated_w = content_w - prefix_w;
                let n_str = truncated_w.to_string();
                if n_str.len() == digits {
                    let result = format!("{}..{}", prefix, n_str);
                    if width::str_width(&result) <= inner_w {
                        return result;
                    }
                }
            }
            "..".to_string()
        }
        ContentAlign::Right => {
            // "{N}..suffix" format
            for digits in 1..=5 {
                let prefix_len = digits + 2; // digits + ".."
                if prefix_len > inner_w {
                    continue;
                }
                let suffix_space = inner_w - prefix_len;
                let suffix = truncate_from_end(content, suffix_space);
                let suffix_w = width::str_width(&suffix);
                let truncated_w = content_w - suffix_w;
                let n_str = truncated_w.to_string();
                if n_str.len() == digits {
                    let result = format!("{}..{}", n_str, suffix);
                    if width::str_width(&result) <= inner_w {
                        return result;
                    }
                }
            }
            "..".to_string()
        }
    }
}

/// Truncates content for hidden mode.
fn hidden_truncate(content: &str, inner_w: usize, align: ContentAlign) -> String {
    match align {
        ContentAlign::Left | ContentAlign::Center => truncate_to_width(content, inner_w),
        ContentAlign::Right => truncate_from_end(content, inner_w),
    }
}

/// Truncates a string from the beginning to fit within `max_width` display columns.
fn truncate_to_width(s: &str, max_width: usize) -> String {
    let mut result = String::new();
    let mut current_w = 0;
    for ch in s.chars() {
        let w = width::char_width(ch);
        if current_w + w > max_width {
            break;
        }
        result.push(ch);
        current_w += w;
    }
    result
}

/// Truncates a string from the end to fit within `max_width` display columns.
fn truncate_from_end(s: &str, max_width: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::new();
    let mut current_w = 0;
    for &ch in chars.iter().rev() {
        let w = width::char_width(ch);
        if current_w + w > max_width {
            break;
        }
        result.insert(0, ch);
        current_w += w;
    }
    result
}

fn border_chars(style: BorderStyle) -> (char, char, char, char, char, char) {
    match style {
        BorderStyle::Light => ('┌', '┐', '└', '┘', '─', '│'),
        BorderStyle::Heavy => ('┏', '┓', '┗', '┛', '━', '┃'),
        BorderStyle::Double => ('╔', '╗', '╚', '╝', '═', '║'),
        BorderStyle::Rounded => ('╭', '╮', '╰', '╯', '─', '│'),
    }
}

fn hline_char(style: LineStyle) -> char {
    match style {
        LineStyle::Light => '─',
        LineStyle::Heavy => '━',
        LineStyle::Double => '═',
        LineStyle::Dash => '╌',
    }
}

fn vline_char(style: LineStyle) -> char {
    match style {
        LineStyle::Light => '│',
        LineStyle::Heavy => '┃',
        LineStyle::Double => '║',
        LineStyle::Dash => '╎',
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::*;
    use pretty_assertions::assert_eq;

    fn render_objects(
        width: usize,
        height: usize,
        objects: &[DrawObject],
        collision: bool,
    ) -> String {
        let canvas = Canvas::new(width, height);
        let mut renderer = Renderer::new(canvas, collision);
        renderer.draw_all(objects).unwrap();
        renderer.render()
    }

    #[test]
    fn render_simple_rect() {
        let rect = Rect::new(0, 0, 4, 1);
        let result = render_objects(6, 3, &[DrawObject::Rect(rect)], false);
        assert_eq!(
            result,
            "\
┌────┐\n\
│    │\n\
└────┘"
        );
    }

    #[test]
    fn render_rect_with_content() {
        let mut rect = Rect::new(0, 0, 8, 1);
        rect.content = Some("Hello".to_string());
        let result = render_objects(10, 3, &[DrawObject::Rect(rect)], false);
        // Default align=Left, but content fits → center
        // "Hello" is 5 wide, inner is 8, but Left align: pad=0
        assert_eq!(
            result,
            "\
┌────────┐\n\
│Hello   │\n\
└────────┘"
        );
    }

    #[test]
    fn render_rect_with_center_aligned_content() {
        let mut rect = Rect::new(0, 0, 8, 1);
        rect.content = Some("Hello".to_string());
        rect.content_align = ContentAlign::Center;
        let result = render_objects(10, 3, &[DrawObject::Rect(rect)], false);
        assert_eq!(
            result,
            "\
┌────────┐\n\
│ Hello  │\n\
└────────┘"
        );
    }

    #[test]
    fn render_rect_with_cjk_content() {
        let mut rect = Rect::new(0, 0, 10, 1);
        rect.content = Some("한글".to_string());
        rect.content_align = ContentAlign::Center;
        let result = render_objects(12, 3, &[DrawObject::Rect(rect)], false);
        assert_eq!(
            result,
            "\
┌──────────┐\n\
│   한글   │\n\
└──────────┘"
        );
    }

    #[test]
    fn render_heavy_rect() {
        let mut rect = Rect::new(0, 0, 4, 1);
        rect.style = BorderStyle::Heavy;
        let result = render_objects(6, 3, &[DrawObject::Rect(rect)], false);
        assert_eq!(
            result,
            "\
┏━━━━┓\n\
┃    ┃\n\
┗━━━━┛"
        );
    }

    #[test]
    fn render_double_rect() {
        let mut rect = Rect::new(0, 0, 4, 1);
        rect.style = BorderStyle::Double;
        let result = render_objects(6, 3, &[DrawObject::Rect(rect)], false);
        assert_eq!(
            result,
            "\
╔════╗\n\
║    ║\n\
╚════╝"
        );
    }

    #[test]
    fn render_rounded_rect() {
        let mut rect = Rect::new(0, 0, 4, 1);
        rect.style = BorderStyle::Rounded;
        let result = render_objects(6, 3, &[DrawObject::Rect(rect)], false);
        assert_eq!(
            result,
            "\
╭────╮\n\
│    │\n\
╰────╯"
        );
    }

    #[test]
    fn render_text() {
        let text = Text::new(2, 1, "Hello");
        let result = render_objects(10, 3, &[DrawObject::Text(text)], false);
        assert_eq!(result, "\n  Hello");
    }

    #[test]
    fn render_text_cjk() {
        let text = Text::new(0, 0, "한글ABC");
        let result = render_objects(10, 1, &[DrawObject::Text(text)], false);
        assert_eq!(result, "한글ABC");
    }

    #[test]
    fn render_hline() {
        let hline = HLine::new(1, 0, 5);
        let result = render_objects(7, 1, &[DrawObject::HLine(hline)], false);
        assert_eq!(result, " ─────");
    }

    #[test]
    fn render_hline_heavy() {
        let mut hline = HLine::new(0, 0, 3);
        hline.style = LineStyle::Heavy;
        let result = render_objects(3, 1, &[DrawObject::HLine(hline)], false);
        assert_eq!(result, "━━━");
    }

    #[test]
    fn render_vline() {
        let vline = VLine::new(0, 0, 3);
        let result = render_objects(1, 3, &[DrawObject::VLine(vline)], false);
        assert_eq!(result, "│\n│\n│");
    }

    #[test]
    fn render_horizontal_arrow_right() {
        let arrow = ResolvedArrow { waypoints: vec![(0, 0), (4, 0)], head: None, both: false, legend: None };
        let result = render_objects(5, 1, &[DrawObject::Arrow(arrow)], false);
        assert_eq!(result, "────→");
    }

    #[test]
    fn render_horizontal_arrow_left() {
        let arrow = ResolvedArrow { waypoints: vec![(4, 0), (0, 0)], head: None, both: false, legend: None };
        let result = render_objects(5, 1, &[DrawObject::Arrow(arrow)], false);
        assert_eq!(result, "←────");
    }

    #[test]
    fn render_vertical_arrow_down() {
        let arrow = ResolvedArrow { waypoints: vec![(0, 0), (0, 2)], head: None, both: false, legend: None };
        let result = render_objects(1, 3, &[DrawObject::Arrow(arrow)], false);
        assert_eq!(result, "│\n│\n↓");
    }

    #[test]
    fn render_vertical_arrow_up() {
        let arrow = ResolvedArrow { waypoints: vec![(0, 2), (0, 0)], head: None, both: false, legend: None };
        let result = render_objects(1, 3, &[DrawObject::Arrow(arrow)], false);
        assert_eq!(result, "↑\n│\n│");
    }

    #[test]
    fn collision_detected_between_objects() {
        let canvas = Canvas::new(10, 3);
        let mut renderer = Renderer::new(canvas, true);
        let rect = Rect::new(0, 0, 4, 1);
        let text = Text::new(0, 0, "X");
        renderer.draw(&DrawObject::Rect(rect)).unwrap();
        let result = renderer.draw(&DrawObject::Text(text));
        assert!(result.is_err());
    }

    #[test]
    fn collision_off_allows_overlap() {
        let canvas = Canvas::new(10, 3);
        let mut renderer = Renderer::new(canvas, false);
        let rect = Rect::new(0, 0, 4, 1);
        let text = Text::new(0, 0, "X");
        renderer.draw(&DrawObject::Rect(rect)).unwrap();
        renderer.draw(&DrawObject::Text(text)).unwrap();
        let output = renderer.render();
        assert!(output.starts_with('X'));
    }

    #[test]
    fn render_multiple_objects() {
        let objects = vec![
            DrawObject::Rect(Rect::new(0, 0, 8, 1)),
            DrawObject::Text(Text::new(15, 1, "World")),
        ];
        let result = render_objects(20, 3, &objects, false);
        assert!(result.contains("┌────────┐"));
        assert!(result.contains("World"));
    }

    #[test]
    fn render_rect_multiline_inner() {
        let mut rect = Rect::new(0, 0, 6, 3);
        rect.content = Some("Hi".to_string());
        rect.content_align = ContentAlign::Center;
        let result = render_objects(8, 5, &[DrawObject::Rect(rect)], false);
        assert_eq!(
            result,
            "\
┌──────┐\n\
│      │\n\
│  Hi  │\n\
│      │\n\
└──────┘"
        );
    }

    #[test]
    fn ellipsis_truncate_basic() {
        // "HelloWorld" (10 cols), inner_w=7 → "Hell..6"
        let result = ellipsis_truncate("HelloWorld", 10, 7, ContentAlign::Left);
        assert_eq!(result, "Hell..6");
        assert!(width::str_width(&result) <= 7);
    }

    #[test]
    fn ellipsis_truncate_right() {
        // "HelloWorld" (10 cols), inner_w=7 → "6..orld"
        let result = ellipsis_truncate("HelloWorld", 10, 7, ContentAlign::Right);
        assert_eq!(result, "6..orld");
    }

    #[test]
    fn ellipsis_truncate_very_small() {
        let result = ellipsis_truncate("Hello", 5, 2, ContentAlign::Left);
        assert_eq!(result, "..");
    }

    #[test]
    fn hidden_truncate_left() {
        let result = hidden_truncate("HelloWorld", 5, ContentAlign::Left);
        assert_eq!(result, "Hello");
    }

    #[test]
    fn hidden_truncate_right() {
        let result = hidden_truncate("HelloWorld", 5, ContentAlign::Right);
        assert_eq!(result, "World");
    }

    #[test]
    fn render_rect_ellipsis_overflow() {
        let mut rect = Rect::new(0, 0, 7, 1);
        rect.content = Some("HelloWorld".to_string());
        rect.content_overflow = ContentOverflow::Ellipsis;
        let result = render_objects(9, 3, &[DrawObject::Rect(rect)], false);
        assert!(result.contains("Hell..6"));
    }

    #[test]
    fn render_rect_hidden_overflow() {
        let mut rect = Rect::new(0, 0, 5, 1);
        rect.content = Some("HelloWorld".to_string());
        rect.content_overflow = ContentOverflow::Hidden;
        let result = render_objects(7, 3, &[DrawObject::Rect(rect)], false);
        assert!(result.contains("Hello"));
        assert!(!result.contains("World"));
    }

    #[test]
    fn render_l_shaped_arrow() {
        // L-shape: right then down via bend at (3,0)
        let arrow = ResolvedArrow { waypoints: vec![(0, 0), (3, 0), (3, 2)], head: None, both: false, legend: None };
        let result = render_objects(4, 3, &[DrawObject::Arrow(arrow)], false);
        assert_eq!(result, "───┐\n   │\n   ↓");
    }

    #[test]
    fn render_z_shaped_arrow() {
        // Z-shape: down, then right, then down
        let arrow = ResolvedArrow { waypoints: vec![(0, 0), (0, 2), (4, 2), (4, 4)], head: None, both: false, legend: None };
        let result = render_objects(5, 5, &[DrawObject::Arrow(arrow)], false);
        assert!(result.contains('│'));
        assert!(result.contains('└'));
        assert!(result.contains('┐'));
        assert!(result.contains('↓'));
    }

    #[test]
    fn render_u_shaped_arrow() {
        // U-shape: down, right, then up
        let arrow = ResolvedArrow { waypoints: vec![(0, 0), (0, 3), (4, 3), (4, 0)], head: None, both: false, legend: None };
        let result = render_objects(5, 4, &[DrawObject::Arrow(arrow)], false);
        assert!(result.contains('└'));
        assert!(result.contains('┘'));
        assert!(result.contains('↑'));
    }

    #[test]
    fn render_rect_multiline_content() {
        let mut rect = Rect::new(0, 0, 6, 3);
        rect.content = Some("AA\nBB".to_string());
        rect.content_align = ContentAlign::Center;
        let result = render_objects(8, 5, &[DrawObject::Rect(rect)], false);
        // inner_h=3, 2 lines → start_row=0+1+(3-2)/2=1, lines at rows 1,2
        assert_eq!(
            result,
            "\
┌──────┐\n\
│  AA  │\n\
│  BB  │\n\
│      │\n\
└──────┘"
        );
    }

    #[test]
    fn render_text_multiline() {
        let text = Text::new(0, 0, "Hello\nWorld");
        let result = render_objects(5, 2, &[DrawObject::Text(text)], false);
        assert_eq!(result, "Hello\nWorld");
    }

    #[test]
    fn render_rect_error_overflow() {
        let mut rect = Rect::new(0, 0, 3, 1);
        rect.content = Some("VeryLong".to_string());
        rect.content_overflow = ContentOverflow::Error;
        let canvas = Canvas::new(5, 3);
        let mut renderer = Renderer::new(canvas, false);
        let result = renderer.draw(&DrawObject::Rect(rect));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("overflow"));
    }
}
