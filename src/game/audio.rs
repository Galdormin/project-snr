use avian2d::physics_transform::Position;
use bevy::prelude::*;
use bevy_seedling::{prelude::*, utils::perceptual_volume::PerceptualVolume};

use crate::{
    asset_tracking::LoadResource,
    game::{
        drone::Drone,
        sound::{NoiseEvent, NoiseType},
    },
    screens::Screen,
};

/// Level the drone's own ping is played back at.
const SONAR_PING_DB: f32 = -12.0;
/// Level a beacon's answer is played back at, before distance
const BEACON_PING_DB: f32 = -12.0;
/// Level the ambient bed is played back at.
const AMBIENT_MUSIC_DB: f32 = -12.0;

pub const DEFAULT_SONAR_GAIN: f32 = 0.5;
pub const DEFAULT_AMBIENT_GAIN: f32 = 0.8;
pub const VOLUME: PerceptualVolume = PerceptualVolume::new();

pub(super) fn plugin(app: &mut App) {
    app.load_resource::<NoiseAssets>();
    app.load_resource::<MusicAssets>();

    app.add_systems(Startup, spawn_audio_graph);
    app.add_systems(OnEnter(Screen::Gameplay), start_ambient_music);
    app.add_systems(
        Update,
        (emit_sound_effect.run_if(resource_exists::<NoiseAssets>),),
    );
}

// -----
// The graph
// -----

/// Everything the drone hears through the water, sonar included.
#[derive(NodeLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct OceanBus;

/// The drone's own ping, coming back through the water.
///
/// It sits under [`OceanBus`]
#[derive(NodeLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SonarBus;

/// Everything the passive picks up that the drone didn't make itself.
#[derive(NodeLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AmbientBus;

/// The samplers the drone's own ping is played on, routed to [`SonarBus`].
#[derive(PoolLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SonarPool;

/// The samplers everything else is played on, routed to [`AmbientBus`].
#[derive(PoolLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AmbientPool;

/// Wires the underwater buses under the engine's sound effects bus.
fn spawn_audio_graph(mut commands: Commands) {
    // The ocean's output is the link itself: everything mixed into the bus is
    // heard through the same failing filter.
    commands
        .spawn((Name::new("Ocean Bus"), OceanBus, VolumeNode::default()))
        .connect(SoundEffectsBus);

    // The drone's own ping goes out into the open cave and comes back long.
    commands
        .spawn((
            Name::new("Sonar Bus"),
            SonarBus,
            VolumeNode {
                volume: VOLUME.perceptual_to_volume(DEFAULT_SONAR_GAIN),
                ..default()
            },
        ))
        .chain_node(FreeverbNode {
            room_size: 0.7,
            damping: 0.1,
            width: 0.3,
            ..default()
        })
        .connect(OceanBus);

    // Everything else is heard closer in, with a tighter tail.
    commands
        .spawn((
            Name::new("Ambient Bus"),
            AmbientBus,
            VolumeNode {
                volume: VOLUME.perceptual_to_volume(DEFAULT_AMBIENT_GAIN),
                ..default()
            },
        ))
        .chain_node(FreeverbNode {
            room_size: 0.5,
            damping: 0.2,
            width: 0.3,
            ..default()
        })
        .connect(OceanBus);

    commands
        .spawn((Name::new("Sonar Pool"), SamplerPool(SonarPool)))
        .connect(SonarBus);
    commands
        .spawn((Name::new("Ambient Pool"), SamplerPool(AmbientPool)))
        .connect(AmbientBus);
}

// -----
// Samples
// -----

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
struct NoiseAssets {
    #[dependency]
    sonar: Handle<AudioSample>,
    #[dependency]
    beacon: Handle<AudioSample>,
}

impl FromWorld for NoiseAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            sonar: assets.load("audio/sound_effects/sonar_ping.ogg"),
            beacon: assets.load("audio/sound_effects/beacon_ping.ogg"),
        }
    }
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
struct MusicAssets {
    #[dependency]
    music: Handle<AudioSample>,
}

impl FromWorld for MusicAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            music: assets.load("audio/ambient/abyss_bed_loop.ogg"),
        }
    }
}

fn start_ambient_music(mut commands: Commands, music: Res<MusicAssets>) {
    commands.spawn((
        Name::new("Ambient Music"),
        DespawnOnExit(Screen::Gameplay),
        AmbientPool,
        SamplePlayer::new(music.music.clone())
            .looping()
            .with_volume(Volume::Decibels(AMBIENT_MUSIC_DB)),
    ));
}

fn emit_sound_effect(
    mut commands: Commands,
    mut noise_events: MessageReader<NoiseEvent>,
    noise_assets: Res<NoiseAssets>,
    drone_pos: Single<&Position, With<Drone>>,
) {
    for noise in noise_events.read() {
        match noise.noise_type {
            NoiseType::Sonar => {
                commands.spawn((
                    SonarPool,
                    SamplePlayer::new(noise_assets.sonar.clone())
                        .with_volume(Volume::Decibels(SONAR_PING_DB)),
                ));
            }
            NoiseType::Beacon => {
                let volume = BEACON_PING_DB + 0.5 * (noise.db_at(drone_pos.0) - 50.0);
                commands.spawn((
                    AmbientPool,
                    SamplePlayer::new(noise_assets.beacon.clone())
                        .with_volume(Volume::Decibels(volume)),
                ));
            }
            // The engine is does not create souond to the player
            NoiseType::Drone => {}
        }
    }
}
