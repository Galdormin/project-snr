//! The gameplay.

use avian2d::prelude::*;
use bevy::prelude::*;

pub mod audio;
pub mod beacon;
mod camera;
pub mod creatures;
pub mod drone;
mod ldtk;
mod note;
mod sonar;
pub mod sound;
mod terrain;
mod ui;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(PhysicsPlugins::default());
    app.insert_resource(Gravity(Vec2::ZERO));

    app.add_plugins((
        audio::plugin,
        beacon::plugin,
        camera::plugin,
        creatures::plugin,
        drone::plugin,
        ldtk::plugin,
        note::plugin,
        sonar::plugin,
        sound::plugin,
        terrain::plugin,
        ui::plugin,
    ));
}

#[derive(Default, PhysicsLayer)]
pub enum PhysicLayer {
    #[default]
    Terrain,
    Drone,
    Creature,
    Beacon,
}
