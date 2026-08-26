//! Code for the terrain

use avian2d::{
    collision::collider::{Collider, CollisionLayers},
    dynamics::rigid_body::RigidBody,
};
use bevy::prelude::*;
use bevy_ecs_ldtk::{LdtkIntCell, TileMetadata};
use bevy_ecs_tilemap::tiles::TileFlip;
use serde::Deserialize;

use crate::game::{
    PhysicLayer,
    ldtk::TileMetadataParser,
    sonar::feature::{SurfaceFeatures, SurfacePassage, SurfaceReflexivity, SurfaceTexture},
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, load_wall_collider);
}

/// Collider shape for a wall tile, read from its `collider` metadata key.
#[derive(Debug, Clone, Copy, Deserialize)]
pub enum WallCollider {
    None,
    Square(f32),
    Triangle(f32),
}

impl WallCollider {
    /// Builds the collider, mirroring it to match the tile's LDtk flip flags. Flips
    /// only affect the tile's rendering (UVs), not its `Transform`, so the collider
    /// has to be mirrored by hand to stay aligned with what's drawn.
    fn as_collider(self, flip: TileFlip) -> Option<Collider> {
        match self {
            WallCollider::None => None,
            WallCollider::Square(length) => Some(Collider::rectangle(length, length)),
            WallCollider::Triangle(length) => {
                let half = length / 2.0;
                let sign_x = if flip.x { -1.0 } else { 1.0 };
                let sign_y = if flip.y { -1.0 } else { 1.0 };
                Some(Collider::triangle(
                    Vec2::new(-half * sign_x, -half * sign_y),
                    Vec2::new(half * sign_x, -half * sign_y),
                    Vec2::new(-half * sign_x, half * sign_y),
                ))
            }
        }
    }
}

#[derive(Component, Reflect, Clone, Copy, Debug)]
#[reflect(Component)]
pub struct Wall;

/// Only the marker: `RigidBody`/`Collider`/`CollisionLayers` are inserted
/// lazily by `load_wall_collider`, only for tiles that actually need one.
/// Eagerly spawning them for every int-grid cell (most of which turn out to
/// be interior tiles with no collider) registers and immediately tears down
/// thousands of physics bodies, which is what made startup slow.
#[derive(Bundle, LdtkIntCell)]
pub(super) struct TerrainBundle {
    wall: Wall,
    features: SurfaceFeatures,
}

impl Default for TerrainBundle {
    fn default() -> Self {
        Self {
            wall: Wall,
            features: SurfaceFeatures {
                texture: SurfaceTexture::Hard,
                reflexivity: SurfaceReflexivity::Strong,
                passage: SurfacePassage::Solid,
            },
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

        let wall_collider = metadata.parse_key::<WallCollider>("collider")?;
        if let Some(collider) = wall_collider.as_collider(flip.copied().unwrap_or_default()) {
            commands.entity(entity).insert((
                RigidBody::Static,
                collider,
                CollisionLayers::new(
                    PhysicLayer::Terrain,
                    [PhysicLayer::Drone, PhysicLayer::Creature],
                ),
            ));
        } else {
            commands.entity(entity).despawn();
        }
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
