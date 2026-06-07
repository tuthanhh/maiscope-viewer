use super::{RADIUS, component::NoteTiming, resources::ButtonLayout, resources::NoteAssets};
use crate::systems::{
    MOVING,
    chart_playback::ChartPlayback,
    component::{Duration, NoteKind},
    visual::{
        NOTE_RADIUS,
        component::{
            HoldHalo, HoldNoteElement, NoteBpm, SlideArrow, SlideElement, SlidePath, TouchElement,
            TouchHoldCountdown,
        },
        note_colors,
        shapes::{self, hexagon_shape, hold_halo_shape, spark_shape},
        slide_path,
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
type HaloHoldQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Shape,
        &'static mut Transform,
        &'static HoldHalo,
    ),
    (
        Without<NoteTiming>,
        Without<TouchElement>,
        Without<TouchHoldCountdown>,
        Without<SlideArrow>,
        Without<SlideElement>,
        Without<HoldNoteElement>,
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

/// True hold-bar length: how far the note travels during the hold
/// (`velocity · hold_secs`, where `velocity = travel_dist / move_duration`).
fn hold_max_tail(travel_dist: f32, speed: f32, duration: Duration, bpm: f32) -> f32 {
    travel_dist * speed / MOVING as f32 * duration_to_secs(duration, bpm)
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
        Option<&mut Shape>,
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
    mut halo_holds: HaloHoldQuery,
    chart: Res<ChartPlayback>,
    layout: Res<ButtonLayout>,
    time: Res<Time>,
    assets: Res<NoteAssets>,
) {
    let speed = chart.chart_speed * chart.note_speed;
    let move_duration = MOVING as f32 / speed;

    for (
        entity,
        mut shape,
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
                            *timing = NoteTiming::Dying(Timer::from_seconds(0.25, TimerMode::Once));
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
                            // Pop effect where the head star lands on the ring.
                            if let NoteKind::Slide { head_button, .. } = kind {
                                let pos = layout.tap[head_button - 1] * RADIUS;
                                commands.spawn((
                                    hexagon_shape(&assets, note_colors::HEXAGON),
                                    Transform::from_translation(pos.extend(2.5)),
                                    Visibility::Visible,
                                    NoteKind::Tap(*head_button),
                                    NoteTiming::Dying(Timer::from_seconds(0.25, TimerMode::Once)),
                                ));
                            }
                            let wait = slide_path_data.map(|p| p.wait_secs).unwrap_or(0.0);
                            *timing =
                                NoteTiming::Waiting(Timer::from_seconds(wait, TimerMode::Once));
                        }
                    }
                }
            }
            NoteTiming::Holding(timer) => {
                if timer.elapsed().is_zero() {
                    // adding an effect visual, a glowing halo around the head, to make it more visually distinct from a regular tap
                    commands.entity(entity).with_children(|parent| {
                        parent.spawn((hold_halo_shape(&assets, note_colors::HEXAGON), HoldHalo));
                    });
                }
                timer.tick(time.delta());

                let t = timer.fraction();

                match kind {
                    NoteKind::TapHold { .. } => {
                        hold_tap(
                            kind,
                            t,
                            timer,
                            note_bpm.map(|b| b.0).unwrap_or(0.0),
                            speed,
                            children,
                            &mut hold_elements,
                            &mut halo_holds,
                            &layout,
                        );
                    }
                    NoteKind::TouchHold { .. } => {
                        hold_touch(t, timer, children, &mut countdown_query, &mut halo_holds);
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
                    let total = slide_path_data
                        .map(slide_path::trace_total_secs)
                        .unwrap_or(0.0);
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
                    // Slides have no death visual (shapeless container), so Dying
                    // just despawns children + self on its first frame.
                    *timing = NoteTiming::Dying(Timer::from_seconds(0.0, TimerMode::Once));
                }
            }
            NoteTiming::Dying(timer) => {
                // On the very first frame: transform into a hexagon
                if timer.elapsed().is_zero()
                    && !matches!(
                        kind,
                        NoteKind::Slide { .. } | NoteKind::HeadlessSlide { .. }
                    )
                {
                    // Despawn all children (slide stars, hold bodies, touch triangles)
                    // so we only see the hexagon effect
                    if let Some(children) = children {
                        for child in children.iter() {
                            commands.entity(child).despawn();
                        }
                    }
                    // Touch sparks with a cluster of small stars; everything else
                    // (incl. TouchHold) pops a hexagon.
                    if let Some(shape) = shape.as_deref_mut() {
                        *shape = if matches!(kind, NoteKind::Touch { .. }) {
                            spark_shape(&assets, note_colors::TOUCH)
                        } else {
                            hexagon_shape(&assets, note_colors::HEXAGON)
                        };
                    } else {
                        if matches!(kind, NoteKind::TouchHold { .. } | NoteKind::TapHold { .. }) {
                            commands
                                .entity(entity)
                                .insert(hexagon_shape(&assets, note_colors::HEXAGON));
                        }
                    }
                }

                timer.tick(time.delta());
                let t = timer.fraction();

                // Calculate the wave: goes 0.0 -> 1.0 -> 0.0
                let wave = (t * std::f32::consts::PI).sin();

                // 1. Increase and Decrease Scale
                // Base scale is 1.0, expands up to 2.0 at the peak, then shrinks back to 1.0
                // (Tweak the 1.0 multiplier to make the pop bigger or smaller)
                transform.scale = Vec3::splat(1.0 + (wave * 1.0));

                // 2. Fade In and Fade Out
                // Alpha follows the wave exactly (0% -> 100% -> 0%)
                if let Some(shape) = shape.as_deref_mut() {
                    set_alpha(shape, wave);
                }

                // 3. Final Cleanup
                if timer.just_finished() {
                    commands.entity(entity).despawn();
                }
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
                    // Land on the outer rim, coinciding with the path start.
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

fn hold_tap(
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

    let spawn = layout.tap_spawn[button - 1] * RADIUS;
    let hit = layout.tap[button - 1] * RADIUS;

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
            // Halo pulse: a 0.25s sawtooth that repeats for the whole hold —
            // scale ramps 0.75 -> 1.75 and alpha 0.3 -> 0.5, then snaps back.
            let cycle = (timer.elapsed_secs() % 0.25) / 0.25;
            transform.scale = Vec3::splat(0.75 + cycle); // 0.75 -> 1.75
            set_alpha(&mut shape, 0.3 + cycle * 0.2); // 0.3 -> 0.5
        }
    }
}

fn hold_touch(
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
            // Halo pulse: a 0.25s sawtooth that repeats for the whole hold —
            // scale ramps 0.75 -> 1.75 and alpha 0.3 -> 0.5, then snaps back.
            let cycle = (timer.elapsed_secs() % 0.25) / 0.25;
            transform.scale = Vec3::splat(0.75 + cycle); // 0.75 -> 1.75
            set_alpha(&mut shape, 0.3 + cycle * 0.2); // 0.3 -> 0.5
        }
    }
}
