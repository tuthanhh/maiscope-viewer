mod audio;
mod camera;
mod lyon_shape;
use bevy::{prelude::*, window::WindowResolution};

pub fn register_plugins(app: &mut App) {
    let resolution: WindowResolution = (1000, 800).into();

    app.insert_resource(ClearColor(Color::linear_rgb(0.0, 0.0, 0.0)))
        .add_plugins((
            // Default plugins with window configuration
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "simai player".into(),
                    resizable: false,
                    resolution,
                    canvas: Some("#bevy".to_owned()),
                    desired_maximum_frame_latency: core::num::NonZero::new(1u32),
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            }),
            audio::plugin,
            camera::plugin,
            lyon_shape::plugin,
        ));
}
