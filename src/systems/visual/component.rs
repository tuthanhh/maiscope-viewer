// ── ECS visual components ──────────────────────────────────────────────────

use bevy::prelude::*;

#[derive(Component)]
pub enum NoteTiming {
    Growing(Timer),
    Moving(Timer),
    Holding(Timer),
    Waiting(Timer),
    Sliding(Timer),
    Dying(Timer),
}

// ------------------------
// Special visual element
// ------------------------
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
    /// `(cumulative_trace_time, cumulative_distance)` at each segment end.
    /// Drives the time→distance mapping for the tracing star.
    pub breakpoints: Vec<(f32, f32)>,
    /// Seconds the slide waits (Waiting phase) before the trace begins.
    pub wait_secs: f32,
}

/// A chevron arrow sitting at `distance_along_path` units along the slide track.
/// `lane` is the fan-lane index (0 for ordinary single-lane slides).
#[derive(Component)]
pub struct SlideArrow {
    pub distance_along_path: f32,
    pub lane: usize,
}

/// The diverging lanes of a fan (`w`) slide: one waypoint list + length per end.
#[derive(Component)]
pub struct FanLanes {
    pub lanes: Vec<Vec<Vec2>>,
    pub lengths: Vec<f32>,
}

/// Glowing halo child spawned around a hold head during the Holding phase.
#[derive(Component)]
pub struct HoldHalo;

/// Marks the two star visuals that belong to a slide note.
#[derive(Component)]
pub enum SlideElement {
    /// Initial star-tap that approaches the judgment ring, then vanishes (Slide only).
    Head,
    /// Star that traces a path during the Sliding phase. The payload is the
    /// fan-lane index (0 for ordinary single-lane slides).
    TraceStar(usize),
}

#[derive(Component)]
pub struct NoteBpm(pub f32);
