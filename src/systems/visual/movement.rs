use super::{RADIUS, component::NoteTiming, resources::ButtonLayout};
use crate::systems::{
    MOVING,
    chart_playback::ChartPlayback,
    component::NoteKind,
    visual::{
        NOTE_RADIUS,
        component::{HoldNoteElement, SlideArrow, TouchElement, TouchHoldCountdown},
    },
};
use bevy::prelude::*;

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

pub fn update_movement(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &mut Transform,
        &mut NoteTiming,
        &NoteKind,
        &mut Visibility,
        Option<&Children>,
    )>,
    mut triangles: TriangleQuery,
    chart: Res<ChartPlayback>,
    layout: Res<ButtonLayout>,
    time: Res<Time>,
) {
    let speed = chart.chart_speed * chart.note_speed;
    let move_duration = MOVING as f32 / speed;

    for (entity, mut transform, mut timing, kind, mut visibility, children) in query.iter_mut() {
        match &mut *timing {
            NoteTiming::Growing(timer) => {
                timer.tick(time.delta());
                transform.scale = Vec3::splat(timer.fraction());
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

                if timer.just_finished() {
                    commands.entity(entity).despawn();
                }
            }
            _ => {}
        }
    }
}

fn move_tap(transform: &mut Transform, kind: &NoteKind, t: f32, layout: &ButtonLayout) {
    if let Some(id) = tap_button(kind) {
        let spawn = layout.tap_spawn[id] * RADIUS;
        let hit = layout.tap[id] * RADIUS;
        transform.translation = spawn.lerp(hit, t).extend(2.0);
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

fn tap_button(kind: &NoteKind) -> Option<usize> {
    match kind {
        NoteKind::Tap(id) | NoteKind::TapHold { button: id, .. } => Some(*id),
        _ => None,
    }
}
