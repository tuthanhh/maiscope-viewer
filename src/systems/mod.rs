use bevy::prelude::*;

mod chart_playback;
mod component;
mod visual;

const GROWING: f64 = 1.5;
const MOVING: f64 = 2.0;
const DEFAULT_BPM: f64 = 240.0;

pub fn register_systems(app: &mut App) {
    // The core game loop will be defined in this function
    app.init_resource::<visual::resources::ButtonLayout>()
        .init_resource::<visual::resources::NoteAssets>()
        .init_resource::<chart_playback::ChartPlayback>()
        .add_systems(Startup, visual::spawn_judgement_ring)
        .add_systems(Update, (visual::next_event, visual::update_movement));
}
