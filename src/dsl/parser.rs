use crate::dsl::command::{CanvasSize, DslCommand};
use crate::error::UnidError;
use crate::object::rect::Side;
use crate::object::{
    BorderStyle, ContentAlign, ContentOverflow, DrawObject, HLine, LineStyle, Rect, Text,
    VLine,
};

/// Parses DSL text into a list of commands.
/// Lines are separated by newlines only. Comments start with #.
pub fn parse(input: &str) -> Result<Vec<DslCommand>, UnidError> {
    let lines = split_lines(input);
    let mut commands = Vec::new();

    for (line_num, line) in lines.iter().enumerate() {
        let line_num = line_num + 1; // 1-based
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let tokens = tokenize(trimmed);
        if tokens.is_empty() {
            continue;
        }

        let cmd = parse_command(&tokens, line_num)?;
        commands.push(cmd);
    }

    Ok(commands)
}

/// Splits input into lines by newline only. No comma separation.
fn split_lines(input: &str) -> Vec<String> {
    input.lines().map(|l| l.to_string()).collect()
}

/// Tokenizes a line by whitespace. No quote handling.
fn tokenize(line: &str) -> Vec<String> {
    line.split_whitespace().map(|s| s.to_string()).collect()
}

/// Extracts a `content=` or `c=` value from tokens starting at `from`.
/// Everything after the `content=`/`c=` prefix (including remaining tokens) is joined.
/// Returns (content_string, index_after_content).
fn extract_content(tokens: &[String], from: usize, line: usize) -> Result<String, UnidError> {
    for i in from..tokens.len() {
        let token = &tokens[i];
        let value_start = token
            .strip_prefix("content=")
            .or_else(|| token.strip_prefix("c="));

        if let Some(first_part) = value_start {
            // content= must be the last option; gather remaining tokens
            let mut parts = Vec::new();
            if !first_part.is_empty() {
                parts.push(first_part.to_string());
            }
            for t in &tokens[i + 1..] {
                parts.push(t.clone());
            }
            if parts.is_empty() {
                return Err(UnidError::Parse {
                    line,
                    message: "content= requires a value".to_string(),
                });
            }
            let content = parts.join(" ");
            // Unescape \n to actual newlines
            let content = content.replace("\\n", "\n");
            // Trim leading/trailing whitespace from each line
            let content = content
                .lines()
                .map(|l| l.trim())
                .collect::<Vec<_>>()
                .join("\n");
            return Ok(content);
        }
    }
    Err(UnidError::Parse {
        line,
        message: "missing content= (or c=)".to_string(),
    })
}

/// Finds the index of the content= or c= token, or returns tokens.len() if not found.
fn content_token_index(tokens: &[String], from: usize) -> usize {
    for (i, token) in tokens.iter().enumerate().skip(from) {
        if token.starts_with("content=") || token.starts_with("c=") {
            return i;
        }
    }
    tokens.len()
}

fn parse_command(tokens: &[String], line: usize) -> Result<DslCommand, UnidError> {
    let keyword = tokens[0].to_lowercase();
    match keyword.as_str() {
        "canvas" => parse_canvas(tokens, line),
        "collision" => parse_collision(tokens, line),
        "rect" => parse_rect(tokens, line),
        "text" => parse_text(tokens, line),
        "hline" => parse_hline(tokens, line),
        "vline" => parse_vline(tokens, line),
        "arrow" => parse_arrow(tokens, line),
        _ => Err(UnidError::Parse {
            line,
            message: format!("unknown command '{}'", tokens[0]),
        }),
    }
}

fn parse_canvas(tokens: &[String], line: usize) -> Result<DslCommand, UnidError> {
    if tokens.len() < 2 {
        return Err(UnidError::Parse {
            line,
            message: "canvas requires size arguments (e.g., 'canvas 40 10' or 'canvas auto')"
                .to_string(),
        });
    }

    let (width, height, opts_start) = if tokens[1].to_lowercase() == "auto" {
        (CanvasSize::Auto, CanvasSize::Auto, 2)
    } else {
        if tokens.len() < 3 {
            return Err(UnidError::Parse {
                line,
                message: "canvas requires width and height (e.g., 'canvas 40 10')".to_string(),
            });
        }
        let w = parse_usize(&tokens[1], "canvas width", line)?;
        let h = parse_usize(&tokens[2], "canvas height", line)?;
        (CanvasSize::Fixed(w), CanvasSize::Fixed(h), 3)
    };

    let mut border = None;
    let mut content_overflow = None;
    let mut content_align = None;

    for token in &tokens[opts_start..] {
        if let Some(v) = strip_option(token, "border") {
            border = Some(parse_border_style(v, line)?);
        } else if let Some(v) = strip_option(token, "overflow")
        {
            content_overflow = Some(parse_content_overflow(v, line)?);
        } else if let Some(v) =
            strip_option(token, "align")
        {
            content_align = Some(parse_content_align(v, line)?);
        } else {
            return Err(UnidError::Parse {
                line,
                message: format!("unknown canvas option '{}'", token),
            });
        }
    }

    Ok(DslCommand::Canvas {
        width,
        height,
        border,
        content_overflow,
        content_align,
    })
}

fn parse_collision(tokens: &[String], line: usize) -> Result<DslCommand, UnidError> {
    if tokens.len() < 2 {
        return Err(UnidError::Parse {
            line,
            message: "collision requires 'on' or 'off'".to_string(),
        });
    }

    match tokens[1].to_lowercase().as_str() {
        "on" => Ok(DslCommand::Collision(true)),
        "off" => Ok(DslCommand::Collision(false)),
        _ => Err(UnidError::Parse {
            line,
            message: format!("collision must be 'on' or 'off', got '{}'", tokens[1]),
        }),
    }
}

fn parse_rect(tokens: &[String], line: usize) -> Result<DslCommand, UnidError> {
    if tokens.len() < 5 {
        return Err(UnidError::Parse {
            line,
            message: "rect requires col, row, width, height".to_string(),
        });
    }

    let col = parse_usize(&tokens[1], "col", line)?;
    let row = parse_usize(&tokens[2], "row", line)?;
    let width = parse_usize(&tokens[3], "width", line)?;
    let height = parse_usize(&tokens[4], "height", line)?;

    let mut rect = Rect::new(col, row, width, height);

    // Parse options up to content= (content must be last)
    let content_idx = content_token_index(tokens, 5);

    for token in &tokens[5..content_idx] {
        if let Some(v) = strip_option(token, "id") {
            validate_id(v, line)?;
            rect.id = Some(v.to_string());
        } else if let Some(v) = strip_option(token, "style").or_else(|| strip_option(token, "s")) {
            rect.style = parse_border_style(v, line)?;
        } else if let Some(v) = strip_option(token, "overflow")
        {
            rect.content_overflow = parse_content_overflow(v, line)?;
        } else if let Some(v) =
            strip_option(token, "align")
        {
            rect.content_align = parse_content_align(v, line)?;
        } else {
            return Err(UnidError::Parse {
                line,
                message: format!("unknown rect option '{}'", token),
            });
        }
    }

    // Parse content if present
    if content_idx < tokens.len() {
        let content = extract_content(tokens, content_idx, line)?;
        rect.content = Some(content);
    }

    Ok(DslCommand::Object(DrawObject::Rect(rect)))
}

fn parse_text(tokens: &[String], line: usize) -> Result<DslCommand, UnidError> {
    if tokens.len() < 4 {
        return Err(UnidError::Parse {
            line,
            message: "text requires col, row, content=<value>".to_string(),
        });
    }

    let col = parse_usize(&tokens[1], "col", line)?;
    let row = parse_usize(&tokens[2], "row", line)?;
    let content = extract_content(tokens, 3, line)?;

    Ok(DslCommand::Object(DrawObject::Text(Text::new(
        col, row, content,
    ))))
}

fn parse_hline(tokens: &[String], line: usize) -> Result<DslCommand, UnidError> {
    if tokens.len() < 4 {
        return Err(UnidError::Parse {
            line,
            message: "hline requires col, row, length".to_string(),
        });
    }

    let col = parse_usize(&tokens[1], "col", line)?;
    let row = parse_usize(&tokens[2], "row", line)?;
    let length = parse_usize(&tokens[3], "length", line)?;

    let mut hline = HLine::new(col, row, length);

    for token in &tokens[4..] {
        if let Some(v) = strip_option(token, "style").or_else(|| strip_option(token, "s")) {
            hline.style = parse_line_style(v, line)?;
        } else {
            return Err(UnidError::Parse {
                line,
                message: format!("unknown hline option '{}'", token),
            });
        }
    }

    Ok(DslCommand::Object(DrawObject::HLine(hline)))
}

fn parse_vline(tokens: &[String], line: usize) -> Result<DslCommand, UnidError> {
    if tokens.len() < 4 {
        return Err(UnidError::Parse {
            line,
            message: "vline requires col, row, length".to_string(),
        });
    }

    let col = parse_usize(&tokens[1], "col", line)?;
    let row = parse_usize(&tokens[2], "row", line)?;
    let length = parse_usize(&tokens[3], "length", line)?;

    let mut vline = VLine::new(col, row, length);

    for token in &tokens[4..] {
        if let Some(v) = strip_option(token, "style").or_else(|| strip_option(token, "s")) {
            vline.style = parse_line_style(v, line)?;
        } else {
            return Err(UnidError::Parse {
                line,
                message: format!("unknown vline option '{}'", token),
            });
        }
    }

    Ok(DslCommand::Object(DrawObject::VLine(vline)))
}

fn parse_arrow(tokens: &[String], line: usize) -> Result<DslCommand, UnidError> {
    if tokens.len() < 3 {
        return Err(UnidError::Parse {
            line,
            message: "arrow requires <src_id>.<side> <dst_id>.<side> (e.g., 'arrow api.right db.top')".to_string(),
        });
    }

    let (src_id, src_side) = parse_anchor(&tokens[1], line)?;
    let (dst_id, dst_side) = parse_anchor(&tokens[2], line)?;

    Ok(DslCommand::Arrow {
        src_id,
        src_side,
        dst_id,
        dst_side,
        line,
    })
}

/// Parses an anchor reference like "api.right" into ("api", Side::Right).
fn parse_anchor(s: &str, line: usize) -> Result<(String, Side), UnidError> {
    let (id, side_str) = s.rsplit_once('.').ok_or_else(|| UnidError::Parse {
        line,
        message: format!("invalid anchor '{}' (expected <id>.<side>, e.g., 'api.right')", s),
    })?;

    if id.is_empty() {
        return Err(UnidError::Parse {
            line,
            message: format!("anchor '{}' has empty id", s),
        });
    }

    validate_id(id, line)?;

    let side = match side_str.to_lowercase().as_str() {
        "top" | "t" => Side::Top,
        "right" | "r" => Side::Right,
        "bottom" | "b" => Side::Bottom,
        "left" | "l" => Side::Left,
        _ => {
            return Err(UnidError::Parse {
                line,
                message: format!(
                    "unknown side '{}' in anchor '{}' (expected top/t, right/r, bottom/b, left/l)",
                    side_str, s
                ),
            });
        }
    };

    Ok((id.to_string(), side))
}

/// Validates that an ID contains only alphanumeric, underscore, or hyphen characters.
fn validate_id(id: &str, line: usize) -> Result<(), UnidError> {
    if id.is_empty() {
        return Err(UnidError::Parse {
            line,
            message: "id cannot be empty".to_string(),
        });
    }
    if !id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(UnidError::Parse {
            line,
            message: format!(
                "invalid id '{}' (only alphanumeric, '_', '-' allowed)",
                id
            ),
        });
    }
    Ok(())
}

/// Strips a key= prefix from a token. Supports both full name and abbreviation.
fn strip_option<'a>(token: &'a str, key: &str) -> Option<&'a str> {
    token.strip_prefix(&format!("{}=", key))
}

fn parse_border_style(s: &str, line: usize) -> Result<BorderStyle, UnidError> {
    match s.to_lowercase().as_str() {
        "light" | "l" => Ok(BorderStyle::Light),
        "heavy" | "h" => Ok(BorderStyle::Heavy),
        "double" | "d" => Ok(BorderStyle::Double),
        "rounded" | "r" => Ok(BorderStyle::Rounded),
        "none" => Ok(BorderStyle::Light), // none treated as light for border= on canvas
        _ => Err(UnidError::Parse {
            line,
            message: format!(
                "unknown border style '{}' (expected light/l, heavy/h, double/d, rounded/r)",
                s
            ),
        }),
    }
}

fn parse_line_style(s: &str, line: usize) -> Result<LineStyle, UnidError> {
    match s.to_lowercase().as_str() {
        "light" | "l" => Ok(LineStyle::Light),
        "heavy" | "h" => Ok(LineStyle::Heavy),
        "double" | "d" => Ok(LineStyle::Double),
        "dash" => Ok(LineStyle::Dash),
        _ => Err(UnidError::Parse {
            line,
            message: format!(
                "unknown line style '{}' (expected light/l, heavy/h, double/d, dash)",
                s
            ),
        }),
    }
}

fn parse_content_overflow(s: &str, line: usize) -> Result<ContentOverflow, UnidError> {
    match s.to_lowercase().as_str() {
        "ellipsis" => Ok(ContentOverflow::Ellipsis),
        "overflow" => Ok(ContentOverflow::Overflow),
        "hidden" => Ok(ContentOverflow::Hidden),
        "error" => Ok(ContentOverflow::Error),
        _ => Err(UnidError::Parse {
            line,
            message: format!(
                "unknown overflow mode '{}' (expected ellipsis, overflow, hidden, error)",
                s
            ),
        }),
    }
}

fn parse_content_align(s: &str, line: usize) -> Result<ContentAlign, UnidError> {
    match s.to_lowercase().as_str() {
        "left" | "l" => Ok(ContentAlign::Left),
        "center" | "c" => Ok(ContentAlign::Center),
        "right" | "r" => Ok(ContentAlign::Right),
        _ => Err(UnidError::Parse {
            line,
            message: format!(
                "unknown align '{}' (expected left/l, center/c, right/r)",
                s
            ),
        }),
    }
}

fn parse_usize(s: &str, name: &str, line: usize) -> Result<usize, UnidError> {
    s.parse().map_err(|_| UnidError::Parse {
        line,
        message: format!("invalid {} '{}' (expected a non-negative integer)", name, s),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::command::CanvasSize;

    #[test]
    fn parse_canvas_fixed() {
        let cmds = parse("canvas 40 10").unwrap();
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            DslCommand::Canvas { width, height, .. } => {
                assert_eq!(*width, CanvasSize::Fixed(40));
                assert_eq!(*height, CanvasSize::Fixed(10));
            }
            _ => panic!("expected Canvas"),
        }
    }

    #[test]
    fn parse_canvas_auto() {
        let cmds = parse("canvas auto").unwrap();
        match &cmds[0] {
            DslCommand::Canvas { width, height, .. } => {
                assert_eq!(*width, CanvasSize::Auto);
                assert_eq!(*height, CanvasSize::Auto);
            }
            _ => panic!("expected Canvas"),
        }
    }

    #[test]
    fn parse_canvas_with_border() {
        let cmds = parse("canvas 20 5 border=r").unwrap();
        match &cmds[0] {
            DslCommand::Canvas { border, .. } => {
                assert_eq!(*border, Some(BorderStyle::Rounded));
            }
            _ => panic!("expected Canvas"),
        }
    }

    #[test]
    fn parse_canvas_with_global_defaults() {
        let cmds = parse("canvas 20 5 overflow=hidden align=r").unwrap();
        match &cmds[0] {
            DslCommand::Canvas {
                content_overflow,
                content_align,
                ..
            } => {
                assert_eq!(*content_overflow, Some(ContentOverflow::Hidden));
                assert_eq!(*content_align, Some(ContentAlign::Right));
            }
            _ => panic!("expected Canvas"),
        }
    }

    #[test]
    fn parse_collision_on_off() {
        let on = parse("collision on").unwrap();
        let off = parse("collision off").unwrap();
        match &on[0] {
            DslCommand::Collision(v) => assert!(*v),
            _ => panic!("expected Collision"),
        }
        match &off[0] {
            DslCommand::Collision(v) => assert!(!(*v)),
            _ => panic!("expected Collision"),
        }
    }

    #[test]
    fn parse_rect_basic() {
        let cmds = parse("rect 0 0 10 3").unwrap();
        match &cmds[0] {
            DslCommand::Object(DrawObject::Rect(r)) => {
                assert_eq!((r.col, r.row, r.width, r.height), (0, 0, 10, 3));
                assert!(r.content.is_none());
                assert_eq!(r.style, BorderStyle::Light);
            }
            _ => panic!("expected Rect"),
        }
    }

    #[test]
    fn parse_rect_with_content_and_style() {
        let cmds = parse("rect 0 0 10 3 s=r c=Hello World").unwrap();
        match &cmds[0] {
            DslCommand::Object(DrawObject::Rect(r)) => {
                assert_eq!(r.content.as_deref(), Some("Hello World"));
                assert_eq!(r.style, BorderStyle::Rounded);
            }
            _ => panic!("expected Rect"),
        }
    }

    #[test]
    fn parse_rect_shorthand() {
        let cmds = parse("rect 0 0 10 3 s=h c=Test").unwrap();
        match &cmds[0] {
            DslCommand::Object(DrawObject::Rect(r)) => {
                assert_eq!(r.style, BorderStyle::Heavy);
                assert_eq!(r.content.as_deref(), Some("Test"));
            }
            _ => panic!("expected Rect"),
        }
    }

    #[test]
    fn parse_rect_with_overflow_options() {
        let cmds = parse("rect 0 0 10 3 overflow=hidden align=r c=Text").unwrap();
        match &cmds[0] {
            DslCommand::Object(DrawObject::Rect(r)) => {
                assert_eq!(r.content_overflow, ContentOverflow::Hidden);
                assert_eq!(r.content_align, ContentAlign::Right);
                assert_eq!(r.content.as_deref(), Some("Text"));
            }
            _ => panic!("expected Rect"),
        }
    }

    #[test]
    fn parse_text_with_content() {
        let cmds = parse("text 5 3 c=Hello 한글").unwrap();
        match &cmds[0] {
            DslCommand::Object(DrawObject::Text(t)) => {
                assert_eq!(t.col, 5);
                assert_eq!(t.row, 3);
                assert_eq!(t.content, "Hello 한글");
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn parse_text_with_comma_in_content() {
        let cmds = parse("text 0 0 c=Hello, World").unwrap();
        match &cmds[0] {
            DslCommand::Object(DrawObject::Text(t)) => {
                assert_eq!(t.content, "Hello, World");
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn parse_text_content_with_newline_escape() {
        let cmds = parse(r"text 0 0 c=Line1\nLine2").unwrap();
        match &cmds[0] {
            DslCommand::Object(DrawObject::Text(t)) => {
                assert_eq!(t.content, "Line1\nLine2");
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn parse_hline() {
        let cmds = parse("hline 0 5 20 s=h").unwrap();
        match &cmds[0] {
            DslCommand::Object(DrawObject::HLine(h)) => {
                assert_eq!((h.col, h.row, h.length), (0, 5, 20));
                assert_eq!(h.style, LineStyle::Heavy);
            }
            _ => panic!("expected HLine"),
        }
    }

    #[test]
    fn parse_vline() {
        let cmds = parse("vline 10 0 5").unwrap();
        match &cmds[0] {
            DslCommand::Object(DrawObject::VLine(v)) => {
                assert_eq!((v.col, v.row, v.length), (10, 0, 5));
                assert_eq!(v.style, LineStyle::Light);
            }
            _ => panic!("expected VLine"),
        }
    }

    #[test]
    fn parse_arrow_anchor() {
        let cmds = parse("arrow api.right db.top").unwrap();
        match &cmds[0] {
            DslCommand::Arrow { src_id, src_side, dst_id, dst_side, .. } => {
                assert_eq!(src_id, "api");
                assert_eq!(*src_side, Side::Right);
                assert_eq!(dst_id, "db");
                assert_eq!(*dst_side, Side::Top);
            }
            _ => panic!("expected Arrow"),
        }
    }

    #[test]
    fn parse_arrow_shorthand_sides() {
        let cmds = parse("arrow a.r b.l").unwrap();
        match &cmds[0] {
            DslCommand::Arrow { src_side, dst_side, .. } => {
                assert_eq!(*src_side, Side::Right);
                assert_eq!(*dst_side, Side::Left);
            }
            _ => panic!("expected Arrow"),
        }
    }

    #[test]
    fn parse_arrow_invalid_anchor() {
        let result = parse("arrow api db.top");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid anchor"));
    }

    #[test]
    fn parse_arrow_invalid_side() {
        let result = parse("arrow api.up db.top");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("unknown side"));
    }

    #[test]
    fn parse_rect_with_id() {
        let cmds = parse("rect 0 0 10 3 id=api c=API").unwrap();
        match &cmds[0] {
            DslCommand::Object(DrawObject::Rect(r)) => {
                assert_eq!(r.id.as_deref(), Some("api"));
                assert_eq!(r.content.as_deref(), Some("API"));
            }
            _ => panic!("expected Rect"),
        }
    }

    #[test]
    fn parse_rect_invalid_id() {
        let result = parse("rect 0 0 10 3 id=a@b");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid id"));
    }

    #[test]
    fn parse_comments_and_blank_lines() {
        let input = "\
# This is a comment
canvas 20 5

collision on

# Another comment
rect 0 0 5 2
";
        let cmds = parse(input).unwrap();
        assert_eq!(cmds.len(), 3);
    }

    #[test]
    fn parse_case_insensitive() {
        let cmds = parse("CANVAS 20 5").unwrap();
        assert!(matches!(cmds[0], DslCommand::Canvas { .. }));

        let cmds = parse("Rect 0 0 5 2").unwrap();
        assert!(matches!(cmds[0], DslCommand::Object(DrawObject::Rect(_))));
    }

    #[test]
    fn parse_error_unknown_command() {
        let result = parse("unknown 1 2 3");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("unknown command"));
        assert!(err.contains("line 1"));
    }

    #[test]
    fn parse_error_invalid_number() {
        let result = parse("rect abc 0 5 2");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid col"));
    }

    #[test]
    fn parse_error_missing_args() {
        let result = parse("rect 0 0");
        assert!(result.is_err());
    }
}
