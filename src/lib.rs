pub struct AppPlugin;
use bevy::prelude::{App, Plugin};

mod plugins;
mod systems;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(plugins::register_plugins);
        systems::register_systems(app);
    }
}
