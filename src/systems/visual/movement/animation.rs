//! Stateless per-element animators driven by `update_movement`. Each function
//! advances one note's visual elements for a given phase fraction `t`; the
//! phase state machine that calls them lives in [`super`].

use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

use crate::systems::{
    MOVING,
    component::{Duration, NoteKind},
    visual::{
        NOTE_RADIUS, RADIUS,
        component::{HoldNoteElement, SlideElement, SlidePath, TouchElement},
        note_colors,
        resources::ButtonLayout,
        shapes, slide_path,
    },
};

use super::{
    CountdownQuery, HaloHoldQuery, HoldElementQuery, SlideArrowQuery, SlideElementQuery,
    TriangleQuery,
};

// ── Small shared helpers ─────────────────────────────────────────────────────

pub(super) fn duration_to_secs(duration: Duration, bpm: f32) -> f32 {
    match duration {
        Duration::Simple { divider, count } => count as f32 / divider as f32 * (240.0 / bpm),
        Duration::BpmOverride {
            bpm,
            divider,
            count,
        } => count as f32 / divider as f32 * (240.0 / bpm),
        Duration::BpmOverrideSeconds { seconds, .. } => seconds,
        _ => 0.0,
    }
}

/// True hold-bar length: how far the note travels during the hold
/// (`velocity · hold_secs`, where `velocity = travel_dist / move_duration`).
fn hold_max_tail(travel_dist: f32, speed: f32, duration: Duration, bpm: f32) -> f32 {
    travel_dist * speed / MOVING as f32 * duration_to_secs(duration, bpm)
}

pub(super) fn set_alpha(shape: &mut Shape, a: f32) {
    if let Some(fill) = shape.fill.as_mut() {
        fill.color.set_alpha(a);
    }
    if let Some(stroke) = shape.stroke.as_mut() {
        stroke.color.set_alpha(a);
    }
}

/// True for slide notes (with or without a head star).
pub(super) fn is_slide(kind: &NoteKind) -> bool {
    matches!(
        kind,
        NoteKind::Slide { .. } | NoteKind::HeadlessSlide { .. }
    )
}

/// `(spawn, hit)` world positions for button `id`: where the note appears and
/// where it lands on the outer ring.
fn travel_endpoints(layout: &ButtonLayout, id: usize) -> (Vec2, Vec2) {
    let spawn = layout.tap_spawn[id - 1] * RADIUS;
    let hit = layout.tap[id - 1] * RADIUS;
    (spawn, hit)
}

/// Halo pulse: a 0.25s sawtooth that repeats for the whole hold — scale ramps
/// 0.75 -> 1.75 and alpha 0.3 -> 0.5, then snaps back.
fn pulse_halo(shape: &mut Shape, transform: &mut Transform, timer: &Timer) {
    let cycle = (timer.elapsed_secs() % 0.25) / 0.25;
    transform.scale = Vec3::splat(0.75 + cycle); // 0.75 -> 1.75
    set_alpha(shape, 0.3 + cycle * 0.2); // 0.3 -> 0.5
}

// ── Growing phase ────────────────────────────────────────────────────────────

pub(super) fn grow_slide(
    t: f32,
    children: Option<&Children>,
    slide_elements: &mut SlideElementQuery,
    slide_arrows: &mut SlideArrowQuery,
) {
    let Some(children) = children else { return };
    for child in children.iter() {
        if let Ok((mut transform, el, _, _)) = slide_elements.get_mut(child) {
            if matches!(*el, SlideElement::Head) {
                transform.scale = Vec3::splat(t);
            }
        }
        if let Ok((mut shape, _)) = slide_arrows.get_mut(child) {
            set_alpha(&mut shape, t);
        }
    }
}

// ── Moving phase ─────────────────────────────────────────────────────────────

pub(super) fn move_tap(transform: &mut Transform, kind: &NoteKind, t: f32, layout: &ButtonLayout) {
    if let NoteKind::Tap(id) | NoteKind::SlideStar(id) = kind {
        let (spawn, hit) = travel_endpoints(layout, *id);
        transform.translation = spawn.lerp(hit, t).extend(2.0);
    }
}

pub(super) fn move_taphold(
    kind: &NoteKind,
    t: f32,
    bpm: f32,
    speed: f32,
    children: Option<&Children>,
    transform: &mut Transform,
    hold_elements: &mut HoldElementQuery,
    layout: &ButtonLayout,
) {
    let NoteKind::TapHold { button, duration } = kind else {
        return;
    };

    let Some(children) = children else { return };

    let (spawn, hit) = travel_endpoints(layout, *button);
    transform.translation = spawn.lerp(hit, t).extend(2.0);

    let travel_dist = spawn.distance(hit);
    let max_tail = hold_max_tail(travel_dist, speed, *duration, bpm);
    let current_length = (travel_dist * t).min(max_tail);

    for child in children.iter() {
        if let Ok((mut tf, el)) = hold_elements.get_mut(child) {
            match el {
                HoldNoteElement::Body => {
                    tf.scale.y = current_length;
                    tf.translation.y = -current_length / 2.0;
                }
                HoldNoteElement::Tail => {
                    tf.translation.y = -current_length;
                }
                HoldNoteElement::Head => {}
            }
        }
    }
}

pub(super) fn move_slide(
    t: f32,
    kind: &NoteKind,
    children: Option<&Children>,
    slide_elements: &mut SlideElementQuery,
    layout: &ButtonLayout,
) {
    let Some(children) = children else { return };
    for child in children.iter() {
        if let Ok((mut transform, el, _, _)) = slide_elements.get_mut(child) {
            if matches!(*el, SlideElement::Head) {
                if let NoteKind::Slide {
                    head_button: id, ..
                } = kind
                {
                    // Land on the outer rim, coinciding with the path start.
                    let (spawn, hit) = travel_endpoints(layout, *id);
                    transform.translation = spawn.lerp(hit, t).extend(2.0);
                }
            }
        }
    }
}

pub(super) fn hide_slide_head(children: Option<&Children>, slide_elements: &mut SlideElementQuery) {
    let Some(children) = children else { return };
    for child in children.iter() {
        if let Ok((_t, el, mut vis, _s)) = slide_elements.get_mut(child) {
            if matches!(*el, SlideElement::Head) {
                *vis = Visibility::Hidden;
            }
        }
    }
}

pub(super) fn move_triangles(
    kind: &NoteKind,
    t: f32,
    children: Option<&Children>,
    triangles: &mut TriangleQuery,
) {
    if !matches!(kind, NoteKind::Touch { .. } | NoteKind::TouchHold { .. }) {
        return;
    }
    let Some(children) = children else { return };
    let current_dist = shapes::touch_triangle_start_distance(NOTE_RADIUS) * (1.0 - t);
    for child in children.iter() {
        if let Ok((mut tf, element)) = triangles.get_mut(child) {
            if matches!(element, TouchElement::Triangle) {
                let dir = tf.translation.truncate().normalize_or_zero();
                tf.translation = (dir * current_dist).extend(-0.1);
            }
        }
    }
}

// ── Holding phase ────────────────────────────────────────────────────────────

pub(super) fn hold_tap(
    kind: &NoteKind,
    t: f32,
    timer: &Timer,
    bpm: f32,
    speed: f32,
    children: Option<&Children>,
    hold_elements: &mut HoldElementQuery,
    halo_holds: &mut HaloHoldQuery,
    layout: &ButtonLayout,
) {
    let NoteKind::TapHold { button, duration } = kind else {
        return;
    };
    let Some(children) = children else { return };

    let (spawn, hit) = travel_endpoints(layout, *button);

    let travel_dist = spawn.distance(hit);
    let max_tail = hold_max_tail(travel_dist, speed, *duration, bpm);
    let current_length = (max_tail * (1.0 - t)).min(travel_dist);

    for child in children.iter() {
        if let Ok((mut tf, el)) = hold_elements.get_mut(child) {
            match el {
                HoldNoteElement::Body => {
                    tf.scale.y = current_length;
                    tf.translation.y = -current_length / 2.0;
                }
                HoldNoteElement::Tail => {
                    tf.translation.y = -current_length;
                }
                HoldNoteElement::Head => {}
            }
        }
        if let Ok((mut shape, mut transform, _)) = halo_holds.get_mut(child) {
            pulse_halo(&mut shape, &mut transform, timer);
        }
    }
}

pub(super) fn hold_touch(
    t: f32,
    timer: &Timer,
    children: Option<&Children>,
    countdown_query: &mut CountdownQuery,
    halo_holds: &mut HaloHoldQuery,
) {
    let Some(children) = children else { return };
    for child in children.iter() {
        if let Ok((mut arc_shape, mut vis, countdown)) = countdown_query.get_mut(child) {
            *vis = Visibility::Visible;
            let r = countdown.arc_radius;
            let new_path = shapes::build_countdown_path(r, t);
            arc_shape.path = ShapeBuilder::with(&new_path)
                .stroke((note_colors::RING, NOTE_RADIUS * 0.18))
                .build()
                .path;
        }
        if let Ok((mut shape, mut transform, _)) = halo_holds.get_mut(child) {
            pulse_halo(&mut shape, &mut transform, timer);
        }
    }
}

// ── Waiting / Sliding phases ─────────────────────────────────────────────────

/// Waiting phase: the trace star fades in, stationary at the path start.
pub(super) fn wait_slide(
    t: f32,
    children: Option<&Children>,
    slide_elements: &mut SlideElementQuery,
) {
    let Some(children) = children else { return };
    for child in children.iter() {
        if let Ok((_t, el, mut vis, mut shape)) = slide_elements.get_mut(child) {
            if matches!(*el, SlideElement::TraceStar) {
                *vis = Visibility::Visible;
                set_alpha(&mut shape, t);
            }
        }
    }
}

/// Sliding phase: the trace star walks the path; chevrons it passes are removed.
pub(super) fn slide_trace(
    elapsed: f32,
    path: &SlidePath,
    children: Option<&Children>,
    slide_elements: &mut SlideElementQuery,
    slide_arrows: &mut SlideArrowQuery,
    commands: &mut Commands,
) {
    let Some(children) = children else { return };
    let dist = slide_path::trace_distance(path, elapsed);
    let (pos, _angle) = slide_path::get_transform_at_distance(&path.waypoints, dist);
    for child in children.iter() {
        if let Ok((mut transform, el, mut vis, mut shape)) = slide_elements.get_mut(child) {
            if matches!(*el, SlideElement::TraceStar) {
                *vis = Visibility::Visible;
                set_alpha(&mut shape, 1.0);
                transform.translation = pos.extend(3.0);
            }
        }
        if let Ok((_s, arrow)) = slide_arrows.get_mut(child) {
            if arrow.distance_along_path <= dist {
                commands.entity(child).despawn();
            }
        }
    }
}
