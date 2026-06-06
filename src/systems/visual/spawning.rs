use super::{
    NOTE_RADIUS,
    component::{
        HoldNoteElement, NoteBpm, NoteTiming, SlideArrow, SlideElement, TouchElement,
        TouchHoldCountdown,
    },
    note_colors,
    resources::{ButtonLayout, NoteAssets},
    shapes::{
        chevron_shape, hold_arch_shape, hold_body_shape, star_shape, tap_shape, touch_circle_shape,
        touch_hold_triangle_shape, touch_triangle_shape,
    },
    slide_path,
};
use crate::systems::{
    GROWING,
    chart_playback::ChartPlayback,
    component::{ChartEvent, Note, NoteKind},
};
use bevy::ecs::hierarchy::ChildOf;
use bevy::ecs::relationship::RelatedSpawnerCommands;
use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

// ── Color helpers ──────────────────────────────────────────────────────────

fn note_color(is_paired: bool, base: Color) -> Color {
    if is_paired { note_colors::PAIRED } else { base }
}

fn tap_color(is_paired: bool) -> Color {
    note_color(is_paired, note_colors::TAP)
}
fn slide_color(is_paired: bool) -> Color {
    note_color(is_paired, note_colors::SLIDE)
}

// ── Main system ────────────────────────────────────────────────────────────

pub fn next_event(
    mut commands: Commands,
    mut chart: ResMut<ChartPlayback>,
    time: Res<Time>,
    note_assets: Res<NoteAssets>,
    layout: Res<ButtonLayout>,
) {
    if !chart.is_playing {
        return;
    }

    chart.elapsed_time += time.delta_secs_f64() * chart.chart_speed as f64;

    // .map() clones event data and releases the &mut borrow on chart,
    // so chart.chart_speed / chart.note_speed are accessible in the loop body.
    while let Some((event, bpm)) = chart.advance().map(|e| (e.event.clone(), e.bpm)) {
        if let ChartEvent::NoteGroup(notes) = event {
            let is_paired = notes.len() >= 2;
            for note in &notes {
                spawn_note(
                    &mut commands,
                    note,
                    is_paired,
                    bpm,
                    chart.chart_speed,
                    chart.note_speed,
                    &note_assets,
                    &layout,
                );
            }
        }
    }

    if chart.next_spawn_index >= chart.timed_events.len() {
        chart.is_playing = false;
    }
}

// ── Per-note spawning ──────────────────────────────────────────────────────

fn spawn_note(
    commands: &mut Commands,
    note: &Note,
    is_paired: bool,
    bpm: f32,
    chart_speed: f32,
    note_speed: f32,
    assets: &NoteAssets,
    layout: &ButtonLayout,
) {
    let growing_time = GROWING as f32 / (chart_speed * note_speed);

    let mut e = commands.spawn((
        note.kind.clone(),
        NoteTiming::Growing(Timer::from_seconds(growing_time, TimerMode::Once)),
        NoteBpm(bpm),
    ));

    match &note.kind {
        NoteKind::Tap(id) => {
            let pos = tap_pos(*id, layout) * super::RADIUS;
            e.insert((
                tap_shape(assets, tap_color(is_paired)),
                Transform::from_translation(pos.extend(2.0)).with_scale(Vec3::ZERO),
            ));
        }

        NoteKind::TapHold { button, .. } => {
            let pos = tap_pos(*button, layout) * super::RADIUS;
            let dir = layout.tap[*button - 1];
            let angle = dir.y.atan2(dir.x) - FRAC_PI_2;
            e.insert((
                Visibility::default(),
                Transform::from_translation(pos.extend(2.0))
                    .with_rotation(Quat::from_rotation_z(angle))
                    .with_scale(Vec3::ZERO),
            ));
            e.with_children(|p| spawn_hold_children(p, assets, tap_color(is_paired)));
        }

        NoteKind::Touch { value, group } => {
            let pos = touch_pos(*value, *group, layout) * super::RADIUS;
            e.insert((
                touch_circle_shape(assets, slide_color(is_paired)),
                Transform::from_translation(pos.extend(2.0)),
                TouchElement::Center,
                Visibility::Hidden,
            ));
            e.with_children(|p| spawn_approach_triangles(p, is_paired, false, assets));
        }

        NoteKind::TouchHold { value, group, .. } => {
            let pos = touch_pos(*value, *group, layout) * super::RADIUS;
            e.insert((
                touch_circle_shape(assets, slide_color(is_paired)),
                Transform::from_translation(pos.extend(2.0)),
                TouchElement::Center,
                Visibility::Hidden,
            ));
            e.with_children(|p| {
                spawn_approach_triangles(p, is_paired, true, assets);
                spawn_touch_hold_countdown(p, assets);
            });
        }

        // Standalone star (no path): behaves like a tap, parent carries the star.
        NoteKind::SlideStar(button) => {
            let pos = tap_pos(*button, layout) * super::RADIUS;
            e.insert((
                star_shape(assets, slide_color(is_paired)),
                Transform::from_translation(pos.extend(2.0)).with_scale(Vec3::ZERO),
            ));
        }

        // Slide / HeadlessSlide: parent is an origin container; head star, trace
        // star, and chevrons live as children at absolute world positions.
        NoteKind::Slide {
            head_button,
            segments,
            shared_duration,
        }
        | NoteKind::HeadlessSlide {
            start_button: head_button,
            segments,
            shared_duration,
        } => {
            let path = slide_path::build_slide_trace(
                segments,
                *head_button,
                bpm,
                *shared_duration,
                super::RADIUS,
            );
            let has_head = matches!(note.kind, NoteKind::Slide { .. });
            let color = slide_color(is_paired);
            let head = *head_button;

            e.insert((Transform::default(), Visibility::Visible));
            e.with_children(|p| {
                spawn_slide_children(p, &path, has_head, head, layout, assets, color);
            });
            e.insert(path);
        }
    }
}

// ── Transform helpers ──────────────────────────────────────────────────────

fn tap_pos(value: usize, layout: &ButtonLayout) -> Vec2 {
    layout.tap_spawn[value - 1]
}

fn touch_pos(value: usize, group: char, layout: &ButtonLayout) -> Vec2 {
    match group.to_ascii_uppercase() {
        'C' => layout.c[value - 1],
        'B' => layout.b[value - 1],
        'A' => layout.a[value - 1],
        'D' => layout.d[value - 1],
        'E' => layout.e[value - 1],
        _ => Vec2::ZERO,
    }
}

// ── Child entity spawners ──────────────────────────────────────────────────

fn spawn_hold_children(
    parent: &mut RelatedSpawnerCommands<ChildOf>,
    assets: &NoteAssets,
    color: Color,
) {
    parent.spawn((hold_arch_shape(assets, color), HoldNoteElement::Head));
    parent.spawn((
        hold_body_shape(assets, color),
        Transform::from_xyz(0.0, -0.001, 0.0).with_scale(Vec3::new(1.0, 0.001, 1.0)),
        HoldNoteElement::Body,
    ));
    parent.spawn((
        hold_arch_shape(assets, color),
        Transform::from_xyz(0.0, -0.001, 0.0).with_rotation(Quat::from_rotation_z(PI)),
        HoldNoteElement::Tail,
    ));
}

fn spawn_touch_hold_countdown(parent: &mut RelatedSpawnerCommands<ChildOf>, assets: &NoteAssets) {
    let arc_radius = NOTE_RADIUS * 1.15;
    parent.spawn((
        ShapeBuilder::with(&assets.countdown_arc_path)
            .stroke((note_colors::RING, NOTE_RADIUS * 0.18))
            .build(),
        Transform::from_xyz(0.0, 0.0, 5.0),
        Visibility::Hidden,
        TouchHoldCountdown { arc_radius },
    ));
}

fn spawn_approach_triangles(
    parent: &mut RelatedSpawnerCommands<ChildOf>,
    is_paired: bool,
    is_hold: bool,
    assets: &NoteAssets,
) {
    let dist = NOTE_RADIUS * 0.65;

    if !is_hold {
        for dir in [Vec2::Y, Vec2::NEG_Y, Vec2::NEG_X, Vec2::X] {
            parent.spawn((
                touch_triangle_shape(assets, slide_color(is_paired)),
                Transform::from_translation((dir * dist).extend(-0.1))
                    .with_rotation(Quat::from_rotation_z(dir.y.atan2(dir.x) - FRAC_PI_2)),
                TouchElement::Triangle,
            ));
        }
    } else {
        let dirs = [
            Vec2::ONE.normalize(),
            Vec2::new(1.0, -1.0).normalize(),
            Vec2::NEG_ONE.normalize(),
            Vec2::new(-1.0, 1.0).normalize(),
        ];
        for (idx, dir) in dirs.iter().enumerate() {
            parent.spawn((
                touch_hold_triangle_shape(assets, note_colors::TOUCH_HOLD_DIRS[idx]),
                Transform::from_translation((dir * dist).extend(-0.1))
                    .with_rotation(Quat::from_rotation_z(dir.y.atan2(dir.x) - FRAC_PI_2)),
                TouchElement::Triangle,
            ));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_slide_children(
    parent: &mut RelatedSpawnerCommands<ChildOf>,
    path: &super::component::SlidePath,
    has_head: bool,
    head_button: usize,
    layout: &ButtonLayout,
    assets: &NoteAssets,
    color: Color,
) {
    // Head star (Slide only): spawns at the head button, scale ZERO so it grows
    // during the Growing phase, then approaches the ring during Moving.
    if has_head {
        let pos = layout.tap_spawn[head_button - 1] * super::RADIUS;
        parent.spawn((
            star_shape(assets, color),
            Transform::from_translation(pos.extend(2.0)).with_scale(Vec3::ZERO),
            SlideElement::Head,
        ));
    }

    // Trace star: hidden at the path start; revealed and faded in during Waiting,
    // then moved along the path during Sliding.
    let start = path.waypoints.first().copied().unwrap_or(Vec2::ZERO);
    parent.spawn((
        star_shape(assets, color),
        Transform::from_translation(start.extend(3.0)),
        Visibility::Hidden,
        SlideElement::TraceStar,
    ));

    // Chevron arrows spaced along the path, each rotated to face the travel
    // direction. Their alpha fades in during Growing; each is removed as the
    // trace star passes it during Sliding.
    let mut d = super::CHEVRON_SPACING;
    while d < path.total_length {
        let (pos, angle) = slide_path::get_transform_at_distance(&path.waypoints, d);
        parent.spawn((
            chevron_shape(assets, color.with_alpha(0.0)),
            Transform::from_translation(pos.extend(1.0))
                .with_rotation(Quat::from_rotation_z(angle)),
            SlideArrow {
                distance_along_path: d,
            },
        ));
        d += super::CHEVRON_SPACING;
    }
}
