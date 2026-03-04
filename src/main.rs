use std::collections::HashMap;
use std::io::{self, IsTerminal, Read};
use std::process;

use clap::Parser;
use unicode_diagram::canvas::Canvas;
use unicode_diagram::cli::{Cli, CollisionMode, Commands};
use unicode_diagram::dsl::command::{CanvasSize, DslCommand};
use unicode_diagram::dsl::parse;
use unicode_diagram::error::UnidError;
use unicode_diagram::object::arrow::{compute_route, ResolvedArrow};
use unicode_diagram::object::rect::{BorderStyle, ContentAlign, ContentOverflow, Side};
use unicode_diagram::object::{DrawObject, Rect};
use unicode_diagram::renderer::Renderer;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::List) => run_list(),
        Some(Commands::Guide) => {
            print_guide();
            Ok(())
        }
        Some(Commands::Lint) => run_lint(),
        None => {
            // Default: render from stdin
            if io::stdin().is_terminal() {
                eprintln!("warning: no input provided. Use 'echo \"...\" | unid' or 'unid guide' for details.\n");
                Cli::parse_from(["unid", "--help"]);
                Ok(())
            } else {
                run_render(cli.collision)
            }
        }
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn read_stdin() -> Result<String, UnidError> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

/// A slot in the draw order: either a resolved object or a pending arrow.
enum DrawSlot {
    Ready(DrawObject),
    PendingArrow,
}

struct CanvasConfig {
    width: CanvasSize,
    height: CanvasSize,
    border: Option<BorderStyle>,
    global_overflow: ContentOverflow,
    global_align: ContentAlign,
    collision: bool,
    objects: Vec<DrawObject>,
}

fn process_commands(
    commands: Vec<DslCommand>,
    collision_override: Option<CollisionMode>,
) -> Result<CanvasConfig, UnidError> {
    let mut canvas_width = None;
    let mut canvas_height = None;
    let mut border = None;
    let mut global_overflow = ContentOverflow::default();
    let mut global_align = ContentAlign::default();
    let mut collision = None;
    // Slots preserve DSL ordering: arrows start as PendingArrow and get
    // replaced with resolved DrawObjects after all rects are collected.
    let mut slots: Vec<DrawSlot> = Vec::new();
    let mut arrow_slots: Vec<(usize, String, Side, String, Side, usize)> = Vec::new();

    for cmd in commands {
        match cmd {
            DslCommand::Canvas {
                width,
                height,
                border: b,
                content_overflow,
                content_align,
            } => {
                canvas_width = Some(width);
                canvas_height = Some(height);
                border = b;
                if let Some(co) = content_overflow {
                    global_overflow = co;
                }
                if let Some(ca) = content_align {
                    global_align = ca;
                }
            }
            DslCommand::Collision(v) => {
                collision = Some(v);
            }
            DslCommand::Object(obj) => {
                slots.push(DrawSlot::Ready(obj));
            }
            DslCommand::Arrow {
                src_id,
                src_side,
                dst_id,
                dst_side,
                line,
            } => {
                let idx = slots.len();
                slots.push(DrawSlot::PendingArrow);
                arrow_slots.push((idx, src_id, src_side, dst_id, dst_side, line));
            }
        }
    }

    let cw = canvas_width.ok_or(UnidError::NoCanvas)?;
    let ch = canvas_height.ok_or(UnidError::NoCanvas)?;
    let coll_dsl = collision.ok_or(UnidError::NoCollision)?;

    let coll = match collision_override {
        Some(CollisionMode::On) => true,
        Some(CollisionMode::Off) => false,
        None => coll_dsl,
    };

    // Resolve arrows and fill PendingArrow slots
    resolve_arrows_into_slots(&mut slots, &arrow_slots)?;

    let objects: Vec<DrawObject> = slots
        .into_iter()
        .filter_map(|slot| match slot {
            DrawSlot::Ready(obj) => Some(obj),
            DrawSlot::PendingArrow => None, // Should not happen after resolution
        })
        .collect();

    Ok(CanvasConfig {
        width: cw,
        height: ch,
        border,
        global_overflow,
        global_align,
        collision: coll,
        objects,
    })
}

/// Resolves unresolved arrows and fills their reserved slots.
fn resolve_arrows_into_slots(
    slots: &mut [DrawSlot],
    arrow_slots: &[(usize, String, Side, String, Side, usize)],
) -> Result<(), UnidError> {
    // Phase 1: Build ID → Rect data mapping (clone to avoid borrow conflicts)
    let mut id_anchors: HashMap<String, Rect> = HashMap::new();
    for slot in slots.iter() {
        if let DrawSlot::Ready(DrawObject::Rect(r)) = slot
            && let Some(id) = &r.id
        {
            if id_anchors.contains_key(id) {
                return Err(UnidError::Parse {
                    line: 0,
                    message: format!("duplicate object id '{}'", id),
                });
            }
            id_anchors.insert(id.clone(), r.clone());
        }
    }

    // Phase 2: Resolve each arrow and replace PendingArrow slots
    for (idx, src_id, src_side, dst_id, dst_side, line) in arrow_slots {
        let src_rect = id_anchors.get(src_id).ok_or_else(|| UnidError::Parse {
            line: *line,
            message: format!("unknown object id '{}' in arrow source", src_id),
        })?;
        let dst_rect = id_anchors.get(dst_id).ok_or_else(|| UnidError::Parse {
            line: *line,
            message: format!("unknown object id '{}' in arrow destination", dst_id),
        })?;

        let (sx, sy) = src_rect.src_anchor(*src_side);
        let (ex, ey) = dst_rect.dst_anchor(*dst_side);
        let waypoints = compute_route(sx, sy, *src_side, ex, ey, *dst_side);

        slots[*idx] = DrawSlot::Ready(DrawObject::Arrow(ResolvedArrow { waypoints }));
    }
    Ok(())
}

fn compute_canvas_size(
    width: CanvasSize,
    height: CanvasSize,
    objects: &[DrawObject],
    border: Option<BorderStyle>,
) -> (usize, usize) {
    let (mut w, mut h) = match (width, height) {
        (CanvasSize::Fixed(w), CanvasSize::Fixed(h)) => (w, h),
        _ => {
            let (mut max_w, mut max_h) = (1, 1);
            for obj in objects {
                let (bw, bh) = obj.bounds();
                max_w = max_w.max(bw);
                max_h = max_h.max(bh);
            }
            let w = if let CanvasSize::Fixed(fw) = width {
                fw
            } else {
                max_w
            };
            let h = if let CanvasSize::Fixed(fh) = height {
                fh
            } else {
                max_h
            };
            (w, h)
        }
    };

    // Add border space if needed
    if border.is_some() {
        // Canvas size includes border, so objects are placed at offset (1,1)
        // But we need to ensure the canvas is at least 3x3 for the border
        w = w.max(3);
        h = h.max(3);
    }

    (w, h)
}

fn run_render(collision_override: Option<CollisionMode>) -> Result<(), UnidError> {
    let input = read_stdin()?;
    let commands = parse(&input)?;
    let config = process_commands(commands, collision_override)?;
    let (width, height) = compute_canvas_size(config.width, config.height, &config.objects, config.border);

    let canvas = Canvas::new(width, height);
    let mut renderer = Renderer::new(canvas, config.collision);
    renderer.global_overflow = config.global_overflow;
    renderer.global_align = config.global_align;

    // Draw border first if specified
    if let Some(border_style) = config.border {
        renderer.draw_border(border_style)?;
    }

    // Apply global defaults to rects that don't have explicit settings
    let objects: Vec<DrawObject> = config
        .objects
        .into_iter()
        .map(|obj| match obj {
            DrawObject::Rect(mut r) => {
                if r.content_overflow == ContentOverflow::default() {
                    r.content_overflow = config.global_overflow;
                }
                if r.content_align == ContentAlign::default() {
                    r.content_align = config.global_align;
                }
                DrawObject::Rect(r)
            }
            other => other,
        })
        .collect();

    renderer.draw_all(&objects)?;
    println!("{}", renderer.render());
    Ok(())
}

fn run_list() -> Result<(), UnidError> {
    let input = read_stdin()?;
    let commands = parse(&input)?;
    let config = process_commands(commands, None)?;
    let (width, height) = compute_canvas_size(config.width, config.height, &config.objects, config.border);

    let auto_label = match (config.width, config.height) {
        (CanvasSize::Auto, CanvasSize::Auto) => " (auto)",
        _ => "",
    };

    println!("Canvas: {}x{}{}", width, height, auto_label);
    println!("Collision: {}", if config.collision { "on" } else { "off" });
    if let Some(b) = config.border {
        println!("Border: {:?}", b);
    }
    println!("Objects: {}", config.objects.len());

    let mut objects = config.objects;
    objects.sort_by(|a, b| {
        let (ac, ar) = a.position();
        let (bc, br) = b.position();
        (ar, ac).cmp(&(br, bc))
    });

    for (i, obj) in objects.iter().enumerate() {
        println!("  {}. {}", i + 1, obj.summary());
    }

    Ok(())
}

fn run_lint() -> Result<(), UnidError> {
    let input = read_stdin()?;
    let commands = parse(&input)?;
    let config = process_commands(commands, None)?;
    let (width, height) = compute_canvas_size(config.width, config.height, &config.objects, config.border);

    println!("Canvas: {}x{}", width, height);
    println!("Collision: {}", if config.collision { "on" } else { "off" });
    println!("Objects: {}", config.objects.len());

    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    for (i, obj) in config.objects.iter().enumerate() {
        let (bw, bh) = obj.bounds();
        if bw > width || bh > height {
            let msg = format!(
                "object #{} ({}): bounds ({}x{}) exceed canvas ({}x{})",
                i + 1,
                obj.collision_desc(),
                bw,
                bh,
                width,
                height,
            );
            if config.collision {
                errors.push(msg);
            } else {
                warnings.push(msg);
            }
        }
    }

    // Simulate collision detection
    if config.collision {
        let canvas = Canvas::new(width, height);
        let mut renderer = Renderer::new(canvas, true);
        if let Some(border_style) = config.border {
            let _ = renderer.draw_border(border_style);
        }
        for obj in &config.objects {
            if let Err(e) = renderer.draw(obj) {
                errors.push(e.to_string());
            }
        }
    }

    if !warnings.is_empty() {
        println!("Warnings:");
        for w in &warnings {
            println!("  - {}", w);
        }
    }

    if !errors.is_empty() {
        println!("Errors:");
        for e in &errors {
            println!("  - {}", e);
        }
        process::exit(1);
    }

    if warnings.is_empty() && errors.is_empty() {
        println!("OK");
    }

    Ok(())
}

fn print_guide() {
    print!(
        r#"unid - Unicode Diagram Renderer

USAGE:
  echo "..." | unid                    Render from stdin (default)
  echo "..." | unid --collision=off    Override collision mode
  echo "..." | unid list               List objects in diagram
  echo "..." | unid lint               Lint DSL for errors/warnings
  unid guide                           Show this guide

DSL SYNTAX:
  Lines starting with # are comments. Blank lines are ignored.
  Commands are case-insensitive. Each command is on its own line.

  HEADER (required, must appear before objects):
    canvas <width> <height> [border=<style>] [overflow=<mode>] [align=<align>]
    canvas auto [border=<style>]
    collision on|off

  OBJECTS:
    rect <col> <row> <w> <h> [id=<name>] [s=<style>] [overflow=<mode>] [align=<align>] [c=<content>]
    text <col> <row> c=<content>
    hline <col> <row> <length> [s=<style>]
    vline <col> <row> <length> [s=<style>]
    arrow <src_id>.<side> <dst_id>.<side>

  SHORTHAND:
    s=  → style=       c=  → content=
    Style values: l=light h=heavy d=double r=rounded
    Overflow values: ellipsis|overflow|hidden|error
    Align values: l=left c=center r=right
    Side values: t=top r=right b=bottom l=left

BORDER STYLES:
  light/l (default):  ┌─┐ │ └─┘
  heavy/h:            ┏━┓ ┃ ┗━┛
  double/d:           ╔═╗ ║ ╚═╝
  rounded/r:          ╭─╮ │ ╰─╯

LINE STYLES:
  light/l (default):  ─ │
  heavy/h:            ━ ┃
  double/d:           ═ ║
  dash:               ╌ ╎

CONTENT OVERFLOW MODES:
  ellipsis (default): Truncate with "prefix..{{N}}" where N=truncated display width
  overflow:           Content overwrites borders
  hidden:             Truncate without indicator
  error:              Return error if content exceeds width

CONTENT ALIGNMENT:
  left/l (default):   Left-aligned (right side truncated/overflows)
  center/c:           Center-aligned (both sides truncated/overflow)
  right/r:            Right-aligned (left side truncated/overflows)

ARROWS:
  Arrows connect objects by id and side (anchor point).
  Routing is automatic based on source/destination sides.

  Syntax: arrow <src_id>.<side> <dst_id>.<side>

  Route types (auto-selected):
    Straight:     ──→        (opposite sides, aligned)
    L-shaped:     ──┐        (perpendicular sides, favorable)
                    ↓
    Z-shaped:     ──┐        (same direction, not aligned)
                    └──→
    U-shaped:     ──┐        (opposite sides, same axis — ㄷ shape)
                    │
                  ←─┘

  Source anchor: 1 cell outside border (arrow starts here)
  Dest anchor:   on border (arrowhead replaces border character)

EXAMPLE:
  input:
    echo "canvas 52 27 border=r
    collision off
    # Boxes: 4 styles + alignment + id
    rect 2 1 16 1 id=api s=d align=c c=API Gateway
    rect 28 1 12 1 id=web align=c c=Web Client
    rect 2 9 16 1 id=auth s=r c=Auth 인증
    rect 28 9 12 1 id=db s=h align=r c=Data Store
    # Anchor-based arrows (auto-routed)
    arrow api.r web.l
    arrow api.b auth.t
    text 12 5 c=req
    arrow web.b db.t
    text 36 5 c=query
    arrow auth.r db.l
    text 21 10 c=sync
    # Separator + overflow demos
    hline 2 17 48 s=dash
    text 2 19 c=ellipsis:
    rect 12 19 10 1 c=LongServiceName
    text 26 19 c=overflow:
    rect 36 19 10 1 overflow=overflow c=LongServiceName
    text 2 22 c=hidden:
    rect 12 22 10 1 overflow=hidden c=LongServiceName" | unid

  output:
    ╭──────────────────────────────────────────────────╮
    │ ╔════════════════╗        ┌────────────┐         │
    │ ║  API Gateway   ║────────→ Web Client │         │
    │ ╚════════════════╝        └────────────┘         │
    │          │                       │               │
    │          │req                    │query          │
    │          │                       │               │
    │          │                       │               │
    │          │                       │               │
    │ ╭────────↓───────╮        ┏━━━━━━↓━━━━━┓         │
    │ │Auth 인증       │─sync───→  Data Store┃         │
    │ ╰────────────────╯        ┗━━━━━━━━━━━━┛         │
    │                                                  │
    │                                                  │
    │                                                  │
    │                                                  │
    │                                                  │
    │ ╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌ │
    │                                                  │
    │ ellipsis: ┌──────────┐  overflow: ┌──────────┐   │
    │           │LongSer..8│            │LongServiceName
    │           └──────────┘            └──────────┘   │
    │ hidden:   ┌──────────┐                           │
    │           │LongServic│                           │
    │           └──────────┘                           │
    │                                                  │
    ╰──────────────────────────────────────────────────╯

NOTES:
  - --collision CLI flag overrides DSL collision declaration
  - Canvas auto computes minimum size from all object bounds
  - CJK characters (한글, 漢字, かな) take 2 display columns
  - content= (c=) must be the last option on a line
  - Use \n in content for literal newlines
  - Canvas border is included in the specified size (e.g., 20x5 with border → 18x3 inner)
  - id= names: alphanumeric, underscore, hyphen only
  - Arrow routing is fully automatic based on anchor sides
"#
    );
}
