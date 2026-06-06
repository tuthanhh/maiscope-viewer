// Self-contained slide path geometry; not yet wired into spawning.
#![allow(dead_code)]

use bevy::prelude::Vec2;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, FRAC_PI_8, PI, TAU};

use crate::systems::component::{Duration, SlideSegment, SlideShape};

use super::component::SlidePath;

// cos(3*pi/8) = sin(pi/8) ≈ 0.3827 — natural tangent-circle radius for P/Q inner loop
const INNER_RING_FRAC: f32 = 0.382_683_43;
// B-sensor ring fraction — matches ButtonLayout's `b` ring at 2.3/4.8 of the radius.
const B_RING_FRAC: f32 = 2.3 / 4.8;

// ── Button geometry ──────────────────────────────────────────────────────────
// Matches ButtonLayout's angle convention: button index 1..=8 increases
// counter-clockwise, with button 1 at FRAC_PI_8 (ButtonLayout's rings use
// `FRAC_PI_8 + i·FRAC_PI_4`).

/// Polar angle of a button's spoke, in radians.
fn button_angle(button: usize) -> f32 {
    FRAC_PI_8 + (button - 1) as f32 * FRAC_PI_4
}

/// A-sensor (outer judgment ring) position at the full boundary radius.
fn a_sensor_pos(button: usize, boundary_radius: f32) -> Vec2 {
    let ang = button_angle(button);
    Vec2::new(boundary_radius * ang.cos(), boundary_radius * ang.sin())
}

/// B-sensor (inner ring) position at `B_RING_FRAC` of the boundary radius.
fn b_sensor_pos(button: usize, boundary_radius: f32) -> Vec2 {
    let r = boundary_radius * B_RING_FRAC;
    let ang = button_angle(button);
    Vec2::new(r * ang.cos(), r * ang.sin())
}

/// The play-field center.
fn center_pos() -> Vec2 {
    Vec2::ZERO
}

/// Button `steps` positions counter-clockwise (increasing index, wrapping 1..=8).
fn button_ccw(button: usize, steps: usize) -> usize {
    (button - 1 + steps) % 8 + 1
}

/// Button `steps` positions clockwise (decreasing index, wrapping 1..=8).
fn button_cw(button: usize, steps: usize) -> usize {
    ((button - 1) as isize - steps as isize).rem_euclid(8) as usize + 1
}

/// Number of counter-clockwise steps from `start` to `end`.
fn ccw_distance(start: usize, end: usize) -> usize {
    (end as isize - start as isize).rem_euclid(8) as usize
}

/// Number of clockwise steps from `start` to `end`.
fn cw_distance(start: usize, end: usize) -> usize {
    (start as isize - end as isize).rem_euclid(8) as usize
}

/// Compute the two tangent points from an external point `p` to a circle
/// **centered at the origin** with radius `r`.
///
/// Returns `(T_ccw, T_cw)` where:
/// - `T_ccw`: tangent point where the arc is on the **CCW (left)** side of the
///   incoming ray from `p`  — `p × T_ccw > 0`
/// - `T_cw`:  tangent point where the arc is on the **CW (right)** side
///   — `p × T_cw < 0`
///
/// Panics (debug) if `p` is inside or on the circle.
fn tangent_points_to_circle(p: Vec2, r: f32) -> (Vec2, Vec2) {
    let d_sq = p.length_squared();
    let l = (d_sq - r * r).max(0.0).sqrt(); // tangent length
    let perp = Vec2::new(-p.y, p.x); // p rotated 90° CCW

    // T± = (r²·p ± r·L·p⊥) / |p|²
    let t_ccw = (r * r * p + r * l * perp) / d_sq;
    let t_cw = (r * r * p - r * l * perp) / d_sq;
    (t_ccw, t_cw)
}

/// Compute the two tangent points from an external point `p` to a circle
/// centered at `center` with radius `r`.
///
/// Returns `(T_ccw, T_cw)` in the same convention as [`tangent_points_to_circle`],
/// but relative to the circle's center (not the origin).
fn tangent_points_to_offset_circle(p: Vec2, center: Vec2, r: f32) -> (Vec2, Vec2) {
    let p_local = p - center;
    let (t_ccw_local, t_cw_local) = tangent_points_to_circle(p_local, r);
    (center + t_ccw_local, center + t_cw_local)
}

/// Generate a G1-smooth **blend arc** from `p_from` to `tangent_pt`, where
/// `tangent_pt` lies on a circle centred at `circle_center`.
///
/// The blend arc arrives at `tangent_pt` sharing the exact same tangent
/// direction as the loop circle, so the junction has no direction kink.
///
/// Geometry:
///   The blend-arc center must lie on the outward normal of the loop circle at
///   `tangent_pt` (this is the only line whose perpendicular at `tangent_pt`
///   is also tangent to the loop circle).  Combined with the requirement that
///   `p_from` lies on the arc, the center is uniquely determined:
///
/// ```text
///     outward = (tangent_pt - circle_center) / r   (unit outward normal)
///     d       = tangent_pt - p_from                (chord)
///     t       = -|d|^2 / (2 * (d . outward))
///     center  = tangent_pt + t * outward
/// ```
///
/// `ccw` — desired winding of the blend arc (true = CCW, false = CW).
///         Must match the winding of the loop arc that follows/precedes it.
///
/// Falls back to a two-point straight segment on degenerate input.
fn blend_arc(
    p_from: Vec2,
    tangent_pt: Vec2,
    circle_center: Vec2,
    ccw: bool,
    spacing: f32,
) -> Vec<Vec2> {
    let outward = (tangent_pt - circle_center).normalize_or_zero();
    let d = tangent_pt - p_from;
    let d_dot_n = d.dot(outward);

    if d_dot_n.abs() < 1e-4 || d.length_squared() < 1e-4 {
        return vec![p_from, tangent_pt];
    }

    let t = -d.length_squared() / (2.0 * d_dot_n);
    let arc_center = tangent_pt + t * outward;
    let arc_radius = arc_center.distance(p_from);

    if arc_radius < 1e-4 {
        return vec![p_from, tangent_pt];
    }

    let start_ang = (p_from - arc_center).to_angle();
    let end_ang_raw = (tangent_pt - arc_center).to_angle();

    // Shift end_ang into the correct half-plane for the requested winding,
    // keeping the arc strictly less than one full revolution.
    let end_ang = if ccw {
        let mut e = end_ang_raw;
        while e <= start_ang {
            e += TAU;
        }
        while e > start_ang + TAU {
            e -= TAU;
        }
        e
    } else {
        let mut e = end_ang_raw;
        while e >= start_ang {
            e -= TAU;
        }
        while e < start_ang - TAU {
            e += TAU;
        }
        e
    };

    generate_offset_arc_points(arc_center, start_ang, end_ang, arc_radius, spacing)
}

/// Calculate the total physical distance along a polyline path.
pub fn calculate_total_length(points: &[Vec2]) -> f32 {
    let mut length = 0.0;
    for window in points.windows(2) {
        length += window[0].distance(window[1]);
    }
    length
}

/// Find the exact position and facing angle at a specific distance along a polyline path.
pub fn get_transform_at_distance(points: &[Vec2], target_distance: f32) -> (Vec2, f32) {
    let mut current_dist = 0.0;

    for window in points.windows(2) {
        let p1 = window[0];
        let p2 = window[1];
        let segment_length = p1.distance(p2);

        if segment_length < f32::EPSILON {
            continue;
        }

        if current_dist + segment_length >= target_distance {
            let t = (target_distance - current_dist) / segment_length;
            let exact_pos = p1.lerp(p2, t);
            let angle = (p2.y - p1.y).atan2(p2.x - p1.x);
            return (exact_pos, angle);
        }
        current_dist += segment_length;
    }

    // Fallback: return the last point
    if points.len() >= 2 {
        let last = points[points.len() - 1];
        let prev = points[points.len() - 2];
        let angle = (last.y - prev.y).atan2(last.x - prev.x);
        (last, angle)
    } else if let Some(p) = points.last() {
        (*p, 0.0)
    } else {
        (Vec2::ZERO, 0.0)
    }
}

// Append `next_segment` to `base`, skipping the first point if it duplicates
// the last point of `base` (within 1.0 units).
fn append_path_dedup(base: &mut Vec<Vec2>, mut next_segment: Vec<Vec2>) {
    if let (Some(last), Some(first)) = (base.last(), next_segment.first()) {
        if last.distance(*first) < 1.0 {
            next_segment.remove(0);
        }
    }
    base.extend(next_segment);
}

/// Generate the waypoint path for a single slide segment.
///
/// The path follows the actual maimai touch sensor positions as documented
/// in the official slide touch sensor usage guide. The segment's start button
/// is supplied separately (it comes from the slide head, or the previous
/// segment's end); `SlideShape` only carries the segment's end(s).
///
/// # Arguments
/// * `shape` - The slide shape to generate
/// * `start_button` - 1-based button the segment starts from
/// * `boundary_radius` - The radius of the outer A-sensor ring (judgment line)
pub fn generate_points(shape: &SlideShape, start_button: usize, boundary_radius: f32) -> Vec<Vec2> {
    let s = start_button;
    match shape {
        SlideShape::Straight { end } => generate_shape_straight(s, *end, boundary_radius),
        SlideShape::ShortArc { end } => generate_shape_short_arc(s, *end, boundary_radius),
        SlideShape::ClockwiseArc { end } => generate_shape_clockwise_arc(s, *end, boundary_radius),
        SlideShape::CounterClockwiseArc { end } => generate_shape_ccw_arc(s, *end, boundary_radius),
        SlideShape::VShape { end } => generate_shape_v(s, *end, boundary_radius),
        SlideShape::GrandVShape { mid, end } => {
            generate_shape_grand_v(s, *end, *mid, boundary_radius)
        }
        SlideShape::PShape { end } => generate_shape_inner_loop(s, *end, false, boundary_radius),
        SlideShape::QShape { end } => generate_shape_inner_loop(s, *end, true, boundary_radius),
        SlideShape::GrandPShape { end } => {
            generate_shape_grand_loop(s, *end, false, boundary_radius)
        }
        SlideShape::GrandQShape { end } => {
            generate_shape_grand_loop(s, *end, true, boundary_radius)
        }
        SlideShape::Thunderbolt { end, is_z } => {
            generate_shape_thunderbolt(s, *end, *is_z, boundary_radius)
        }
        SlideShape::FanShape {
            ends: (e1, _e2, _e3),
        } => generate_shape_fan(s, *e1, boundary_radius),
    }
}

// ============================================================================
// Per-shape geometry generators
// ============================================================================

// STRAIGHT (-): Direct line from start A-sensor to end A-sensor.
fn generate_shape_straight(start: usize, end: usize, boundary_radius: f32) -> Vec<Vec2> {
    let p1 = a_sensor_pos(start, boundary_radius);
    let p2 = a_sensor_pos(end, boundary_radius);
    generate_multi_segment_points(&[p1, p2], 35.0)
}

// SHORT ARC (^): The shortest arc along the boundary ring (never > 180°).
fn generate_shape_short_arc(start: usize, end: usize, boundary_radius: f32) -> Vec<Vec2> {
    let start_ang = button_angle(start);
    let end_ang = button_angle(end);

    let mut diff = (end_ang - start_ang) % TAU;
    if diff > PI {
        diff -= TAU;
    }
    if diff < -PI {
        diff += TAU;
    }
    let target = start_ang + diff;

    generate_arc_points(start_ang, target, boundary_radius, 35.0)
}

// CLOCKWISE ARC (>): Arc that passes to the RIGHT of the straight line from
// start to end. Cross product of (end−start) × (mid−start) < 0 = right side.
// Opposite buttons: default CW. Same button: full circle CW.
fn generate_shape_clockwise_arc(start: usize, end: usize, boundary_radius: f32) -> Vec<Vec2> {
    let spacing = 35.0;
    let start_ang = button_angle(start);
    let end_ang = button_angle(end);

    if start == end {
        return generate_arc_points(start_ang, start_ang - TAU, boundary_radius, spacing);
    }

    let steps = (end as isize - start as isize).rem_euclid(8) as usize;
    if steps == 4 {
        let cw_end = {
            let mut e = end_ang;
            while e >= start_ang {
                e -= TAU;
            }
            e
        };
        return generate_arc_points(start_ang, cw_end, boundary_radius, spacing);
    }

    let p_start = Vec2::new(
        boundary_radius * start_ang.cos(),
        boundary_radius * start_ang.sin(),
    );
    let p_end = Vec2::new(
        boundary_radius * end_ang.cos(),
        boundary_radius * end_ang.sin(),
    );
    let cw_end = {
        let mut e = end_ang;
        while e >= start_ang {
            e -= TAU;
        }
        e
    };
    let mid_cw_ang = start_ang + (cw_end - start_ang) * 0.5;
    let mid_cw = Vec2::new(
        boundary_radius * mid_cw_ang.cos(),
        boundary_radius * mid_cw_ang.sin(),
    );
    let dir = p_end - p_start;
    let to_mid = mid_cw - p_start;
    let cross = dir.x * to_mid.y - dir.y * to_mid.x;

    if cross < 0.0 {
        generate_arc_points(start_ang, cw_end, boundary_radius, spacing)
    } else if cross > 0.0 {
        let ccw_end = {
            let mut e = end_ang;
            while e <= start_ang {
                e += TAU;
            }
            e
        };
        generate_arc_points(start_ang, ccw_end, boundary_radius, spacing)
    } else {
        generate_arc_points(start_ang, cw_end, boundary_radius, spacing)
    }
}

// COUNTER-CLOCKWISE ARC (<): Arc that passes to the LEFT of the straight line
// from start to end. Cross product > 0 = left side.
// Opposite buttons: force CW (complementary half to >). Same button: full circle CCW.
fn generate_shape_ccw_arc(start: usize, end: usize, boundary_radius: f32) -> Vec<Vec2> {
    let spacing = 35.0;
    let start_ang = button_angle(start);
    let end_ang = button_angle(end);

    if start == end {
        return generate_arc_points(start_ang, start_ang + TAU, boundary_radius, spacing);
    }

    let steps = (end as isize - start as isize).rem_euclid(8) as usize;
    if steps == 4 {
        let cw_end = {
            let mut e = end_ang;
            while e >= start_ang {
                e -= TAU;
            }
            e
        };
        return generate_arc_points(start_ang, cw_end, boundary_radius, spacing);
    }

    let p_start = Vec2::new(
        boundary_radius * start_ang.cos(),
        boundary_radius * start_ang.sin(),
    );
    let p_end = Vec2::new(
        boundary_radius * end_ang.cos(),
        boundary_radius * end_ang.sin(),
    );
    let ccw_end = {
        let mut e = end_ang;
        while e <= start_ang {
            e += TAU;
        }
        e
    };
    let mid_ccw_ang = start_ang + (ccw_end - start_ang) * 0.5;
    let mid_ccw = Vec2::new(
        boundary_radius * mid_ccw_ang.cos(),
        boundary_radius * mid_ccw_ang.sin(),
    );
    let dir = p_end - p_start;
    let to_mid = mid_ccw - p_start;
    let cross = dir.x * to_mid.y - dir.y * to_mid.x;

    if cross > 0.0 {
        generate_arc_points(start_ang, ccw_end, boundary_radius, spacing)
    } else if cross < 0.0 {
        let cw_end = {
            let mut e = end_ang;
            while e >= start_ang {
                e -= TAU;
            }
            e
        };
        generate_arc_points(start_ang, cw_end, boundary_radius, spacing)
    } else {
        generate_arc_points(start_ang, ccw_end, boundary_radius, spacing)
    }
}

// V-SHAPE (v): start A-sensor → center → end A-sensor.
fn generate_shape_v(start: usize, end: usize, boundary_radius: f32) -> Vec<Vec2> {
    let p_start = a_sensor_pos(start, boundary_radius);
    let p_center = center_pos();
    let p_end = a_sensor_pos(end, boundary_radius);
    generate_multi_segment_points(&[p_start, p_center, p_end], 35.0)
}

// GRAND V-SHAPE (V): start A-sensor → mid A-sensor → end A-sensor.
fn generate_shape_grand_v(start: usize, end: usize, mid: usize, boundary_radius: f32) -> Vec<Vec2> {
    let p_start = a_sensor_pos(start, boundary_radius);
    let p_mid = a_sensor_pos(mid, boundary_radius);
    let p_end = a_sensor_pos(end, boundary_radius);
    generate_multi_segment_points(&[p_start, p_mid, p_end], 35.0)
}

// INNER LOOP — P (p, CCW) and Q (q, CW): tangent to the R/3 inner circle.
//
// Entry and exit use opposite tangent sides:
//   P (CCW arc): entry = T_ccw(start), exit = T_cw(end)
//   Q (CW  arc): entry = T_cw(start),  exit = T_ccw(end)
//
// Full-circle rule: add extra TAU when end is 1–3 steps in the arc direction.
fn generate_shape_inner_loop(
    start: usize,
    end: usize,
    is_q: bool,
    boundary_radius: f32,
) -> Vec<Vec2> {
    let spacing = 35.0;
    let r_inner = boundary_radius * INNER_RING_FRAC;

    let p_start = a_sensor_pos(start, boundary_radius);
    let p_end = a_sensor_pos(end, boundary_radius);

    let (t_ccw_entry, t_cw_entry) = tangent_points_to_circle(p_start, r_inner);
    let (t_ccw_exit, t_cw_exit) = tangent_points_to_circle(p_end, r_inner);

    let tangent_entry = if is_q { t_cw_entry } else { t_ccw_entry };
    let tangent_exit = if is_q { t_ccw_exit } else { t_cw_exit };

    let arc_start_ang = tangent_entry.to_angle();
    let mut arc_end_ang = tangent_exit.to_angle();

    let needs_full_circle = if is_q {
        cw_distance(start, end) >= 5
    } else {
        ccw_distance(start, end) >= 5
    };

    if is_q {
        while arc_end_ang >= arc_start_ang {
            arc_end_ang -= TAU;
        }
        if needs_full_circle {
            arc_end_ang -= TAU;
        }
    } else {
        while arc_end_ang <= arc_start_ang {
            arc_end_ang += TAU;
        }
        if needs_full_circle {
            arc_end_ang += TAU;
        }
    }

    let mut path = blend_arc(p_start, tangent_entry, Vec2::ZERO, !is_q, spacing);
    append_path_dedup(
        &mut path,
        generate_arc_points(arc_start_ang, arc_end_ang, r_inner, spacing),
    );

    let mut exit_seg = blend_arc(p_end, tangent_exit, Vec2::ZERO, is_q, spacing);
    exit_seg.reverse();
    append_path_dedup(&mut path, exit_seg);

    path
}

// OUTER LOOP — PP (pp, CCW) and QQ (qq, CW): loop through a circle of radius R/2
// positioned 90° perpendicular to the start spoke. Exit waypoint varies by
// distance from start to end.
fn generate_shape_grand_loop(
    start: usize,
    end: usize,
    is_q: bool,
    boundary_radius: f32,
) -> Vec<Vec2> {
    let spacing = 35.0;
    let r_loop = boundary_radius * 0.5;
    let start_ang = button_angle(start);

    // Loop circle sits 90° to the side of the spoke (internally tangent to boundary).
    let center_ang = if is_q {
        start_ang + FRAC_PI_2
    } else {
        start_ang - FRAC_PI_2
    };
    let loop_center = Vec2::new(r_loop * center_ang.cos(), r_loop * center_ang.sin());

    let p_start = a_sensor_pos(start, boundary_radius);
    let a_end = a_sensor_pos(end, boundary_radius);

    let (t_ccw_entry, t_cw_entry) = tangent_points_to_offset_circle(p_start, loop_center, r_loop);
    let tangent_entry = if is_q { t_cw_entry } else { t_ccw_entry };
    let arc_entry_ang = (tangent_entry - loop_center).to_angle();

    let dist = if is_q {
        cw_distance(start, end)
    } else {
        ccw_distance(start, end)
    };
    let exit_waypoint = match dist {
        0 | 1 => {
            if is_q {
                a_sensor_pos(button_ccw(start, 1), boundary_radius)
            } else {
                a_sensor_pos(button_cw(start, 1), boundary_radius)
            }
        }
        2 | 3 => b_sensor_pos(start, boundary_radius),
        4 | 5 => Vec2::ZERO,
        6 => b_sensor_pos(button_ccw(end, 1), boundary_radius),
        _ => a_end,
    };

    let (t_ccw_exit, t_cw_exit) =
        tangent_points_to_offset_circle(exit_waypoint, loop_center, r_loop);
    let tangent_exit = if is_q { t_ccw_exit } else { t_cw_exit };
    let arc_exit_ang = (tangent_exit - loop_center).to_angle();

    let mut target_arc_ang = arc_exit_ang;
    if is_q {
        while target_arc_ang >= arc_entry_ang - FRAC_PI_4 {
            target_arc_ang -= TAU;
        }
    } else {
        while target_arc_ang <= arc_entry_ang + FRAC_PI_4 {
            target_arc_ang += TAU;
        }
    }

    let mut path = blend_arc(p_start, tangent_entry, loop_center, !is_q, spacing);
    append_path_dedup(
        &mut path,
        generate_offset_arc_points(loop_center, arc_entry_ang, target_arc_ang, r_loop, spacing),
    );

    let exit_target = if exit_waypoint.distance(a_end) > 1.0 {
        exit_waypoint
    } else {
        a_end
    };
    let mut exit_seg = blend_arc(exit_target, tangent_exit, loop_center, is_q, spacing);
    exit_seg.reverse();
    append_path_dedup(&mut path, exit_seg);

    if exit_waypoint.distance(a_end) > 1.0 {
        append_path_dedup(
            &mut path,
            generate_multi_segment_points(&[exit_waypoint, a_end], spacing),
        );
    }

    path
}

// THUNDERBOLT / ZIGZAG (s / z): 5-point zigzag through B-sensors.
//   s: start → B(start-2) → center → B(end-2) → end
//   z: start → B(start+2) → center → B(end+2) → end
fn generate_shape_thunderbolt(
    start: usize,
    end: usize,
    is_z: bool,
    boundary_radius: f32,
) -> Vec<Vec2> {
    let p_start = a_sensor_pos(start, boundary_radius);
    let p_end = a_sensor_pos(end, boundary_radius);
    let p_center = center_pos();

    let b1 = if is_z {
        button_cw(start, 2)
    } else {
        button_ccw(start, 2)
    };
    let b3 = if is_z {
        button_cw(end, 2)
    } else {
        button_ccw(end, 2)
    };

    generate_multi_segment_points(
        &[
            p_start,
            b_sensor_pos(b1, boundary_radius),
            p_center,
            b_sensor_pos(b3, boundary_radius),
            p_end,
        ],
        35.0,
    )
}

// FAN SHAPE (w): Path to first end only. The spawner creates 3 separate
// simultaneous slides for end1, end2, end3.
fn generate_shape_fan(start: usize, end1: usize, boundary_radius: f32) -> Vec<Vec2> {
    let p_start = a_sensor_pos(start, boundary_radius);
    let p_end = a_sensor_pos(end1, boundary_radius);
    generate_multi_segment_points(&[p_start, p_end], 35.0)
}

// ============================================================================
// Path generation helpers
// ============================================================================

/// Generate evenly-spaced points along a circular arc.
pub fn generate_arc_points(
    start_angle: f32,
    end_angle: f32,
    radius: f32,
    spacing: f32,
) -> Vec<Vec2> {
    let angle_diff = (end_angle - start_angle).abs();
    let arc_length = radius * angle_diff;
    let steps = (arc_length / spacing).ceil().max(1.0) as usize;
    let mut points = Vec::with_capacity(steps + 1);

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let current_angle = start_angle + (end_angle - start_angle) * t;
        points.push(Vec2::new(
            radius * current_angle.cos(),
            radius * current_angle.sin(),
        ));
    }
    points
}

/// Generate evenly-spaced points along an arc centered at an arbitrary point.
#[allow(dead_code)]
pub fn generate_offset_arc_points(
    center: Vec2,
    start_angle: f32,
    end_angle: f32,
    radius: f32,
    spacing: f32,
) -> Vec<Vec2> {
    let angle_diff = (end_angle - start_angle).abs();
    let arc_length = radius * angle_diff;
    let steps = (arc_length / spacing).ceil().max(1.0) as usize;
    let mut points = Vec::with_capacity(steps + 1);

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let current_angle = start_angle + (end_angle - start_angle) * t;
        points.push(center + Vec2::new(radius * current_angle.cos(), radius * current_angle.sin()));
    }
    points
}

/// Generate evenly-spaced points along a multi-segment polyline.
pub fn generate_multi_segment_points(waypoints: &[Vec2], spacing: f32) -> Vec<Vec2> {
    let mut points = Vec::new();

    for w in 0..(waypoints.len() - 1) {
        let p1 = waypoints[w];
        let p2 = waypoints[w + 1];
        let distance = p1.distance(p2);
        let steps = (distance / spacing).ceil().max(1.0) as usize;

        let start_i = if w == 0 { 0 } else { 1 };

        for i in start_i..=steps {
            let t = i as f32 / steps as f32;
            points.push(p1.lerp(p2, t));
        }
    }
    points
}

// ============================================================================
// Slide trace assembly (geometry + timing)
// ============================================================================

/// Per-segment `(wait_secs, trace_secs)` for a slide segment's duration.
///
/// When the duration carries no explicit wait/trace (`Simple` / `BpmOverride`
/// / `BpmOverrideSeconds`), the wait defaults to one beat at the *current
/// note's* BPM and the trace uses the same formula as hold notes. The
/// `ExplicitWait*` variants supply the wait (and, for `ExplicitWaitAndTrace`,
/// the trace) directly.
fn slide_timing(duration: Duration, note_bpm: f32) -> (f32, f32) {
    let beat = 60.0 / note_bpm;
    match duration {
        Duration::ExplicitWaitAndTrace {
            wait_seconds,
            trace_seconds,
        } => (wait_seconds, trace_seconds),
        Duration::ExplicitWaitBeats {
            wait_seconds,
            divider,
            count,
        } => (
            wait_seconds,
            count as f32 / divider as f32 * (240.0 / note_bpm),
        ),
        Duration::ExplicitWaitBpmBeats {
            wait_seconds,
            bpm,
            divider,
            count,
        } => (wait_seconds, count as f32 / divider as f32 * (240.0 / bpm)),
        Duration::Simple { divider, count } => {
            (beat, count as f32 / divider as f32 * (240.0 / note_bpm))
        }
        Duration::BpmOverride {
            bpm,
            divider,
            count,
        } => (beat, count as f32 / divider as f32 * (240.0 / bpm)),
        Duration::BpmOverrideSeconds { seconds, .. } => (beat, seconds),
    }
}

/// The end button a slide segment terminates on (used to chain segments).
fn slide_shape_end(shape: &SlideShape) -> usize {
    match shape {
        SlideShape::Straight { end }
        | SlideShape::ShortArc { end }
        | SlideShape::ClockwiseArc { end }
        | SlideShape::CounterClockwiseArc { end }
        | SlideShape::VShape { end }
        | SlideShape::PShape { end }
        | SlideShape::QShape { end }
        | SlideShape::GrandPShape { end }
        | SlideShape::GrandQShape { end }
        | SlideShape::GrandVShape { end, .. }
        | SlideShape::Thunderbolt { end, .. } => *end,
        SlideShape::FanShape { ends: (e1, ..) } => *e1,
    }
}

/// Build the full slide trace: concatenated waypoints plus the time/distance
/// breakpoints that drive the tracing star.
///
/// * `shared_duration` — when true, the (single) duration covers the whole
///   chained path and is split across segments by arc length (constant speed);
///   otherwise each segment is traced over its own duration, sequentially.
pub fn build_slide_trace(
    segments: &[SlideSegment],
    start_button: usize,
    note_bpm: f32,
    shared_duration: bool,
    boundary_radius: f32,
) -> SlidePath {
    let mut waypoints: Vec<Vec2> = Vec::new();
    let mut seg_lengths: Vec<f32> = Vec::with_capacity(segments.len());
    let mut current_start = start_button;

    for seg in segments {
        let pts = generate_points(&seg.shape, current_start, boundary_radius);
        seg_lengths.push(calculate_total_length(&pts));
        append_path_dedup(&mut waypoints, pts);
        current_start = slide_shape_end(&seg.shape);
    }

    let total_length: f32 = seg_lengths.iter().sum();
    let wait_secs = segments
        .first()
        .map(|s| slide_timing(s.duration, note_bpm).0)
        .unwrap_or(0.0);
    // Shared total = the single duration covering the whole path.
    let shared_total = segments
        .first()
        .map(|s| slide_timing(s.duration, note_bpm).1)
        .unwrap_or(0.0);

    let mut breakpoints = Vec::with_capacity(segments.len());
    let mut cum_t = 0.0;
    let mut cum_d = 0.0;
    for (i, seg) in segments.iter().enumerate() {
        let secs_i = if shared_duration {
            if total_length > 0.0 {
                shared_total * seg_lengths[i] / total_length
            } else {
                shared_total / segments.len().max(1) as f32
            }
        } else {
            slide_timing(seg.duration, note_bpm).1
        };
        cum_t += secs_i;
        cum_d += seg_lengths[i];
        breakpoints.push((cum_t, cum_d));
    }

    SlidePath {
        waypoints,
        total_length,
        breakpoints,
        wait_secs,
    }
}

/// Total trace time (seconds) of the Sliding phase.
pub fn trace_total_secs(path: &SlidePath) -> f32 {
    path.breakpoints.last().map(|b| b.0).unwrap_or(0.0)
}

/// Position and facing angle of the tracing star at `elapsed` seconds into the
/// Sliding phase (piecewise-linear time→distance across segment breakpoints).
pub fn trace_position(path: &SlidePath, elapsed: f32) -> (Vec2, f32) {
    get_transform_at_distance(&path.waypoints, trace_distance(path, elapsed))
}

/// Map `elapsed` seconds to a distance along the concatenated path.
pub fn trace_distance(path: &SlidePath, elapsed: f32) -> f32 {
    let mut prev_t = 0.0;
    let mut prev_d = 0.0;
    for &(t_end, d_end) in &path.breakpoints {
        if elapsed <= t_end {
            let span = t_end - prev_t;
            let local = if span > 0.0 {
                (elapsed - prev_t) / span
            } else {
                1.0
            };
            return prev_d + (d_end - prev_d) * local;
        }
        prev_t = t_end;
        prev_d = d_end;
    }
    path.total_length
}
