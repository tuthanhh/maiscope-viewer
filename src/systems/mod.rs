use bevy::prelude::*;

mod visual;

pub fn register_systems(app: &mut App) {
    // The core game loop will be defined in this function
    app.init_resource::<visual::resources::ButtonLayout>()
        .add_systems(Startup, visual::spawn_judgement_ring);
}
