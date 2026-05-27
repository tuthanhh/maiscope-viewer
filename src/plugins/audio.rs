use bevy::prelude::*;
use bevy_kira_audio::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(AudioPlugin);
}
