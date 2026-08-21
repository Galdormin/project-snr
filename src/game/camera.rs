//! Code for the main camera

use bevy::camera::Hdr;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

use crate::{game::drone::Drone, screens::Screen};

/// Camera zoom factor: values below 1.0 zoom in, above 1.0 zoom out.
const CAMERA_ZOOM: f32 = 0.5;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_camera);
    app.add_systems(
        PostUpdate,
        sync_camera_to_drone
            .after(TransformSystems::Propagate)
            .run_if(in_state(Screen::Gameplay)),
    );
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Camera"),
        Camera2d,
        Hdr,
        Tonemapping::TonyMcMapface,
        Bloom::NATURAL,
        Projection::Orthographic(OrthographicProjection {
            scale: CAMERA_ZOOM,
            ..OrthographicProjection::default_2d()
        }),
    ));
}

/// Keeps the camera locked on the drone's position and facing, so it translates
/// and rotates together with it.
fn sync_camera_to_drone(
    drone: Single<&GlobalTransform, With<Drone>>,
    mut camera: Single<&mut Transform, (With<Camera2d>, Without<Drone>)>,
) {
    let (_, rotation, translation) = drone.to_scale_rotation_translation();
    camera.translation.x = translation.x;
    camera.translation.y = translation.y;
    camera.rotation = rotation;
}
