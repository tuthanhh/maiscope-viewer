use super::{RADIUS, component::NoteTiming, resources::ButtonLayout};
use crate::systems::{
    MOVING,
    chart_playback::ChartPlayback,
    component::{Duration, NoteKind},
    visual::{
        NOTE_RADIUS,
        component::{
            HoldNoteElement, NoteBpm, SlideArrow, SlideElement, SlidePath, TouchElement,
            TouchHoldCountdown,
        },
        note_colors, shapes, slide_path,
    },
};
use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

type TriangleQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, &'static TouchElement),
    (
        Without<NoteTiming>,
        Without<HoldNoteElement>,
        Without<TouchHoldCountdown>,
        Without<SlideArrow>,
    ),
>;

type HoldElementQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, &'static HoldNoteElement),
    (
        Without<NoteTiming>,
        Without<TouchElement>,
        Without<TouchHoldCountdown>,
        Without<SlideArrow>,
        Without<SlideElement>,
    ),
>;

type CountdownQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Shape,
        &'static mut Visibility,
        &'static TouchHoldCountdown,
    ),
    (
        Without<NoteTiming>,
        Without<TouchElement>,
        Without<TouchHoldCountdown>,
        Without<SlideArrow>,
    ),
>;

type SlideElementQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Transform,
        &'static SlideElement,
        &'static mut Visibility,
        &'static mut Shape,
    ),
    (
        Without<NoteTiming>,
        Without<TouchElement>,
        Without<TouchHoldCountdown>,
        Without<SlideArrow>,
        Without<HoldNoteElement>,
    ),
>;
type SlideArrowQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Shape, &'static mut SlideArrow),
    (
        Without<NoteTiming>,
        Without<TouchElement>,
        Without<TouchHoldCountdown>,
        Without<SlideElement>,
        Without<HoldNoteElement>,
    ),
>;

fn duration_to_secs(duration: Duration, bpm: f32) -> f32 {
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

fn set_alpha(shape: &mut Shape, a: f32) {
    if let Some(fill) = shape.fill.as_mut() {
        fill.color.set_alpha(a);
    }
    if let Some(stroke) = shape.stroke.as_mut() {
        stroke.color.set_alpha(a);
    }
}

pub fn update_movement(
    mut commands: Commands,
    mut entity_query: Query<(
        Entity,
        &mut Transform,
        &mut NoteTiming,
        &NoteKind,
        &mut Visibility,
        Option<&Children>,
        Option<&NoteBpm>,
        Option<&SlidePath>,
    )>,
    mut triangles: TriangleQuery,
    mut hold_elements: HoldElementQuery,
    mut countdown_query: CountdownQuery,
    mut slide_elements: SlideElementQuery,
    mut slide_arrows: SlideArrowQuery,
    chart: Res<ChartPlayback>,
    layout: Res<ButtonLayout>,
    time: Res<Time>,
) {
    let speed = chart.chart_speed * chart.note_speed;
    let move_duration = MOVING as f32 / speed;

    for (
        entity,
        mut transform,
        mut timing,
        kind,
        mut visibility,
        children,
        note_bpm,
        slide_path_data,
    ) in entity_query.iter_mut()
    {
        match &mut *timing {
            NoteTiming::Growing(timer) => {
                timer.tick(time.delta());
                let t = timer.fraction();

                if !matches!(
                    kind,
                    NoteKind::Slide { .. } | NoteKind::HeadlessSlide { .. }
                ) {
                    transform.scale = Vec3::splat(t);
                } else {
                    grow_slide(t, children, &mut slide_elements, &mut slide_arrows);
                }

                if timer.just_finished() {
                    *timing =
                        NoteTiming::Moving(Timer::from_seconds(move_duration, TimerMode::Once));
                    if matches!(kind, NoteKind::Touch { .. } | NoteKind::TouchHold { .. }) {
                        *visibility = Visibility::Visible;
                    }
                }
            }
            NoteTiming::Moving(timer) => {
                timer.tick(time.delta());
                let t = timer.fraction();

                move_tap(&mut transform, kind, t, &layout);
                move_triangles(kind, t, children, &mut triangles);
                move_slide(t, kind, children, &mut slide_elements, &layout);
                move_taphold(
                    kind,
                    t,
                    note_bpm.map(|b| b.0).unwrap_or(0.0),
                    speed,
                    children,
                    &mut transform,
                    &mut hold_elements,
                    &layout,
                );

                if timer.just_finished() {
                    match kind {
                        NoteKind::Tap(_) | NoteKind::Touch { .. } | NoteKind::SlideStar { .. } => {
                            *timing = NoteTiming::Dying(Timer::from_seconds(0.1, TimerMode::Once));
                        }
                        NoteKind::TapHold { duration, .. }
                        | NoteKind::TouchHold { duration, .. } => {
                            if let Some(NoteBpm(bpm)) = note_bpm {
                                *timing = NoteTiming::Holding(Timer::from_seconds(
                                    duration_to_secs(*duration, *bpm),
                                    TimerMode::Once,
                                ));
                            }
                        }
                        NoteKind::Slide { .. } | NoteKind::HeadlessSlide { .. } => {
                            hide_slide_head(children, &mut slide_elements);
                            let wait = slide_path_data.map(|p| p.wait_secs).unwrap_or(0.0);
                            *timing =
                                NoteTiming::Waiting(Timer::from_seconds(wait, TimerMode::Once));
                        }
                    }
                }
            }
            NoteTiming::Holding(timer) => {
                timer.tick(time.delta());
                let t = timer.fraction();
                match kind {
                    NoteKind::TapHold { .. } => {
                        hold_tap(
                            kind,
                            t,
                            note_bpm.map(|b| b.0).unwrap_or(0.0),
                            speed,
                            children,
                            &mut hold_elements,
                            &layout,
                        );
                    }
                    NoteKind::TouchHold { .. } => {
                        hold_touch(t, children, &mut countdown_query);
                    }
                    _ => {}
                }

                if timer.just_finished() {
                    *timing = NoteTiming::Dying(Timer::from_seconds(0.1, TimerMode::Once));
                }
            }
            NoteTiming::Waiting(timer) => {
                timer.tick(time.delta());
                wait_slide(timer.fraction(), children, &mut slide_elements);
                if timer.just_finished() {
                    let total = slide_path_data.map(slide_path::trace_total_secs).unwrap_or(0.0);
                    *timing = NoteTiming::Sliding(Timer::from_seconds(total, TimerMode::Once));
                }
            }
            NoteTiming::Sliding(timer) => {
                timer.tick(time.delta());
                let elapsed = timer.elapsed_secs();
                if let Some(path) = slide_path_data {
                    slide_trace(
                        elapsed,
                        path,
                        children,
                        &mut slide_elements,
                        &mut slide_arrows,
                        &mut commands,
                    );
                }
                if timer.just_finished() {
                    *timing = NoteTiming::Dying(Timer::from_seconds(0.1, TimerMode::Once));
                }
            }
            NoteTiming::Dying(timer) => {
                if timer.elapsed().is_zero() {
                    // Play the guide sound
                    commands.entity(entity).despawn();
                }
                timer.tick(time.delta());
                // Play the after effect
            }
        }
    }
}
fn grow_slide(
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

// Moving phase
fn move_tap(transform: &mut Transform, kind: &NoteKind, t: f32, layout: &ButtonLayout) {
    if let NoteKind::Tap(id) | NoteKind::SlideStar(id) = kind {
        let spawn = layout.tap_spawn[id - 1] * RADIUS;
        let hit = layout.tap[id - 1] * RADIUS;
        transform.translation = spawn.lerp(hit, t).extend(2.0);
    }
}
// Moving phase
fn move_taphold(
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

    let spawn = layout.tap_spawn[button - 1] * RADIUS;
    let hit = layout.tap[button - 1] * RADIUS;
    transform.translation = spawn.lerp(hit, t).extend(2.0);

    let travel_dist = spawn.distance(hit);
    let max_tail = travel_dist * speed / MOVING as f32 * duration_to_secs(*duration, bpm);
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

fn move_slide(
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
                    let spawn = layout.tap_spawn[id - 1] * RADIUS;
                    let hit = layout.tap[id - 1] * RADIUS;
                    transform.translation = spawn.lerp(hit, t).extend(2.0);
                }
            }
        }
    }
}

fn hide_slide_head(children: Option<&Children>, slide_elements: &mut SlideElementQuery) {
    let Some(children) = children else { return };
    for child in children.iter() {
        if let Ok((_t, el, mut vis, _s)) = slide_elements.get_mut(child) {
            if matches!(*el, SlideElement::Head) {
                *vis = Visibility::Hidden;
            }
        }
    }
}

// Waiting phase: the trace star fades in, stationary at the path start.
fn wait_slide(t: f32, children: Option<&Children>, slide_elements: &mut SlideElementQuery) {
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

// Sliding phase: the trace star walks the path; chevrons it passes are removed.
fn slide_trace(
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
fn move_triangles(
    kind: &NoteKind,
    t: f32,
    children: Option<&Children>,
    triangles: &mut TriangleQuery,
) {
    if !matches!(kind, NoteKind::Touch { .. } | NoteKind::TouchHold { .. }) {
        return;
    }
    let Some(children) = children else { return };
    let current_dist = 0.65 * NOTE_RADIUS * (1.0 - t);
    for child in children.iter() {
        if let Ok((mut tf, element)) = triangles.get_mut(child) {
            if matches!(element, TouchElement::Triangle) {
                let dir = tf.translation.truncate().normalize_or_zero();
                tf.translation = (dir * current_dist).extend(-0.1);
            }
        }
    }
}

fn hold_tap(
    kind: &NoteKind,
    t: f32,
    bpm: f32,
    speed: f32,
    children: Option<&Children>,
    hold_elements: &mut HoldElementQuery,
    layout: &ButtonLayout,
) {
    let NoteKind::TapHold { button, duration } = kind else {
        return;
    };
    let Some(children) = children else { return };

    let spawn = layout.tap_spawn[button - 1] * RADIUS;
    let hit = layout.tap[button - 1] * RADIUS;

    let travel_dist = spawn.distance(hit);
    let max_tail = travel_dist * speed / MOVING as f32 * duration_to_secs(*duration, bpm);
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
    }
}

fn hold_touch(t: f32, children: Option<&Children>, countdown_query: &mut CountdownQuery) {
    let Some(children) = children else { return };
    for child in children.iter() {
        if let Ok((mut arc_shape, mut vis, countdown)) = countdown_query.get_mut(child) {
            *vis = Visibility::Visible;
            let r = countdown.arc_radius;
            let new_path = shapes::build_countdown_path(r, t);
            arc_shape.path = ShapeBuilder::with(&new_path)
                .stroke((note_colors::RING, r * 0.15))
                .build()
                .path;
        }
    }
}
