use std::path::Path;

use crate::systems::chart_playback::ChartPlayback;
use bevy::prelude::*;
use bevy_kira_audio::prelude::*;

mod audio;
mod chart_playback;
mod component;
pub mod parser;
mod visual;

const GROWING: f64 = 1.5;
const MOVING: f64 = 2.0;
const DEFAULT_BPM: f64 = 240.0;

fn prepare_chart(mut playback: ResMut<ChartPlayback>) {
    playback.compute_timestamps(
        parser::parse_chart(Path::new("assets/songs/the EmpErroR/maidata.txt")).unwrap(),
    );
}

pub fn register_systems(app: &mut App) {
    // The core game loop will be defined in this function
    app.init_resource::<visual::resources::ButtonLayout>()
        .init_resource::<visual::resources::NoteAssets>()
        .init_resource::<chart_playback::ChartPlayback>()
        .add_message::<audio::PlayGuideSoundMessage>() // Register the event
        .add_audio_channel::<audio::Bgm>() // Register Background channel
        .add_audio_channel::<audio::Sfx>() // Register Sound Effects channel
        .add_systems(Startup, prepare_chart)
        .add_systems(
            Startup,
            (
                visual::spawn_judgement_ring,
                audio::load_audio_assets,
                audio::start_bgm.after(audio::load_audio_assets),
            ),
        )
        .add_systems(
            Update,
            (
                visual::next_event,
                visual::update_movement,
                audio::handle_guide_sounds,
            ),
        );
}
