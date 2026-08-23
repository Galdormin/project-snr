//! Code for all the creatures and their behaviour

use avian2d::{
    collision::collider::CollisionLayers,
    dynamics::rigid_body::{LockedAxes, RigidBody},
};
use bevy::prelude::*;

use crate::game::PhysicLayer;

mod perception;
mod silent;
pub mod steer_glide;

pub use silent::SilentBundle;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((perception::plugin, silent::plugin, steer_glide::plugin));
}

/// Marker component for a creature.
#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component)]
pub struct Creature;

/// Physic bundle for a Creature.
#[derive(Bundle)]
pub struct CreatureBundle {
    creature: Creature,
    rigid_body: RigidBody,
    collision_layers: CollisionLayers,
    locked_axes: LockedAxes,
}

impl Default for CreatureBundle {
    fn default() -> Self {
        Self {
            creature: Creature,
            rigid_body: RigidBody::Dynamic,
            collision_layers: CollisionLayers::new(
                PhysicLayer::Creature,
                [PhysicLayer::Terrain, PhysicLayer::Drone],
            ),
            locked_axes: LockedAxes::ROTATION_LOCKED,
        }
    }
}
