//! The gameplay.

use avian2d::prelude::*;
use bevy::prelude::*;

mod camera;
pub mod creatures;
mod drone;
mod ldtk;
mod scan;
mod sound;
mod terrain;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(PhysicsPlugins::default());
    app.insert_resource(Gravity(Vec2::ZERO));

    app.add_plugins((
        camera::plugin,
        creatures::plugin,
        drone::plugin,
        ldtk::plugin,
        scan::plugin,
        sound::plugin,
        terrain::plugin,
    ));
}

#[derive(Default, PhysicsLayer)]
pub enum PhysicLayer {
    #[default]
    Terrain,
    Drone,
    Creature,
}
