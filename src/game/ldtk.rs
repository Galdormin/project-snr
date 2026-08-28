//! All Ldtk related stuff

use std::collections::HashMap;

use bevy::{ecs::name::Name, prelude::*};
use thiserror::Error;

use bevy_ecs_ldtk::{
    LdtkPlugin, LdtkProjectHandle, LdtkSettings, LdtkWorldBundle, LevelSelection,
    LevelSpawnBehavior, TileMetadata,
    app::{LdtkEntityAppExt, LdtkIntCellAppExt},
};

use crate::{
    game::{
        beacon::BeaconBundle, creatures::SilentBundle, drone::DroneBundle, terrain::TerrainBundle,
    },
    screens::Screen,
};

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(LdtkPlugin);

    // Init LDtk settings
    app.insert_resource(LevelSelection::default());
    app.insert_resource(LdtkSettings {
        level_spawn_behavior: LevelSpawnBehavior::UseWorldTranslation {
            load_level_neighbors: true,
        },
        ..default()
    });

    app.register_ldtk_int_cell::<TerrainBundle>(1);
    app.register_ldtk_entity::<DroneBundle>("Drone");
    app.register_ldtk_entity::<SilentBundle>("Silent");
    app.register_ldtk_entity::<BeaconBundle>("Beacon");

    app.add_systems(OnEnter(Screen::Gameplay), spawn_world);
}

fn spawn_world(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Name::new("Ldtk World"),
        DespawnOnExit(Screen::Gameplay),
        LdtkWorldBundle {
            ldtk_handle: LdtkProjectHandle::from(asset_server.load("cave_test.ldtk")),
            ..default()
        },
    ));
}

/// Error returned when reading a typed value out of a tile's [`TileMetadata`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MetadataParseError {
    /// No metadata to parse
    #[error("no metadata to parse")]
    NoMetadata,
    /// The metadata string isn't in the expected `key: value` line format.
    #[error("malformed metadata line: {0:?}")]
    MetadataFormatError(String),
    /// No line in the metadata matches the requested key.
    #[error("unknown metadata key: {0:?}")]
    UnknownKey(String),
    /// The value for the key couldn't be deserialized into the requested type.
    #[error("failed to parse metadata value: {0}")]
    ParseError(String),
}

/// Extension trait for reading typed values out of a tile's `key: value` metadata lines,
/// e.g. `collider: Square`.
pub trait TileMetadataParser {
    fn parse_key<T>(&self, key: &str) -> Result<T, MetadataParseError>
    where
        T: serde::de::DeserializeOwned;
}

impl TileMetadataParser for TileMetadata {
    fn parse_key<T>(&self, key: &str) -> Result<T, MetadataParseError>
    where
        T: serde::de::DeserializeOwned,
    {
        if self.data.is_empty() {
            return Err(MetadataParseError::NoMetadata);
        }

        let map = self
            .data
            .lines()
            .map(|line| {
                let split = line.splitn(2, ": ").collect::<Vec<&str>>();
                if split.len() != 2 {
                    Err(MetadataParseError::MetadataFormatError(line.into()))
                } else {
                    Ok((split[0], split[1]))
                }
            })
            .collect::<Result<HashMap<&str, &str>, MetadataParseError>>()?;

        let row_data = map
            .get(key)
            .ok_or(MetadataParseError::UnknownKey(key.into()))?;

        ron::from_str(row_data).map_err(|err| MetadataParseError::ParseError(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(data: &str) -> TileMetadata {
        TileMetadata { data: data.into() }
    }

    #[test]
    fn error_on_empty_metadata() {
        let result = metadata("").parse_key::<i32>("key");
        assert_eq!(result, Err(MetadataParseError::NoMetadata));
    }

    #[test]
    fn parse_known_key() {
        let result = metadata("count: 42").parse_key::<i32>("count");
        assert_eq!(result, Ok(42));
    }

    #[test]
    fn parse_key_among_multiple_lines() {
        let result = metadata("a: 1\nb: 2").parse_key::<i32>("b");
        assert_eq!(result, Ok(2));
    }

    #[test]
    fn error_on_unknown_key() {
        let result = metadata("count: 42").parse_key::<i32>("missing");
        assert_eq!(
            result,
            Err(MetadataParseError::UnknownKey("missing".into()))
        );
    }

    #[test]
    fn error_on_malformed_line() {
        let result = metadata("not_a_key_value_pair").parse_key::<i32>("count");
        assert_eq!(
            result,
            Err(MetadataParseError::MetadataFormatError(
                "not_a_key_value_pair".into()
            ))
        );
    }

    #[test]
    fn error_on_invalid_value_for_type() {
        let result = metadata("count: not_a_number").parse_key::<i32>("count");
        assert!(matches!(result, Err(MetadataParseError::ParseError(_))));
    }
}
