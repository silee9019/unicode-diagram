use std::io::Write;
use std::process::{Command, Stdio};

fn unid() -> Command {
    Command::new(env!("CARGO_BIN_EXE_unid"))
}

/// Pipe DSL input to unid via stdin and return (stdout, stderr, success).
fn run_stdin(input: &str) -> (String, String, bool) {
    let mut child = unid()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    (
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
        output.status.success(),
    )
}

/// Pipe DSL input to a subcommand (list, lint).
fn run_subcmd(subcmd: &str, input: &str) -> (String, String, bool) {
    let mut child = unid()
        .arg(subcmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    (
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
        output.status.success(),
    )
}

// ─── Render (stdin default) ──────────────────────────────────────────

#[test]
fn render_simple_rect() {
    let (stdout, _, ok) = run_stdin(
        "canvas 6 3\n\
         collision off\n\
         box 0 0 4 1",
    );
    assert!(ok);
    assert_eq!(stdout.trim(), "┌────┐\n│    │\n└────┘");
}

#[test]
fn render_rect_with_content() {
    let (stdout, _, ok) = run_stdin(
        "canvas 12 3\n\
         collision off\n\
         box 0 0 10 1 c=Hello",
    );
    assert!(ok);
    assert!(stdout.contains("Hello"));
    assert!(stdout.contains("┌"));
    assert!(stdout.contains("└"));
}

#[test]
fn render_cjk_content() {
    let (stdout, _, ok) = run_stdin(
        "canvas 14 3\n\
         collision off\n\
         box 0 0 12 1 c=한글 테스트",
    );
    assert!(ok);
    assert!(stdout.contains("한글 테스트"));
}

#[test]
fn render_auto_canvas() {
    let (stdout, _, ok) = run_stdin(
        "canvas auto\n\
         collision off\n\
         box 0 0 4 1",
    );
    assert!(ok);
    assert_eq!(stdout.trim(), "┌────┐\n│    │\n└────┘");
}

#[test]
fn render_multiple_styles() {
    let (stdout, _, ok) = run_stdin(
        "canvas 30 12\n\
         collision off\n\
         box 0 0 6 1 s=l\n\
         box 0 3 6 1 s=h\n\
         box 0 6 6 1 s=d\n\
         box 0 9 6 1 s=r",
    );
    assert!(ok);
    assert!(stdout.contains('┌')); // light
    assert!(stdout.contains('┏')); // heavy
    assert!(stdout.contains('╔')); // double
    assert!(stdout.contains('╭')); // rounded
}

#[test]
fn render_anchor_arrow_horizontal() {
    let (stdout, _, ok) = run_stdin(
        "canvas 30 5\n\
         collision off\n\
         box 0 0 6 1 id=a c=A\n\
         box 18 0 6 1 id=b c=B\n\
         arrow a.r b.l",
    );
    assert!(ok);
    assert!(stdout.contains('▶'));
    assert!(stdout.contains('─'));
}

#[test]
fn render_anchor_arrow_vertical() {
    let (stdout, _, ok) = run_stdin(
        "canvas 10 10\n\
         collision off\n\
         box 0 0 6 1 id=a c=A\n\
         box 0 6 6 1 id=b c=B\n\
         arrow a.b b.t",
    );
    assert!(ok);
    assert!(stdout.contains('▼'));
    assert!(stdout.contains('│'));
}

#[test]
fn render_anchor_arrow_l_shape() {
    let (stdout, _, ok) = run_stdin(
        "canvas 30 10\n\
         collision off\n\
         box 0 0 6 1 id=a c=A\n\
         box 18 6 6 1 id=b c=B\n\
         arrow a.r b.t",
    );
    assert!(ok);
    assert!(stdout.contains('▶') || stdout.contains('▼'));
    assert!(stdout.contains('─') || stdout.contains('│'));
}

#[test]
fn render_anchor_arrow_u_shape() {
    // Same side (right→right) → ㄷ-shape
    let (stdout, _, ok) = run_stdin(
        "canvas 20 10\n\
         collision off\n\
         box 0 0 6 1 id=a c=A\n\
         box 0 6 6 1 id=b c=B\n\
         arrow a.r b.r",
    );
    assert!(ok);
    // ㄷ-shape should have corners
    assert!(stdout.contains('┐') || stdout.contains('┘'));
}

#[test]
fn render_lines() {
    let (stdout, _, ok) = run_stdin(
        "canvas 10 5\n\
         collision off\n\
         hline 0 0 5\n\
         vline 0 1 4",
    );
    assert!(ok);
    assert!(stdout.contains('─'));
    assert!(stdout.contains('│'));
}

#[test]
fn render_cjk_mixed_diagram() {
    let (stdout, _, ok) = run_stdin(
        "canvas 30 10\n\
         collision off\n\
         box 0 0 12 1 id=srv c=서버\n\
         box 18 0 8 1 id=db c=DB\n\
         arrow srv.r db.l",
    );
    assert!(ok);
    assert!(stdout.contains("서버"));
    assert!(stdout.contains("DB"));
    assert!(stdout.contains('▶'));
}

// ─── Collision ───────────────────────────────────────────────────────

#[test]
fn collision_on_error() {
    let (_, stderr, ok) = run_stdin(
        "canvas 20 5\n\
         collision on\n\
         box 0 0 5 1\n\
         box 3 0 5 1",
    );
    assert!(!ok);
    assert!(stderr.contains("collision"));
}

#[test]
fn collision_off_allows_overlap() {
    let (_, _, ok) = run_stdin(
        "canvas 20 5\n\
         collision off\n\
         box 0 0 5 1\n\
         box 3 0 5 1",
    );
    assert!(ok);
}

#[test]
fn collision_error_format() {
    let (_, stderr, ok) = run_stdin(
        "canvas 20 5\n\
         collision on\n\
         box 0 0 5 1\n\
         box 3 0 5 1",
    );
    assert!(!ok);
    // Error format: "collision: object #N (...) overlaps object #M (...) at (...) size ..."
    assert!(stderr.contains("object #2"));
    assert!(stderr.contains("object #1"));
    assert!(stderr.contains("overlaps"));
    assert!(stderr.contains("size"));
}

// ─── Content Overflow ────────────────────────────────────────────────

#[test]
fn overflow_ellipsis() {
    let (stdout, _, ok) = run_stdin(
        "canvas 10 3\n\
         collision off\n\
         box 0 0 4 1 c=VeryLongText",
    );
    assert!(ok);
    assert!(stdout.contains("..12"));
}

#[test]
fn overflow_hidden() {
    let (stdout, _, ok) = run_stdin(
        "canvas 10 3\n\
         collision off\n\
         box 0 0 5 1 overflow=hidden c=HelloWorld",
    );
    assert!(ok);
    assert!(stdout.contains("Hello"));
    assert!(!stdout.contains("World"));
}

#[test]
fn overflow_error() {
    let (_, stderr, ok) = run_stdin(
        "canvas 10 3\n\
         collision off\n\
         box 0 0 3 1 overflow=error c=VeryLong",
    );
    assert!(!ok);
    assert!(stderr.contains("overflow"));
}

// ─── Content Alignment ──────────────────────────────────────────────

#[test]
fn align_center() {
    let (stdout, _, ok) = run_stdin(
        "canvas 12 3\n\
         collision off\n\
         box 0 0 10 1 align=c c=Hi",
    );
    assert!(ok);
    // "Hi" (2 cols) centered in 10 cols → pad 4 left
    assert!(stdout.contains("│    Hi    │"));
}

#[test]
fn align_right() {
    let (stdout, _, ok) = run_stdin(
        "canvas 12 3\n\
         collision off\n\
         box 0 0 10 1 align=r c=Hi",
    );
    assert!(ok);
    assert!(stdout.contains("│        Hi│"));
}

// ─── Canvas Border ──────────────────────────────────────────────────

#[test]
fn canvas_border_rounded() {
    let (stdout, _, ok) = run_stdin(
        "canvas 10 3 border=r\n\
         collision off",
    );
    assert!(ok);
    assert!(stdout.contains('╭'));
    assert!(stdout.contains('╯'));
}

// ─── List ────────────────────────────────────────────────────────────

#[test]
fn list_subcommand() {
    let (stdout, _, ok) = run_subcmd(
        "list",
        "canvas 30 5\n\
         collision on\n\
         box 0 0 8 1 c=Box\n\
         text 15 1 c=Hi",
    );
    assert!(ok);
    assert!(stdout.contains("Canvas: 30x5"));
    assert!(stdout.contains("Collision: on"));
    assert!(stdout.contains("Objects: 2"));
    assert!(stdout.contains("box"));
    assert!(stdout.contains("text"));
}

#[test]
fn list_auto_canvas() {
    let (stdout, _, ok) = run_subcmd(
        "list",
        "canvas auto\n\
         collision off\n\
         box 0 0 4 1",
    );
    assert!(ok);
    assert!(stdout.contains("(auto)"));
}

// ─── Lint ────────────────────────────────────────────────────────────

#[test]
fn lint_ok() {
    let (stdout, _, ok) = run_subcmd(
        "lint",
        "canvas 10 3\n\
         collision off\n\
         box 0 0 4 1",
    );
    assert!(ok);
    assert!(stdout.contains("OK"));
}

#[test]
fn lint_collision_error() {
    let (stdout, _, ok) = run_subcmd(
        "lint",
        "canvas 10 5\n\
         collision on\n\
         box 0 0 5 1\n\
         box 3 0 5 1",
    );
    assert!(!ok);
    assert!(stdout.contains("Errors:"));
    assert!(stdout.contains("collision"));
}

// ─── Guide ──────────────────────────────────────────────────────────

#[test]
fn guide_subcommand() {
    let output = unid().arg("guide").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("USAGE:"));
    assert!(stdout.contains("DSL SYNTAX:"));
    assert!(stdout.contains("BORDER STYLES"));
}

// ─── Error cases ────────────────────────────────────────────────────

#[test]
fn error_missing_canvas() {
    let (_, stderr, ok) = run_stdin(
        "collision on\n\
         box 0 0 4 1",
    );
    assert!(!ok);
    assert!(stderr.contains("canvas"));
}

#[test]
fn error_missing_collision() {
    let (_, stderr, ok) = run_stdin(
        "canvas 10 5\n\
         box 0 0 4 1",
    );
    assert!(!ok);
    assert!(stderr.contains("collision"));
}

#[test]
fn error_parse_error() {
    let (_, stderr, ok) = run_stdin(
        "canvas 10 5\n\
         collision on\n\
         badcmd 1 2",
    );
    assert!(!ok);
    assert!(stderr.contains("unknown command"));
}

#[test]
fn error_unknown_arrow_id() {
    let (_, stderr, ok) = run_stdin(
        "canvas 20 5\n\
         collision off\n\
         box 0 0 4 1 id=a\n\
         arrow a.r nonexistent.l",
    );
    assert!(!ok);
    assert!(stderr.contains("unknown object id"));
}

#[test]
fn error_invalid_arrow_anchor() {
    let (_, stderr, ok) = run_stdin(
        "canvas 20 5\n\
         collision off\n\
         arrow noid db.top",
    );
    assert!(!ok);
    assert!(stderr.contains("invalid anchor"));
}

// ─── Comments and blank lines ───────────────────────────────────────

#[test]
fn comments_and_blank_lines() {
    let (stdout, _, ok) = run_stdin(
        "# This is a comment\n\
         canvas 6 3\n\
         \n\
         collision off\n\
         # Another comment\n\
         box 0 0 4 1",
    );
    assert!(ok);
    assert_eq!(stdout.trim(), "┌────┐\n│    │\n└────┘");
}

// ─── Text object ────────────────────────────────────────────────────

#[test]
fn render_text_object() {
    let (stdout, _, ok) = run_stdin(
        "canvas 20 3\n\
         collision off\n\
         text 0 0 c=Hello World",
    );
    assert!(ok);
    assert!(stdout.contains("Hello World"));
}

// ─── Backward compatibility ─────────────────────────────────────────

#[test]
fn rect_alias_for_box() {
    // "rect" is accepted as an alias for "box"
    let (stdout, _, ok) = run_stdin(
        "canvas 6 3\n\
         collision off\n\
         rect 0 0 4 1",
    );
    assert!(ok);
    assert_eq!(stdout.trim(), "┌────┐\n│    │\n└────┘");
}

// ─── Shorthand options ──────────────────────────────────────────────

#[test]
fn shorthand_style() {
    let (stdout, _, ok) = run_stdin(
        "canvas 8 3\n\
         collision off\n\
         box 0 0 6 1 style=rounded",
    );
    assert!(ok);
    assert!(stdout.contains('╭'));
}

#[test]
fn content_with_newline_escape() {
    // \n in content is unescaped to a real newline by the parser.
    // Multiline content renders each line on a separate row.
    let (stdout, _, ok) = run_stdin(
        "canvas 12 5\n\
         collision off\n\
         box 0 0 10 3 c=Line1\\nLine2",
    );
    assert!(ok);
    assert!(stdout.contains("Line1"));
    assert!(stdout.contains("Line2"));
}

#[test]
fn multiline_rect_vertical_center() {
    // 2 lines in inner_h=3 → vertically centered
    let (stdout, _, ok) = run_stdin(
        "canvas 10 5\n\
         collision off\n\
         box 0 0 8 3 align=c c=AA\\nBB",
    );
    assert!(ok);
    // inner_h=3, line_count=2, start_row = 0+1+(3-2)/2 = 1+0 = 1
    // But (3-2)/2 = 0, so lines are at rows 1 and 2 (top of inner area)
    // Actually: start_row = row+1 + (3-2)/2 = 1+0 = 1
    assert!(stdout.contains("AA"));
    assert!(stdout.contains("BB"));
}

#[test]
fn multiline_text_object() {
    let (stdout, _, ok) = run_stdin(
        "canvas 10 3\n\
         collision off\n\
         text 0 0 c=Hello\\nWorld",
    );
    assert!(ok);
    assert!(stdout.contains("Hello"));
    assert!(stdout.contains("World"));
}

// ─── Legend ─────────────────────────────────────────────────────────

#[test]
fn rect_legend_top() {
    let (stdout, _, ok) = run_stdin(
        "canvas 14 4\n\
         collision off\n\
         box 0 1 10 1 lg=Title",
    );
    assert!(ok);
    assert!(stdout.contains("Title"));
    assert!(stdout.contains("┌"));
}

#[test]
fn rect_legend_bottom() {
    let (stdout, _, ok) = run_stdin(
        "canvas 14 5\n\
         collision off\n\
         box 0 0 10 1 legend-pos=b lg=Footer",
    );
    assert!(ok);
    assert!(stdout.contains("Footer"));
}

#[test]
fn rect_legend_lr_error() {
    let (_, stderr, ok) = run_stdin(
        "canvas 20 5\n\
         collision off\n\
         box 0 0 10 1 legend-pos=l lg=Bad",
    );
    assert!(!ok);
    assert!(stderr.contains("legend-pos only supports top"));
}

#[test]
fn rect_content_and_legend() {
    let (stdout, _, ok) = run_stdin(
        "canvas 14 4\n\
         collision off\n\
         box 0 1 10 1 c=Content lg=Title",
    );
    assert!(ok);
    assert!(stdout.contains("Content"));
    assert!(stdout.contains("Title"));
}

#[test]
fn hline_legend_top() {
    let (stdout, _, ok) = run_stdin(
        "canvas 15 3\n\
         collision off\n\
         hline 0 1 10 lg=separator",
    );
    assert!(ok);
    assert!(stdout.contains("separator"));
    assert!(stdout.contains("─"));
}

#[test]
fn vline_legend_right() {
    let (stdout, _, ok) = run_stdin(
        "canvas 15 5\n\
         collision off\n\
         vline 0 0 4 lg=axis",
    );
    assert!(ok);
    assert!(stdout.contains("axis"));
    assert!(stdout.contains("│"));
}

#[test]
fn hline_with_id() {
    let (stdout, _, ok) = run_stdin(
        "canvas 15 3\n\
         collision off\n\
         hline 0 1 10 id=sep",
    );
    assert!(ok);
    assert!(stdout.contains("─"));
}

// ─── Arrow from non-rect objects ────────────────────────────────────

#[test]
fn arrow_from_hline() {
    let (stdout, _, ok) = run_stdin(
        "canvas 30 5\n\
         collision off\n\
         hline 0 2 10 id=sep\n\
         box 18 0 6 1 id=b c=B\n\
         arrow sep.r b.l",
    );
    assert!(ok);
    assert!(stdout.contains('→') || stdout.contains('─'));
}

#[test]
fn arrow_from_text() {
    let (stdout, _, ok) = run_stdin(
        "canvas 30 5\n\
         collision off\n\
         text 0 1 id=lbl c=Label\n\
         box 18 0 6 1 id=b c=B\n\
         arrow lbl.r b.l",
    );
    assert!(ok);
    assert!(stdout.contains('→') || stdout.contains('─'));
}

#[test]
fn arrow_from_vline() {
    let (stdout, _, ok) = run_stdin(
        "canvas 20 10\n\
         collision off\n\
         vline 0 0 5 id=axis\n\
         box 10 6 6 1 id=b c=B\n\
         arrow axis.b b.t",
    );
    assert!(ok);
    assert!(stdout.contains('↓') || stdout.contains('│'));
}

// ─── Rect ID ────────────────────────────────────────────────────────

#[test]
fn rect_with_id() {
    let (stdout, _, ok) = run_stdin(
        "canvas 30 5\n\
         collision off\n\
         box 0 0 8 1 id=mybox c=Hello",
    );
    assert!(ok);
    assert!(stdout.contains("Hello"));
}

#[test]
fn duplicate_id_error() {
    let (_, stderr, ok) = run_stdin(
        "canvas 20 5\n\
         collision off\n\
         box 0 0 4 1 id=a\n\
         box 10 0 4 1 id=a",
    );
    assert!(!ok);
    assert!(stderr.contains("duplicate"));
}

// ─── S5: Arrowhead + Bidirectional ──────────────────────────────────

#[test]
fn arrow_custom_head() {
    let (stdout, _, ok) = run_stdin(
        "canvas 30 3\n\
         collision off\n\
         box 0 0 4 1 id=a c=A\n\
         box 20 0 4 1 id=b c=B\n\
         arrow a.r b.l head=▶",
    );
    assert!(ok);
    assert!(stdout.contains('▶'));
    // Should NOT contain default arrowhead
    assert!(!stdout.contains('→'));
}

#[test]
fn arrow_global_arrowhead() {
    let (stdout, _, ok) = run_stdin(
        "canvas 30 3\n\
         collision off\n\
         arrowhead ▶\n\
         box 0 0 4 1 id=a c=A\n\
         box 20 0 4 1 id=b c=B\n\
         arrow a.r b.l",
    );
    assert!(ok);
    assert!(stdout.contains('▶'));
}

#[test]
fn arrow_per_arrow_overrides_global() {
    // global=▶ family, per-arrow=⇒ family. Per-arrow wins → ⇒ (right direction)
    let (stdout, _, ok) = run_stdin(
        "canvas 30 3\n\
         collision off\n\
         arrowhead ▶\n\
         box 0 0 4 1 id=a c=A\n\
         box 20 0 4 1 id=b c=B\n\
         arrow a.r b.l head=⇒",
    );
    assert!(ok);
    assert!(stdout.contains('⇒'), "per-arrow head should override global");
    assert!(!stdout.contains('▶'), "global arrowhead should not appear");
}

#[test]
fn arrow_bidirectional() {
    let (stdout, _, ok) = run_stdin(
        "canvas 30 3\n\
         collision off\n\
         box 0 0 4 1 id=a c=A\n\
         box 20 0 4 1 id=b c=B\n\
         arrow a.r b.l both",
    );
    assert!(ok);
    // Should have arrowhead at both ends: ◀ at destination (b.l) and ▶ at source (a.r reversed)
    assert!(stdout.contains('◀'));
}

#[test]
fn arrow_bidirectional_with_custom_head() {
    // a.r→b.l: horizontal right. head=▶ family: dst=▶ (right), src=◀ (left)
    let (stdout, _, ok) = run_stdin(
        "canvas 30 3\n\
         collision off\n\
         box 0 0 4 1 id=a c=A\n\
         box 20 0 4 1 id=b c=B\n\
         arrow a.r b.l both head=▶",
    );
    assert!(ok);
    assert!(stdout.contains('▶'), "dst should have ▶ (right direction)");
    assert!(stdout.contains('◀'), "src (both) should have ◀ (left direction)");
}

#[test]
fn arrow_invalid_arrowhead_rejected() {
    let (_, stderr, ok) = run_stdin(
        "canvas 30 3\n\
         collision off\n\
         box 0 0 4 1 id=a c=A\n\
         box 20 0 4 1 id=b c=B\n\
         arrow a.r b.l head=◆",
    );
    assert!(!ok);
    assert!(stderr.contains("invalid arrowhead"));
}

#[test]
fn arrow_head_resolves_direction_vertical() {
    // Vertical arrow with head=▶ family: dst should be ▼ (down direction)
    let (stdout, _, ok) = run_stdin(
        "canvas 20 12\n\
         collision off\n\
         box 2 0 6 1 id=a c=A\n\
         box 2 8 6 1 id=b c=B\n\
         arrow a.b b.t head=▶",
    );
    assert!(ok);
    assert!(stdout.contains('▼'), "vertical dst should resolve to ▼");
}

// ─── S6: Arrow Legend ───────────────────────────────────────────────

#[test]
fn arrow_legend_horizontal() {
    let (stdout, _, ok) = run_stdin(
        "canvas 30 5\n\
         collision off\n\
         box 0 1 4 1 id=a c=A\n\
         box 20 1 4 1 id=b c=B\n\
         arrow a.r b.l lg=request",
    );
    assert!(ok);
    assert!(stdout.contains("request"));
}

#[test]
fn arrow_legend_with_pos() {
    let (stdout, _, ok) = run_stdin(
        "canvas 30 5\n\
         collision off\n\
         box 0 1 4 1 id=a c=A\n\
         box 20 1 4 1 id=b c=B\n\
         arrow a.r b.l pos=b lg=data",
    );
    assert!(ok);
    assert!(stdout.contains("data"));
}

#[test]
fn text_overwrites_structure() {
    // Text rendered in pass 2 should overwrite border from pass 1
    let (stdout, _, ok) = run_stdin(
        "canvas 10 3\n\
         collision off\n\
         box 0 0 8 1\n\
         text 0 0 c=X",
    );
    assert!(ok);
    // 'X' should overwrite the top-left corner '┌'
    assert!(stdout.starts_with('X'));
}

#[test]
fn self_loop_right_to_top() {
    let (stdout, _, ok) = run_stdin(
        "canvas 20 8\n\
         collision off\n\
         box 2 2 8 1 id=a c=Loop\n\
         arrow a.r a.t",
    );
    assert!(ok);
    assert!(stdout.contains('▼')); // Arrowhead entering top
    assert!(stdout.contains('┘')); // Corner from horizontal to vertical
    assert!(stdout.contains('┐')); // Corner from vertical to horizontal
}

#[test]
fn self_loop_bottom_to_left() {
    let (stdout, _, ok) = run_stdin(
        "canvas 20 8\n\
         collision off\n\
         box 2 2 8 1 id=a c=Loop\n\
         arrow a.b a.l",
    );
    assert!(ok);
    assert!(stdout.contains('▶')); // Arrowhead entering left
    assert!(stdout.contains('└')); // Corner
}

#[test]
fn arrow_legend_vertical() {
    let (stdout, _, ok) = run_stdin(
        "canvas 20 12\n\
         collision off\n\
         box 2 0 6 1 id=a c=A\n\
         box 2 8 6 1 id=b c=B\n\
         arrow a.b b.t lg=flow",
    );
    assert!(ok);
    assert!(stdout.contains("flow"));
}
