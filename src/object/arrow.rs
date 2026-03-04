use crate::object::rect::Side;

/// Direction of travel along an arrow segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Right,
    Left,
    Up,
    Down,
}

/// Unresolved arrow parsed from DSL (references object IDs).
#[derive(Debug, Clone)]
pub struct Arrow {
    pub src_id: String,
    pub src_side: Side,
    pub dst_id: String,
    pub dst_side: Side,
}

/// Resolved arrow with computed waypoints (ready for rendering).
#[derive(Debug, Clone)]
pub struct ResolvedArrow {
    /// Waypoints from source to destination (2..=5 points).
    /// First point = source anchor (outside border).
    /// Last point = destination anchor (on border).
    pub waypoints: Vec<(usize, usize)>,
    /// Custom arrowhead character (overrides global/default).
    pub head: Option<char>,
    /// Bidirectional mode: arrowhead on both ends.
    pub both: bool,
    /// Legend text displayed near the arrow.
    pub legend: Option<super::Legend>,
}

/// Converts a Side to the outgoing direction from that side.
fn side_to_outgoing_dir(side: Side) -> Dir {
    match side {
        Side::Top => Dir::Up,
        Side::Right => Dir::Right,
        Side::Bottom => Dir::Down,
        Side::Left => Dir::Left,
    }
}

/// Converts a Side to the incoming direction toward that side.
fn side_to_incoming_dir(side: Side) -> Dir {
    match side {
        Side::Top => Dir::Down,
        Side::Right => Dir::Left,
        Side::Bottom => Dir::Up,
        Side::Left => Dir::Right,
    }
}

/// Default gap for self-loop and short-distance routing.
const ROUTING_GAP: usize = 2;

/// Computes adaptive gap based on Manhattan distance between endpoints.
/// Longer distances get larger gaps to reduce crossing with other elements.
fn adaptive_gap(sx: usize, sy: usize, ex: usize, ey: usize) -> usize {
    let dist = sx.abs_diff(ex) + sy.abs_diff(ey);
    match dist {
        0..=10 => 2,
        11..=25 => 3,
        _ => 4,
    }
}

/// Computes the route waypoints between two anchor points.
///
/// - `(sx, sy)`: source anchor (1 cell outside source border)
/// - `src_side`: which side of the source rect
/// - `(ex, ey)`: destination anchor (on destination border)
/// - `dst_side`: which side of the destination rect
///
/// Returns waypoints including start and end points.
pub fn compute_route(
    sx: usize,
    sy: usize,
    src_side: Side,
    ex: usize,
    ey: usize,
    dst_side: Side,
) -> Vec<(usize, usize)> {
    let src_dir = side_to_outgoing_dir(src_side);
    let dst_dir = side_to_incoming_dir(dst_side);

    // Check if directions are perpendicular or parallel
    let src_horizontal = matches!(src_dir, Dir::Left | Dir::Right);
    let dst_horizontal = matches!(dst_dir, Dir::Left | Dir::Right);
    let perpendicular = src_horizontal != dst_horizontal;

    if perpendicular {
        // Perpendicular: try 1-bend (L-shape) or 3-bend
        route_perpendicular(sx, sy, src_dir, ex, ey, dst_dir)
    } else {
        // Parallel: try 0-bend (straight) or 2-bend (Z/U/ㄷ)
        route_parallel(sx, sy, src_dir, ex, ey, dst_dir)
    }
}

/// Routes when src and dst directions are perpendicular (1 or 3 bends).
fn route_perpendicular(
    sx: usize,
    sy: usize,
    src_dir: Dir,
    ex: usize,
    ey: usize,
    dst_dir: Dir,
) -> Vec<(usize, usize)> {
    // Try L-shape: 1 bend at the intersection of src's travel axis and dst's travel axis
    let bend = match (src_dir, dst_dir) {
        // src goes horizontal, dst goes vertical
        (Dir::Right | Dir::Left, Dir::Down | Dir::Up) => (ex, sy),
        // src goes vertical, dst goes horizontal
        (Dir::Down | Dir::Up, Dir::Right | Dir::Left) => (sx, ey),
        _ => unreachable!(),
    };

    // Check if the L-shape is "favorable" (directions align with target positions)
    if is_favorable_bend(sx, sy, src_dir, bend.0, bend.1)
        && is_favorable_bend(bend.0, bend.1, dst_dir, ex, ey)
    {
        return vec![(sx, sy), bend, (ex, ey)];
    }

    // 3-bend: need intermediate waypoints
    route_3bend(sx, sy, src_dir, ex, ey, dst_dir)
}

/// Routes when src and dst directions are parallel (0 or 2 bends).
fn route_parallel(
    sx: usize,
    sy: usize,
    src_dir: Dir,
    ex: usize,
    ey: usize,
    dst_dir: Dir,
) -> Vec<(usize, usize)> {
    let horizontal = matches!(src_dir, Dir::Left | Dir::Right);
    let same_direction = src_dir == dst_dir;

    if same_direction {
        // Same direction = natural flow: try straight, then Z-shape
        let is_straight = if horizontal {
            sy == ey
                && ((src_dir == Dir::Right && ex >= sx) || (src_dir == Dir::Left && ex <= sx))
        } else {
            sx == ex
                && ((src_dir == Dir::Down && ey >= sy) || (src_dir == Dir::Up && ey <= sy))
        };
        if is_straight {
            return vec![(sx, sy), (ex, ey)];
        }
    }

    // 2-bend (Z or ㄷ shape)
    // same_direction but not aligned → Z-shape (midpoint between)
    // opposite directions → ㄷ-shape (detour past both endpoints)
    route_2bend(sx, sy, src_dir, ex, ey, dst_dir, horizontal, same_direction)
}

/// Checks if traveling from (fx,fy) in direction `dir` can reach (tx,ty).
fn is_favorable_bend(fx: usize, fy: usize, dir: Dir, tx: usize, ty: usize) -> bool {
    match dir {
        Dir::Right => tx >= fx,
        Dir::Left => tx <= fx,
        Dir::Down => ty >= fy,
        Dir::Up => ty <= fy,
    }
}

/// 2-bend routing for parallel directions.
#[allow(clippy::too_many_arguments)]
fn route_2bend(
    sx: usize,
    sy: usize,
    src_dir: Dir,
    ex: usize,
    ey: usize,
    _dst_dir: Dir,
    horizontal: bool,
    same_direction: bool,
) -> Vec<(usize, usize)> {
    if horizontal {
        if same_direction {
            let mid_x = midpoint(sx, ex);
            vec![(sx, sy), (mid_x, sy), (mid_x, ey), (ex, ey)]
        } else {
            let gap = adaptive_gap(sx, sy, ex, ey);
            let mid_x = match src_dir {
                Dir::Right => sx.max(ex) + gap,
                Dir::Left => sx.min(ex).saturating_sub(gap),
                _ => unreachable!(),
            };
            vec![(sx, sy), (mid_x, sy), (mid_x, ey), (ex, ey)]
        }
    } else if same_direction {
        let mid_y = midpoint(sy, ey);
        vec![(sx, sy), (sx, mid_y), (ex, mid_y), (ex, ey)]
    } else {
        let gap = adaptive_gap(sx, sy, ex, ey);
        let mid_y = match src_dir {
            Dir::Down => sy.max(ey) + gap,
            Dir::Up => sy.min(ey).saturating_sub(gap),
            _ => unreachable!(),
        };
        vec![(sx, sy), (sx, mid_y), (ex, mid_y), (ex, ey)]
    }
}

/// 3-bend routing for perpendicular directions that can't form a simple L.
fn route_3bend(
    sx: usize,
    sy: usize,
    src_dir: Dir,
    ex: usize,
    ey: usize,
    _dst_dir: Dir,
) -> Vec<(usize, usize)> {
    // Create a detour: extend src direction, then perpendicular, then into dst
    let horizontal_src = matches!(src_dir, Dir::Left | Dir::Right);

    if horizontal_src {
        // src goes horizontal, dst goes vertical (but unfavorable L)
        // Extend horizontal to midpoint x, then vertical to ey, then horizontal to ex
        let mid_x = midpoint(sx, ex);
        vec![(sx, sy), (mid_x, sy), (mid_x, ey), (ex, ey)]
    } else {
        // src goes vertical, dst goes horizontal (but unfavorable L)
        let mid_y = midpoint(sy, ey);
        vec![(sx, sy), (sx, mid_y), (ex, mid_y), (ex, ey)]
    }
}

/// Simple midpoint calculation.
fn midpoint(a: usize, b: usize) -> usize {
    (a + b) / 2
}

/// Computes a self-loop route (src and dst on the same object).
/// Always routes *outside* the object using a ㄷ-shape detour.
pub fn compute_self_loop(
    sx: usize,
    sy: usize,
    src_side: Side,
    ex: usize,
    ey: usize,
    dst_side: Side,
) -> Vec<(usize, usize)> {
    let gap = ROUTING_GAP;
    let src_dir = side_to_outgoing_dir(src_side);
    // For self-loop, extend dst clearance point *away* from the object (outgoing direction)
    let dst_away = side_to_outgoing_dir(dst_side);

    // Extend from src anchor in src_dir by gap, then route to dst approach, then to dst
    let (p1c, p1r) = extend_point(sx, sy, src_dir, gap);
    let (p2c, p2r) = extend_point(ex, ey, dst_away, gap);

    // If p1 and p2 share an axis, we can do a simple 2-bend
    if p1c == p2c || p1r == p2r {
        vec![(sx, sy), (p1c, p1r), (p2c, p2r), (ex, ey)]
    } else {
        // 3-bend: connect via an intermediate segment
        // Choose the corner path based on src/dst directions
        let src_horizontal = matches!(src_dir, Dir::Left | Dir::Right);
        if src_horizontal {
            // p1 is on horizontal extension, p2 on vertical extension (or vice versa)
            vec![(sx, sy), (p1c, p1r), (p1c, p2r), (p2c, p2r), (ex, ey)]
        } else {
            vec![(sx, sy), (p1c, p1r), (p2c, p1r), (p2c, p2r), (ex, ey)]
        }
    }
}

/// Extends a point in the given direction by `distance`.
fn extend_point(c: usize, r: usize, dir: Dir, distance: usize) -> (usize, usize) {
    match dir {
        Dir::Right => (c + distance, r),
        Dir::Left => (c.saturating_sub(distance), r),
        Dir::Down => (c, r + distance),
        Dir::Up => (c, r.saturating_sub(distance)),
    }
}

/// Returns the corner character for a waypoint where incoming direction meets outgoing direction.
pub fn corner_char(incoming: Dir, outgoing: Dir) -> char {
    match (incoming, outgoing) {
        (Dir::Right, Dir::Down) => '┐',
        (Dir::Right, Dir::Up) => '┘',
        (Dir::Left, Dir::Down) => '┌',
        (Dir::Left, Dir::Up) => '└',
        (Dir::Down, Dir::Right) => '└',
        (Dir::Down, Dir::Left) => '┘',
        (Dir::Up, Dir::Right) => '┌',
        (Dir::Up, Dir::Left) => '┐',
        (Dir::Left, Dir::Left) | (Dir::Right, Dir::Right) => '─',
        (Dir::Up, Dir::Up) | (Dir::Down, Dir::Down) => '│',
        _ => '┼',
    }
}

/// Computes the direction from point (fx,fy) to point (tx,ty).
/// Points must share an axis (same row or same column).
pub fn segment_dir(fx: usize, fy: usize, tx: usize, ty: usize) -> Dir {
    if fy == ty {
        if tx > fx { Dir::Right } else { Dir::Left }
    } else if ty > fy {
        Dir::Down
    } else {
        Dir::Up
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn straight_horizontal_route() {
        // right.src → left.dst, aligned
        let wp = compute_route(10, 5, Side::Right, 20, 5, Side::Left);
        assert_eq!(wp, vec![(10, 5), (20, 5)]);
    }

    #[test]
    fn straight_vertical_route() {
        let wp = compute_route(5, 3, Side::Bottom, 5, 10, Side::Top);
        assert_eq!(wp, vec![(5, 3), (5, 10)]);
    }

    #[test]
    fn l_shape_right_to_top() {
        // src exits right, dst enters from top (down)
        let wp = compute_route(10, 5, Side::Right, 20, 10, Side::Top);
        assert_eq!(wp, vec![(10, 5), (20, 5), (20, 10)]);
    }

    #[test]
    fn l_shape_bottom_to_left() {
        let wp = compute_route(5, 5, Side::Bottom, 15, 10, Side::Left);
        assert_eq!(wp, vec![(5, 5), (5, 10), (15, 10)]);
    }

    #[test]
    fn z_shape_opposite_vertical() {
        // Both vertical, opposite directions, not aligned
        let wp = compute_route(5, 3, Side::Bottom, 15, 10, Side::Top);
        // midpoint y = (3+10)/2 = 6
        assert_eq!(wp, vec![(5, 3), (5, 6), (15, 6), (15, 10)]);
    }

    #[test]
    fn u_shape_same_side_right() {
        // Both exit/enter from right → ㄷ-shape
        let wp = compute_route(10, 2, Side::Right, 10, 8, Side::Right);
        // mid_x = max(10,10) + 2 = 12
        assert_eq!(wp, vec![(10, 2), (12, 2), (12, 8), (10, 8)]);
    }

    #[test]
    fn corner_chars_correct() {
        assert_eq!(corner_char(Dir::Right, Dir::Down), '┐');
        assert_eq!(corner_char(Dir::Right, Dir::Up), '┘');
        assert_eq!(corner_char(Dir::Down, Dir::Right), '└');
        assert_eq!(corner_char(Dir::Up, Dir::Right), '┌');
    }
}
