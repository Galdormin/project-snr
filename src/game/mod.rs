//! The gameplay.

use avian2d::prelude::*;
use bevy::prelude::*;

mod drone;
mod ldtk;
mod terrain;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(PhysicsPlugins::default());
    app.insert_resource(Gravity(Vec2::ZERO));

    app.add_plugins((ldtk::plugin, drone::plugin, terrain::plugin));
}

#[derive(Default, PhysicsLayer)]
pub enum PhysicLayer {
    #[default]
    Terrain,
    Drone,
}
