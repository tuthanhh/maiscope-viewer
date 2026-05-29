//! Lyon-based shape path builders for all note visuals.
//!
//! Each builder returns a [`ShapePath`] that can be turned into a [`Shape`]
//! at spawn time with the desired color via:
//!
//! ```ignore
//! ShapeBuilder::with(&path).fill(color).build()
//! ```
//!
//! Hold arch and body paths trace **both** the outer and inner contours
//! (outer clockwise, inner counter-clockwise) so that a simple `.fill()`
//! produces the correct hollow polygon.  This avoids stroke-based rendering
//! which would distort when the body is scaled in Y during gameplay.

use bevy::{color::Color, prelude::Vec2};
use bevy_prototype_lyon::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

use crate::systems::visual::{NOTE_RADIUS, resources::NoteAssets};

// ============================================================================
// Tap note — annulus (stroked circle)
// ============================================================================

/// Build the `ShapePath` for a tap note ring.
///
/// A full-circle arc at `mid_radius` (midpoint between inner and outer radii).
/// Stroke width at spawn time should be `radius * 0.25` to reproduce the old
/// `Annulus::new(radius, radius * 0.75)` look.
///
/// * `radius` — outer note radius (same value previously passed to `Annulus`).
pub fn build_tap_ring_path(radius: f32) -> ShapePath {
    let mid_r = radius * 0.875; // midpoint of outer (r) and inner (0.75r)
    // Start at the rightmost point of the circle and sweep a full turn.
    ShapePath::new()
        .move_to(Vec2::new(mid_r, 0.0))
        .arc(Vec2::ZERO, Vec2::splat(mid_r), TAU, 0.0)
}

// ============================================================================
// Hold note — arch (head / tail)
// ============================================================================

/// Build the `ShapePath` for a hold note head or tail.
///
/// The shape is the **upper half of a hexagon** — a 5-point arch with a
/// flat baseline at y = 0 and a peak at y = radius.  Both the outer and
/// inner contours are traced so that `.fill()` produces a hollow ring
/// polygon (no stroke needed).
///
/// Outer vertices (from the original `create_hollow_arch`):
///   `(-r, 0)  (-r, r·0.5)  (0, r)  (r, r·0.5)  (r, 0)`
///
/// Inner vertices at 0.75× scale:
///   `(-r·0.75, 0)  (-r·0.75, r·0.375)  (0, r·0.75)  (r·0.75, r·0.375)  (r·0.75, 0)`
///
/// For the tail, rotate the spawned entity by π.
///
/// * `radius` — outer note radius.
pub fn build_hold_arch_path(radius: f32) -> ShapePath {
    let r = radius;
    let r_in = r * 0.75;
    let y_out = r * 0.5;
    let y_in = y_out * 0.75; // r * 0.375

    // Outer contour (clockwise: left → peak → right)
    ShapePath::new()
        .move_to(Vec2::new(-r, 0.0))
        .line_to(Vec2::new(-r, y_out))
        .line_to(Vec2::new(0.0, r))
        .line_to(Vec2::new(r, y_out))
        .line_to(Vec2::new(r, 0.0))
        // Inner contour (counter-clockwise: right → peak → left) to cut the hole
        .line_to(Vec2::new(r_in, 0.0))
        .line_to(Vec2::new(r_in, y_in))
        .line_to(Vec2::new(0.0, r_in))
        .line_to(Vec2::new(-r_in, y_in))
        .line_to(Vec2::new(-r_in, 0.0))
        .close()
}

// ============================================================================
// Hold note — body (filled hollow rectangle)
// ============================================================================

/// Build the `ShapePath` for a hold note body (the beam connecting head and tail).
///
/// Traces both the outer and inner rectangles to produce a filled hollow
/// rectangle matching the old `Ring { Rectangle(w, 1.0), Rectangle(w·0.75, 1.0) }`.
///
/// The shape has **unit height** (1.0) and is scaled in Y by the movement
/// system.  Because the path is *filled* (not stroked), scaling only moves
/// vertices and does not distort any stroke width.
///
/// * `hex_flat_width` — `radius * 2.0`, the full width of the body beam.
pub fn build_hold_body_path(hex_flat_width: f32) -> ShapePath {
    let w_out = hex_flat_width;
    let w_in = hex_flat_width * 0.75;
    let half_w_out = w_out / 2.0;
    let half_w_in = w_in / 2.0;
    let half_h = 0.5; // unit height, scaled at runtime

    // Outer rectangle (clockwise)
    ShapePath::new()
        .move_to(Vec2::new(-half_w_out, -half_h))
        .line_to(Vec2::new(half_w_out, -half_h))
        .line_to(Vec2::new(half_w_out, half_h))
        .line_to(Vec2::new(-half_w_out, half_h))
        .close()
        // Inner rectangle (counter-clockwise to cut hole)
        .move_to(Vec2::new(-half_w_in, -half_h))
        .line_to(Vec2::new(-half_w_in, half_h))
        .line_to(Vec2::new(half_w_in, half_h))
        .line_to(Vec2::new(half_w_in, -half_h))
        .close()
}

// ============================================================================
// Slide note — star
// ============================================================================

/// Build the `ShapePath` for a slide note star.
///
/// A closed polygon alternating between outer tips and inner dips.
/// Stroke width at spawn time should be `radius * 0.12`.
///
/// * `radius`     — outer tip radius.
/// * `ratio`      — inner dip radius as a fraction of `radius`.
/// * `num_points` — number of star points (e.g. 5).
pub fn build_slide_star_path(radius: f32, ratio: f32, num_points: usize) -> ShapePath {
    let num_verts = num_points * 2;
    let mut path = ShapePath::new();
    let mut first: Option<Vec2> = None;

    for i in 0..num_verts {
        let angle = (PI / 2.0) + (i as f32 * PI / num_points as f32);
        let r = if i % 2 == 0 { radius } else { radius * ratio };
        let pt = Vec2::new(r * angle.cos(), r * angle.sin());

        if i == 0 {
            path = path.move_to(pt);
            first = Some(pt);
        } else {
            path = path.line_to(pt);
        }
    }

    // Explicitly close the contour back to the first vertex.
    // .close() draws the final segment AND marks the path as closed so
    // Lyon's fill tessellator treats it as a proper closed polygon.
    let _ = first; // consumed by the loop; closure is handled by .close()
    path.close()
}

// ============================================================================
// Touch note — centre dot
// ============================================================================

/// Build the `ShapePath` for a touch note centre dot.
///
/// A full-circle arc meant to be *filled* (not stroked) at spawn time.
///
/// * `radius` — note radius; the dot is `radius * 0.1`.
pub fn build_touch_circle_path(radius: f32) -> ShapePath {
    let dot_r = radius * 0.1;
    ShapePath::new()
        .move_to(Vec2::new(dot_r, 0.0))
        .arc(Vec2::ZERO, Vec2::splat(dot_r), TAU, 0.0)
}

// ============================================================================
// Touch note — approach triangle (hollow, for regular Touch)
// ============================================================================

/// The start distance used when placing approach triangles.
/// Exposed as a function so `spawning.rs` and `movement.rs` always agree.
pub fn touch_triangle_start_distance(note_radius: f32) -> f32 {
    note_radius * 0.65
}

/// Build the `ShapePath` for a hollow approach triangle.
///
/// Reproduces the original `Ring<Triangle2d>` mesh by tracing **both**
/// the outer and inner triangle contours in opposite winding directions,
/// so `.fill()` with the even-odd rule produces a clean hollow triangle
/// with no stroke bleed.
///
/// Outer triangle vertices (same as before):
///   `(0, 0)`, `(w, w)`, `(-w, w)`   where `w = radius * SQRT_2 / 2`
///
/// Inner triangle vertices (inset by thickness `t = radius * 0.15`):
///   tip:   `(0,  t * SQRT_2)`
///   right: `(w - t*(1 + SQRT_2),  w - t)`
///   left:  `(-(w - t*(1 + SQRT_2)),  w - t)`
///
/// * `radius` — note radius.
pub fn build_touch_triangle_path(radius: f32) -> ShapePath {
    // Scale the outer triangle up so that after the inner cutout is subtracted,
    // the visible ring has the same perceived size as the solid touch-hold triangle.
    let r = radius * 1.2;
    let w = r * std::f32::consts::SQRT_2 / 2.0;
    let t = radius * 0.15;
    let sqrt2 = std::f32::consts::SQRT_2;

    // Outer contour — clockwise: tip(bottom) → right → left
    ShapePath::new()
        .move_to(Vec2::new(0.0, 0.0))
        .line_to(Vec2::new(w, w))
        .line_to(Vec2::new(-w, w))
        .close()
        // Inner contour — counter-clockwise (left → right → tip) to cut the hole
        .move_to(Vec2::new(-(w - t * (1.0 + sqrt2)), w - t))
        .line_to(Vec2::new(w - t * (1.0 + sqrt2), w - t))
        .line_to(Vec2::new(0.0, t * sqrt2))
        .close()
}

// ============================================================================
// Touch-hold note — approach triangle (filled, coloured per-direction)
// ============================================================================

/// Build the `ShapePath` for a filled approach triangle (touch-hold variant).
///
/// A simple filled triangle (no hole) — same outer geometry as the hollow
/// version, filled solid with one of the four directional colours.
///
/// * `radius` — note radius.
pub fn build_touch_hold_triangle_path(radius: f32) -> ShapePath {
    let w = radius * std::f32::consts::SQRT_2 / 2.0;
    ShapePath::new()
        .move_to(Vec2::new(0.0, 0.0))
        .line_to(Vec2::new(w, w))
        .line_to(Vec2::new(-w, w))
        .close()
}

// ============================================================================
// Slide track — chevron arrow
// ============================================================================

/// Build the `ShapePath` for a slide track chevron arrow.
pub fn build_chevron_path(radius: f32) -> ShapePath {
    let width = 8.0 * radius / 25.0;
    let height = 20.0 * radius / 25.0;
    ShapePath::new()
        .move_to(Vec2::new(-width, height))
        .line_to(Vec2::new(0.0, 0.0))
        .line_to(Vec2::new(-width, -height))
        .line_to(Vec2::new(0.0, -height))
        .line_to(Vec2::new(width, 0.0))
        .line_to(Vec2::new(0.0, height))
        .close()
}

// ============================================================================
// Touch-hold countdown arc
// ============================================================================

/// Build the `ShapePath` for a touch-hold countdown arc at a given sweep.
///
/// The arc starts at the top of the circle (12-o'clock, angle = π/2 from +X)
/// and sweeps **clockwise** by `sweep_radians`. A full circle uses `TAU`.
///
/// * `arc_radius`    — radius of the countdown ring.
/// * `sweep_radians` — how much of the circle remains (TAU = full, 0 = empty).
pub fn build_countdown_arc_path(arc_radius: f32, sweep_radians: f32) -> ShapePath {
    let start_x = arc_radius * FRAC_PI_2.cos(); // ≈ 0
    let start_y = arc_radius * FRAC_PI_2.sin(); // ≈ arc_radius (top)
    ShapePath::new().move_to(Vec2::new(start_x, start_y)).arc(
        Vec2::ZERO,
        Vec2::splat(arc_radius),
        -sweep_radians, // negative = clockwise
        0.0,
    )
}

// ── Shape builders ─────────────────────────────────────────────────────────

pub(super) fn tap_shape(assets: &NoteAssets, color: Color) -> Shape {
    ShapeBuilder::with(&assets.tap_path)
        .stroke((color, NOTE_RADIUS * 0.4))
        .build()
}

pub(super) fn hold_arch_shape(assets: &NoteAssets, color: Color) -> Shape {
    ShapeBuilder::with(&assets.hold_arch_path)
        .fill(color)
        .build()
}

pub(super) fn hold_body_shape(assets: &NoteAssets, color: Color) -> Shape {
    ShapeBuilder::with(&assets.hold_body_path)
        .fill(color)
        .build()
}

pub(super) fn touch_circle_shape(assets: &NoteAssets, color: Color) -> Shape {
    ShapeBuilder::with(&assets.touch_circle_path)
        .fill(color)
        .build()
}

pub(super) fn touch_triangle_shape(assets: &NoteAssets, color: Color) -> Shape {
    ShapeBuilder::with(&assets.touch_triangle_path)
        .fill(color)
        .build()
}

pub(super) fn touch_hold_triangle_shape(assets: &NoteAssets, color: Color) -> Shape {
    ShapeBuilder::with(&assets.touch_hold_triangle_path)
        .fill(color)
        .build()
}
