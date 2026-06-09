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

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    #[default]
    Loading,
    Playing,
}

fn check_if_ready(
    asset_server: Res<AssetServer>,
    note_assets: Res<audio::GameAudioAssets>,
    playback: Res<chart_playback::ChartPlayback>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    // Example: Check if a specific handle inside your resource is loaded.
    // Replace `bgm_handle` with whatever field stores your loaded audio handle.
    let is_bgm_loaded = asset_server.is_loaded_with_dependencies(&note_assets.bgm);
    let is_sfx_loaded = asset_server.is_loaded_with_dependencies(&note_assets.guide_tap);
    let is_chart_ready = !playback.timed_events.is_empty();

    // You can check multiple things here (e.g., textures, charts)
    if is_bgm_loaded && is_sfx_loaded && is_chart_ready {
        // Everything is ready! Transition to the Playing state.
        // This will trigger OnEnter(AppState::Playing) on the next frame.
        next_state.set(AppState::Playing);
    }
}

pub fn register_systems(app: &mut App) {
    // The core game loop will be defined in this function
    app.init_resource::<visual::resources::ButtonLayout>()
        .init_resource::<visual::resources::NoteAssets>()
        .init_resource::<chart_playback::ChartPlayback>()
        .init_state::<AppState>()
        .add_message::<audio::PlayGuideSoundMessage>() // Register the event
        .add_audio_channel::<audio::Bgm>() // Register Background channel
        .add_audio_channel::<audio::Sfx>() // Register Sound Effects channel
        .add_systems(Startup, chart_playback::prepare_chart)
        .add_systems(
            Startup,
            (visual::spawn_judgement_ring, audio::load_audio_assets),
        )
        .add_systems(Update, check_if_ready.run_if(in_state(AppState::Loading)))
        .add_systems(OnEnter(AppState::Playing), audio::start_bgm)
        .add_systems(
            Update,
            (
                visual::next_event,
                visual::update_movement,
                audio::handle_guide_sounds,
            )
                .run_if(in_state(AppState::Playing)),
        );
}
