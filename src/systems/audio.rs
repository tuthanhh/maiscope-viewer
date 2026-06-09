// systems/audio.rs
use bevy::prelude::*;
use bevy_kira_audio::prelude::*;

// 1. Derive Message instead of Event
#[derive(Message)]
pub struct PlayGuideSoundMessage;

#[derive(Resource)]
pub struct Bgm;

#[derive(Resource)]
pub struct Sfx;

#[derive(Resource)]
pub struct GameAudioAssets {
    pub bgm: Handle<AudioSource>,
    pub guide_tap: Handle<AudioSource>,
}

pub fn load_audio_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(GameAudioAssets {
        bgm: asset_server.load("songs/the EmpErroR/track.mp3"),
        guide_tap: asset_server.load("system_sounds/SE_GAME_ANSWER_.wav"),
    });
}
// In systems/audio.rs

pub fn start_bgm(bgm_channel: Res<AudioChannel<Bgm>>, audio_assets: Res<GameAudioAssets>) {
    // We play the BGM and usually lower the volume slightly
    // so the guide sounds and hit SFX can punch through clearly.
    bgm_channel.play(audio_assets.bgm.clone()).with_volume(0.8);
}
pub fn handle_guide_sounds(
    // 2. Use MessageReader
    mut messages: MessageReader<PlayGuideSoundMessage>,
    sfx_channel: Res<AudioChannel<Sfx>>,
    audio_assets: Res<GameAudioAssets>,
) {
    let hit_count = messages.read().count();

    if hit_count > 0 {
        let volume_multiplier: f32 = 1.0 + (hit_count as f32 - 1.0) * 0.1;
        let final_volume: f32 = (1.0 * volume_multiplier).min(1.5);

        sfx_channel
            .play(audio_assets.guide_tap.clone())
            .with_volume(final_volume);
    }
}
