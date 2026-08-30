//! Code for spatial noise events, visualized as expanding rings.
//!
//! Any system can fire a [`NoiseEvent`] (engine thrust, collisions, ...) to make
//! something audible in the world; this module only takes care of the visuals.

use avian2d::physics_transform::Position;
use bevy::prelude::*;
use bevy_seedling::prelude::*;

use crate::{
    asset_tracking::LoadResource,
    game::{drone::Drone, sonar::scan::ECHO_DELAY_PER_PIXEL},
    theme::palette,
};

/// Speed (px/s) at which a noise ring expands, fading out as it grows — so a
/// loud (large) noise takes longer to fully expand than a faint one, instead
/// of every ring taking the same fixed duration.
const RING_EXPANSION_SPEED: f32 = 500.0;
/// Reference loudness/distance at which a noise sits at exactly 0 dB
const REFERENCE_UNIT: f32 = 1.0;
/// dB level below which a noise ring is considered invisible.
const RING_CUTOFF_DB: f32 = 0.0;
/// Level the drone's own ping is played back at.
const SONAR_PING_DB: f32 = -12.0;
/// Level a beacon's answer is played back at, before distance
const BEACON_PING_DB: f32 = -12.0;

pub const DEFAULT_SONAR_GAIN: f32 = 0.5;
pub const DEFAULT_AMBIENT_GAIN: f32 = 0.5;
pub const VOLUME: PerceptualVolume = PerceptualVolume::new();

pub(super) fn plugin(app: &mut App) {
    app.add_message::<NoiseEvent>();
    app.load_resource::<NoiseAssets>();

    app.add_systems(Startup, spawn_audio_buses);

    app.add_systems(
        Update,
        (
            (spawn_noise_rings, draw_noise_rings).chain(),
            emit_sound_effect.run_if(resource_exists::<NoiseAssets>),
        ),
    );
}

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
///
/// ```text
/// SonarPool -> SonarBus -> cave reverb |
///                                      | => OceanBus -> SoundEffectsBus
/// AmbientPool -> AmbientBus -> reverb  |
/// ```
fn spawn_audio_buses(mut commands: Commands) {
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
        .chain_node((FreeverbNode {
            room_size: 0.7,
            damping: 0.1,
            width: 0.3,
            ..default()
        },))
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
        .chain_node((FreeverbNode {
            room_size: 0.5,
            damping: 0.2,
            width: 0.3,
            ..default()
        },))
        .connect(OceanBus);

    commands
        .spawn((Name::new("Sonar Pool"), SamplerPool(SonarPool)))
        .connect(SonarBus);
    commands
        .spawn((Name::new("Ambient Pool"), SamplerPool(AmbientPool)))
        .connect(AmbientBus);
}

/// Type of noise
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NoiseType {
    /// Sound emitted y Beacon
    Beacon,
    /// Drone motors or collision
    Drone,
    /// Donar sonar scan
    Sonar,
}

impl NoiseType {
    fn color(&self) -> Color {
        match self {
            NoiseType::Beacon => palette::AMBER_HIGH,
            NoiseType::Drone => palette::PHOSPHORE_HIGH,
            NoiseType::Sonar => palette::PHOSPHORE_HIGH,
        }
    }
}

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

/// A noise made at a world position.
#[derive(Message, Clone, Copy, Debug)]
pub struct NoiseEvent {
    pub position: Vec2,
    pub noise_type: NoiseType,
    loudness_db: f32,
}

impl NoiseEvent {
    pub fn new(position: Vec2, noise_type: NoiseType, loudness_db: f32) -> Self {
        Self {
            position,
            noise_type,
            loudness_db,
        }
    }

    pub fn from_loudness(position: Vec2, noise_type: NoiseType, loudness: f32) -> Self {
        Self::new(
            position,
            noise_type,
            20.0 * loudness.max(REFERENCE_UNIT).log10(),
        )
    }

    /// The dB level of this noise as perceived from `position`.
    pub fn db_at(&self, position: Vec2) -> f32 {
        let distance = self.position.distance(position).max(REFERENCE_UNIT);
        self.loudness_db - 20.0 * (distance / REFERENCE_UNIT).log10()
    }

    /// Time for the noise to reach the given point
    pub fn time_to_reach(&self, position: Vec2) -> f32 {
        self.position.distance(position) * ECHO_DELAY_PER_PIXEL
    }

    /// Distance (px) at which this noise fades below `RING_CUTOFF_DB`, i.e.
    /// the max radius of the ring drawn for it.
    fn ring_radius(&self) -> f32 {
        REFERENCE_UNIT * 10f32.powf((self.loudness_db - RING_CUTOFF_DB) / 20.0)
    }
}

/// A single expanding, fading ring drawn at the position it was emitted at.
#[derive(Component)]
struct NoiseRing {
    position: Vec2,
    radius: f32,
    max_radius: f32,
    noise_type: NoiseType,
}

fn spawn_noise_rings(mut commands: Commands, mut noise_events: MessageReader<NoiseEvent>) {
    for noise in noise_events.read() {
        if noise.noise_type == NoiseType::Drone {
            continue;
        }

        commands.spawn(NoiseRing {
            position: noise.position,
            radius: 0.0,
            max_radius: noise.ring_radius(),
            noise_type: noise.noise_type,
        });
    }
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

fn draw_noise_rings(
    mut commands: Commands,
    time: Res<Time>,
    mut gizmos: Gizmos,
    mut rings: Query<(Entity, &mut NoiseRing)>,
) {
    for (entity, mut ring) in &mut rings {
        ring.radius += RING_EXPANSION_SPEED * time.delta_secs();
        if ring.radius >= ring.max_radius {
            commands.entity(entity).despawn();
            continue;
        }

        let progress = ring.radius / ring.max_radius;
        gizmos.circle_2d(
            ring.position,
            ring.radius,
            ring.noise_type.color().with_alpha(1.0 - progress),
        );
    }
}
