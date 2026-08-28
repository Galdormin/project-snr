//! The gameplay.

use avian2d::prelude::*;
use bevy::prelude::*;

mod beacon;
mod camera;
pub mod creatures;
pub mod drone;
mod ldtk;
mod sonar;
mod sound;
mod terrain;
mod ui;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(PhysicsPlugins::default());
    app.insert_resource(Gravity(Vec2::ZERO));

    app.add_plugins((
        beacon::plugin,
        camera::plugin,
        creatures::plugin,
        drone::plugin,
        ldtk::plugin,
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
