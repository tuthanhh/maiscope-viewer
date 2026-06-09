//! Note lifecycle driver. `update_movement` runs every frame and advances each
//! note through its `NoteTiming` phases (Growing → Moving → Holding/Waiting →
//! Sliding → Dying), delegating per-element animation to [`animation`].

use super::{RADIUS, component::NoteTiming, resources::ButtonLayout, resources::NoteAssets};
use crate::systems::{
    MOVING,
    chart_playback::ChartPlayback,
    component::NoteKind,
    visual::{
        component::{
            FanLanes, HoldHalo, HoldNoteElement, NoteBpm, SlideArrow, SlideElement, SlidePath,
            TouchElement, TouchHoldCountdown,
        },
        note_colors,
        shapes::{hexagon_shape, hold_halo_shape, spark_shape},
        slide_path,
    },
};
use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

mod animation;
use crate::systems::audio::PlayGuideSoundMessage;
use animation::*;

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
        Option<&FanLanes>,
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

    mut guide_sound_messages: MessageWriter<PlayGuideSoundMessage>,
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
        fan_lanes,
    ) in entity_query.iter_mut()
    {
        match &mut *timing {
            NoteTiming::Growing(timer) => {
                timer.tick(time.delta());
                let t = timer.fraction();

                if !is_slide(kind) {
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

                                guide_sound_messages.write(PlayGuideSoundMessage);
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
                if let Some(fan) = fan_lanes {
                    fan_trace(
                        timer.fraction(),
                        fan,
                        children,
                        &mut slide_elements,
                        &mut slide_arrows,
                        &mut commands,
                    );
                } else if let Some(path) = slide_path_data {
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
                if timer.elapsed().is_zero() && !is_slide(kind) {
                    guide_sound_messages.write(PlayGuideSoundMessage);
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
