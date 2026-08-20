//!

use avian2d::{
    collision::collider::{Collider, CollisionLayers},
    dynamics::rigid_body::RigidBody,
};
use bevy::prelude::*;
use bevy_ecs_ldtk::{LdtkIntCell, TileMetadata};
use bevy_ecs_tilemap::tiles::TileFlip;
use serde::Deserialize;

use crate::game::{PhysicLayer, ldtk::TileMetadataParser};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, load_wall_collider);
}

/// Collider shape for a wall tile, read from its `collider` metadata key.
#[derive(Debug, Clone, Copy, Deserialize)]
pub enum WallCollider {
    Square(f32),
    Triangle(f32),
}

impl WallCollider {
    /// Builds the collider, mirroring it to match the tile's LDtk flip flags. Flips
    /// only affect the tile's rendering (UVs), not its `Transform`, so the collider
    /// has to be mirrored by hand to stay aligned with what's drawn.
    fn as_collider(self, flip: TileFlip) -> Collider {
        match self {
            WallCollider::Square(length) => Collider::rectangle(length, length),
            WallCollider::Triangle(length) => {
                let half = length / 2.0;
                let sign_x = if flip.x { -1.0 } else { 1.0 };
                let sign_y = if flip.y { -1.0 } else { 1.0 };
                Collider::triangle(
                    Vec2::new(-half * sign_x, -half * sign_y),
                    Vec2::new(half * sign_x, -half * sign_y),
                    Vec2::new(-half * sign_x, half * sign_y),
                )
            }
        }
    }
}

#[derive(Component, Reflect, Clone, Copy, Debug)]
#[reflect(Component)]
pub struct Wall;

#[derive(Bundle, LdtkIntCell)]
pub(super) struct TerrainBundle {
    wall: Wall,
    body: RigidBody,
    collider: Collider,
    collision_layers: CollisionLayers,
}

impl Default for TerrainBundle {
    fn default() -> Self {
        Self {
            wall: Wall,
            body: RigidBody::Static,
            collider: Collider::rectangle(8.0, 8.0),
            collision_layers: CollisionLayers::new(PhysicLayer::Terrain, [PhysicLayer::Drone]),
        }
    }
}

fn load_wall_collider(
    mut commands: Commands,
    walls: Query<
        (Entity, Option<&TileMetadata>, Option<&TileFlip>),
        (Added<TileMetadata>, With<Wall>),
    >,
) -> Result {
    for (entity, maybe_metadata, flip) in walls {
        let Some(metadata) = maybe_metadata else {
            warn!("Wall has no metadata");
            return Ok(());
        };

        let collider = metadata.parse_key::<WallCollider>("collider")?;
        commands
            .entity(entity)
            .insert(collider.as_collider(flip.copied().unwrap_or_default()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_square_collider_from_metadata() {
        let metadata = TileMetadata {
            data: "collider: Square(8.0)\nvegetation: None".into(),
        };

        let collider = metadata.parse_key::<WallCollider>("collider");

        assert!(matches!(collider, Ok(WallCollider::Square(8.))));
    }

    #[test]
    fn parse_triangle_collider_from_metadata() {
        let metadata = TileMetadata {
            data: "collider: Triangle(8.0)".into(),
        };

        let collider = metadata.parse_key::<WallCollider>("collider");

        assert!(matches!(collider, Ok(WallCollider::Triangle(8.))));
    }
}
