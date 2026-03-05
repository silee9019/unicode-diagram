use std::collections::HashMap;
use std::io::{self, IsTerminal, Read};
use std::process;

use clap::Parser;
use unicode_diagram::canvas::Canvas;
use unicode_diagram::cli::{Cli, CollisionMode, Commands};
use unicode_diagram::dsl::command::{CanvasSize, DslCommand};
use unicode_diagram::dsl::parse;
use unicode_diagram::error::UnidError;
use unicode_diagram::object::arrow::{compute_route, compute_self_loop, ResolvedArrow};
use unicode_diagram::object::rect::{BorderStyle, ContentAlign, ContentOverflow, Side};
use unicode_diagram::object::{DrawObject, HLine, Legend, Rect, Text, VLine};
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

/// Unresolved arrow reference stored while objects are being collected.
struct PendingArrowSlot {
    slot_idx: usize,
    src_id: String,
    src_side: Side,
    dst_id: String,
    dst_side: Side,
    head: Option<char>,
    both: bool,
    legend: Option<Legend>,
    line: usize,
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
    let mut arrow_slots: Vec<PendingArrowSlot> = Vec::new();
    let mut global_arrowhead: Option<char> = None;

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
                head,
                both,
                legend,
                line,
            } => {
                let idx = slots.len();
                slots.push(DrawSlot::PendingArrow);
                arrow_slots.push(PendingArrowSlot {
                    slot_idx: idx,
                    src_id,
                    src_side,
                    dst_id,
                    dst_side,
                    head,
                    both,
                    legend,
                    line,
                });
            }
            DslCommand::Arrowhead(ch) => {
                global_arrowhead = Some(ch);
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
    resolve_arrows_into_slots(&mut slots, &arrow_slots, global_arrowhead)?;

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

/// Anchor source: any object type that can be an arrow endpoint.
#[derive(Clone)]
enum AnchorSource {
    Rect(Rect),
    Text(Text),
    HLine(HLine),
    VLine(VLine),
}

impl AnchorSource {
    fn src_anchor(&self, side: Side) -> (usize, usize) {
        match self {
            AnchorSource::Rect(r) => r.src_anchor(side),
            AnchorSource::Text(t) => t.src_anchor(side),
            AnchorSource::HLine(h) => h.src_anchor(side),
            AnchorSource::VLine(v) => v.src_anchor(side),
        }
    }

    fn dst_anchor(&self, side: Side) -> (usize, usize) {
        match self {
            AnchorSource::Rect(r) => r.dst_anchor(side),
            AnchorSource::Text(t) => t.dst_anchor(side),
            AnchorSource::HLine(h) => h.dst_anchor(side),
            AnchorSource::VLine(v) => v.dst_anchor(side),
        }
    }
}

/// Resolves unresolved arrows and fills their reserved slots.
fn resolve_arrows_into_slots(
    slots: &mut [DrawSlot],
    arrow_slots: &[PendingArrowSlot],
    global_arrowhead: Option<char>,
) -> Result<(), UnidError> {
    // Phase 1: Build ID → AnchorSource mapping from all object types
    let mut id_anchors: HashMap<String, AnchorSource> = HashMap::new();
    for slot in slots.iter() {
        let (id_opt, source) = match slot {
            DrawSlot::Ready(DrawObject::Rect(r)) => {
                (r.id.as_ref(), AnchorSource::Rect(r.clone()))
            }
            DrawSlot::Ready(DrawObject::Text(t)) => {
                (t.id.as_ref(), AnchorSource::Text(t.clone()))
            }
            DrawSlot::Ready(DrawObject::HLine(h)) => {
                (h.id.as_ref(), AnchorSource::HLine(h.clone()))
            }
            DrawSlot::Ready(DrawObject::VLine(v)) => {
                (v.id.as_ref(), AnchorSource::VLine(v.clone()))
            }
            _ => continue,
        };
        if let Some(id) = id_opt {
            if id_anchors.contains_key(id) {
                return Err(UnidError::Parse {
                    line: 0,
                    message: format!("duplicate object id '{}'", id),
                });
            }
            id_anchors.insert(id.clone(), source);
        }
    }

    // Phase 2: Resolve each arrow and replace PendingArrow slots
    for slot in arrow_slots {
        let src = id_anchors.get(&slot.src_id).ok_or_else(|| UnidError::Parse {
            line: slot.line,
            message: format!("unknown object id '{}' in arrow source", slot.src_id),
        })?;
        let dst = id_anchors.get(&slot.dst_id).ok_or_else(|| UnidError::Parse {
            line: slot.line,
            message: format!("unknown object id '{}' in arrow destination", slot.dst_id),
        })?;

        let (sx, sy) = src.src_anchor(slot.src_side);
        let (ex, ey) = dst.dst_anchor(slot.dst_side);
        let waypoints = if slot.src_id == slot.dst_id {
            compute_self_loop(sx, sy, slot.src_side, ex, ey, slot.dst_side)
        } else {
            compute_route(sx, sy, slot.src_side, ex, ey, slot.dst_side)
        };

        // Resolve effective arrowhead: per-arrow > global > default
        let effective_head = slot.head.or(global_arrowhead);

        slots[slot.slot_idx] = DrawSlot::Ready(DrawObject::Arrow(ResolvedArrow {
            waypoints,
            head: effective_head,
            both: slot.both,
            legend: slot.legend.clone(),
        }));
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
    rect <col> <row> <w> <h> [id=<name>] [s=<style>] [overflow=<mode>] [align=<align>]
         [lg-pos=<top|bottom>] [lg=<legend>] [c=<content>]
    text <col> <row> [id=<name>] c=<content>
    hline <col> <row> <length> [id=<name>] [s=<style>] [pos=<pos>] [lg=<legend>]
    vline <col> <row> <length> [id=<name>] [s=<style>] [pos=<pos>] [lg=<legend>]
    arrow <src_id>.<side> <dst_id>.<side> [head=<char>] [both] [pos=<pos>] [lg=<legend>]
    arrowhead <char>

  SHORTHAND:
    s=  → style=       c=  → content=       lg= → legend=
    Style values: l=light h=heavy d=double r=rounded
    Overflow values: ellipsis|overflow|hidden|error
    Align values: l=left c=center r=right
    Side values: t=top r=right b=bottom l=left
    Position values: t=top r=right b=bottom l=left a=auto

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

CONTENT & LEGEND:
  c= (content=)       Text inside the object (rect inner area, text object)
  lg= (legend=)       Text outside/near the object (rect, hline, vline, arrow)
  Use \n for multiline text in both c= and lg= values
  Leading/trailing whitespace per line is trimmed automatically

CONTENT OVERFLOW MODES:
  ellipsis (default): Truncate with "prefix..{{N}}" where N=truncated display width
  overflow:           Content overwrites borders
  hidden:             Truncate without indicator
  error:              Return error if content exceeds width

CONTENT ALIGNMENT:
  left/l (default):   Left-aligned (right side truncated/overflows)
  center/c:           Center-aligned (both sides truncated/overflow)
  right/r:            Right-aligned (left side truncated/overflows)

LEGEND POSITION:
  Rect:     lg-pos=top(t)|bottom(b)    (default: top)
  HLine:    pos=top(t)|bottom(b)|...   (default: top)
  VLine:    pos=left(l)|right(r)|...   (default: right)
  Arrow:    pos=top(t)|bottom(b)|...   (default: auto — horizontal=top, vertical=right)

ARROWS:
  Arrows connect objects by id and side (anchor point).
  Any object with id= can be an arrow endpoint (rect, text, hline, vline).
  Routing is automatic based on source/destination sides.

  Syntax: arrow <src_id>.<side> <dst_id>.<side> [head=<char>] [both] [lg=<text>]

  Options:
    head=<char>       Arrowhead family (auto-resolves direction). Valid families:
                      →←↑↓ (default)  ▶◀▲▼  ▷◁△▽  ⇒⇐⇑⇓
    both              Bidirectional arrow (arrowhead on both ends)
    lg=<text>         Legend text near the arrow
    arrowhead <char>  Global arrowhead family (separate command)

  Arrowhead priority: per-arrow head= > global arrowhead > default (→←↑↓)
  Direction auto-resolved: head=▶ on a vertical arrow renders as ▼ or ▲

  Route types (auto-selected):
    Straight:     ──→        (opposite sides, aligned)
    L-shaped:     ──┐        (perpendicular sides, favorable)
                    ↓
    Z-shaped:     ──┐        (same direction, not aligned)
                    └──→
    U-shaped:     ──┐        (opposite sides, same axis — ㄷ shape)
                    │
                  ←─┘
    Self-loop:    ──┐        (same object, different sides)
                    └──↓

  Source anchor: 1 cell outside border (arrow starts here)
  Dest anchor:   1 cell outside border (arrowhead does not overwrite border)

RENDERING:
  2-pass rendering: structure first (borders, lines), then text (c=, lg=, text).
  Text content always renders on top of structural elements.

EXAMPLE:
  input:
    echo "canvas 52 27 border=r
    collision off
    # Boxes with legend and arrow labels
    rect 2 2 16 1 id=api s=d align=c lg=Server c=API Gateway
    rect 28 2 12 1 id=web align=c c=Web Client
    rect 2 10 16 1 id=auth s=r c=Auth 인증
    rect 28 10 12 1 id=db s=h align=r c=Data Store
    # Arrows with legend labels and custom head
    arrow api.r web.l both head=▶ lg=HTTP
    arrow api.b auth.t lg=verify
    arrow web.b db.t lg=query
    arrow auth.r db.l lg=sync
    # Self-loop on db
    arrow db.r db.b pos=r lg=backup
    # Separator + overflow demos
    hline 2 18 48 s=dash lg=Features
    text 2 20 c=ellipsis:
    rect 12 20 10 1 c=LongServiceName
    text 26 20 c=overflow:
    rect 36 20 10 1 overflow=overflow c=LongServiceName
    text 2 23 c=hidden:
    rect 12 23 10 1 overflow=hidden c=LongServiceName" | unid

  output:
    ╭──────────────────────────────────────────────────╮
    │ Server                                           │
    │ ╔════════════════╗ HTTP   ┌────────────┐         │
    │ ║  API Gateway   ║◀──────▶│ Web Client │         │
    │ ╚════════════════╝        └────────────┘         │
    │          │                       │               │
    │          │                       │               │
    │          │verify                 │query          │
    │          │                       │               │
    │          ↓                       ↓               │
    │ ╭────────────────╮ sync   ┏━━━━━━━━━━━━┓         │
    │ │Auth 인증       │───────→┃  Data Store┃──┐      │
    │ ╰────────────────╯        ┗━━━━━━━━━━━━┛  │      │
    │                                  ↑        │      │
    │                                  │        │      │
    │                                  └────backup     │
    │                                                  │
    │ Features                                         │
    │ ╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌ │
    │                                                  │
    │ ellipsis: ┌──────────┐  overflow: ┌──────────┐   │
    │           │LongSer..8│            │LongServiceName
    │           └──────────┘            └──────────┘   │
    │ hidden:   ┌──────────┐                           │
    │           │LongServic│                           │
    │           └──────────┘                           │
    ╰──────────────────────────────────────────────────╯

NOTES:
  - --collision CLI flag overrides DSL collision declaration
  - Canvas auto computes minimum size from all object bounds
  - CJK characters (한글, 漢字, かな) take 2 display columns
  - content= (c=) and lg= (legend=) must be the last options on a line
  - Use \n in content/legend for literal newlines (leading/trailing spaces trimmed)
  - Canvas border is included in the specified size (e.g., 20x5 with border → 18x3 inner)
  - id= names: alphanumeric, underscore, hyphen only
  - Arrow routing is fully automatic based on anchor sides
  - Self-loop arrows (same src and dst id) use dedicated ㄷ-shape routing
"#
    );
}
