use std::path::Path;

use bevy::prelude::*;

use crate::systems::chart_playback::ChartPlayback;

mod chart_playback;
mod component;
pub mod parser;
mod visual;

const GROWING: f64 = 1.5;
const MOVING: f64 = 2.0;
const DEFAULT_BPM: f64 = 240.0;

fn prepare_chart(mut playback: ResMut<ChartPlayback>) {
    playback.compute_timestamps(parser::parse_chart(Path::new("test.txt")).unwrap());
}

pub fn register_systems(app: &mut App) {
    // The core game loop will be defined in this function
    app.init_resource::<visual::resources::ButtonLayout>()
        .init_resource::<visual::resources::NoteAssets>()
        .init_resource::<chart_playback::ChartPlayback>()
        .add_systems(Startup, visual::spawn_judgement_ring)
        .add_systems(Startup, prepare_chart)
        .add_systems(Update, (visual::next_event, visual::update_movement));
}
