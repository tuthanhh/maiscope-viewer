// ── ECS visual components ──────────────────────────────────────────────────

use crate::systems::component::SlideDuration;
use bevy::prelude::*;

#[derive(Component)]
pub enum NoteTiming {
    Growing(Timer),
    Moving(Timer),
    Holding(Timer, f32),
    Waiting(Timer),
    Sliding(Timer),
    Dying(Timer),
}

#[derive(Component)]
pub enum HoldNoteElement {
    Head,
    Body,
    Tail,
}

#[derive(Component)]
pub enum TouchElement {
    Center,
    Triangle,
}

#[derive(Component)]
pub struct TouchHoldCountdown {
    pub arc_radius: f32,
}

#[derive(Component)]
pub struct SlidePath {
    pub waypoints: Vec<Vec2>,
    pub total_length: f32,
    pub track_entity: Option<Entity>,
}

#[derive(Component)]
pub struct SlideArrow {
    pub distance_along_path: f32,
}
