use crate::systems::visual::shapes;
use bevy::prelude::*;
use bevy_prototype_lyon::prelude::ShapePath;

#[derive(Resource, Debug, Clone)]
pub struct ButtonLayout {
    pub tap: Vec<Vec2>,
    pub a: Vec<Vec2>,
    pub b: Vec<Vec2>,
    pub c: Vec<Vec2>,
    pub d: Vec<Vec2>,
    pub e: Vec<Vec2>,
    pub tap_spawn: Vec<Vec2>,
}

impl Default for ButtonLayout {
    fn default() -> Self {
        let mut tap = Vec::new();
        let mut a = Vec::new();
        let mut b = Vec::new();
        let mut c = Vec::new();
        let mut d = Vec::new();
        let mut e = Vec::new();
        let mut tap_spawn = Vec::new();
        for i in 0..8 {
            let a1 = std::f32::consts::FRAC_PI_8 + i as f32 * std::f32::consts::FRAC_PI_4;
            let a2 = i as f32 * std::f32::consts::FRAC_PI_4;
            tap.push(Vec2::new(1.0 * a1.cos(), 1.0 * a1.sin()));
            a.push(Vec2::new(4.1 / 4.8 * a1.cos(), 4.1 / 4.8 * a1.sin()));
            d.push(Vec2::new(4.1 / 4.8 * a2.cos(), 4.1 / 4.8 * a2.sin()));
            b.push(Vec2::new(2.3 / 4.8 * a1.cos(), 2.3 / 4.8 * a1.sin()));
            e.push(Vec2::new(3.0 / 4.8 * a2.cos(), 3.0 / 4.8 * a2.sin()));
            tap_spawn.push(Vec2::new(1.225 / 4.8 * a1.cos(), 1.225 / 4.8 * a1.sin()));
        }
        for _ in 0..3 {
            c.push(Vec2::ZERO);
        }

        Self {
            tap,
            a,
            b,
            c,
            d,
            e,
            tap_spawn,
        }
    }
}

/// Shared visual assets for all note types.
///
/// Every field is a [`ShapePath`] — the spawning system turns it into a
/// concrete [`Shape`] with the right color via `ShapeBuilder::with(&path)`.
#[derive(Resource)]
pub struct NoteAssets {
    // ── Note outlines ──────────────────────────────────────────────────
    /// Tap note ring (full-circle arc, stroked).
    pub tap_path: ShapePath,

    /// Hold note head/tail (half-circle arc, stroked).
    pub hold_arch_path: ShapePath,

    /// Hold note body beam (rectangle, stroked; scaled in Y at spawn).
    pub hold_body_path: ShapePath,

    /// Slide note star (closed polygon, stroked).
    pub slide_star_path: ShapePath,

    /// Touch note centre dot (full circle, filled).
    pub touch_circle_path: ShapePath,

    /// Approach triangle for regular Touch notes (hollow triangle, stroked).
    pub touch_triangle_path: ShapePath,

    /// Approach triangle for TouchHold notes (filled triangle).
    pub touch_hold_triangle_path: ShapePath,

    /// Slide track chevron arrow (closed polygon, stroked).
    pub chevron_path: ShapePath,

    /// TouchHold countdown ring (full-circle arc, stroked; sweep shrinks at runtime).
    pub countdown_arc_path: ShapePath,
}

impl Default for NoteAssets {
    fn default() -> Self {
        let radius = super::NOTE_RADIUS;
        Self {
            tap_path: shapes::build_tap_ring_path(radius),
            hold_arch_path: shapes::build_hold_arch_path(radius),
            hold_body_path: shapes::build_hold_body_path(radius * 2.0),
            slide_star_path: shapes::build_slide_star_path(radius, 0.55, 5),
            touch_circle_path: shapes::build_touch_circle_path(radius),
            touch_triangle_path: shapes::build_touch_triangle_path(radius),
            touch_hold_triangle_path: shapes::build_touch_hold_triangle_path(radius),
            chevron_path: shapes::build_chevron_path(radius),
            countdown_arc_path: shapes::build_countdown_arc_path(radius * 1.15, std::f32::consts::TAU),
        }
    }
}
