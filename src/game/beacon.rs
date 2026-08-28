//! Code for beacon

use bevy::prelude::*;

use avian2d::prelude::*;
use bevy_ecs_ldtk::LdtkEntity;

use crate::game::{
    PhysicLayer,
    sonar::feature::{SurfaceFeatures, SurfacePassage, SurfaceReflexivity, SurfaceTexture},
    sound::{NoiseEvent, NoiseType},
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, (detect_sound, emit_noise_beacon));
}

/// Marker component for all beacons
#[derive(Component, Reflect, Clone, Copy, Debug)]
#[reflect(Component)]
pub struct Beacon;

/// LDtk Bundle for a Silent creature.
#[derive(Bundle, LdtkEntity)]
pub struct BeaconBundle {
    beacon: Beacon,
    receiver: BeaconReceiver,
    emitter: BeaconEmitter,
    features: SurfaceFeatures,

    // Physics
    rigid_body: RigidBody,
    collider: Collider,
    collision_layers: CollisionLayers,
}

impl Default for BeaconBundle {
    fn default() -> Self {
        Self {
            beacon: Beacon,
            receiver: BeaconReceiver {
                sensitivity_db: 0.0,
            },
            emitter: BeaconEmitter::new(200.0),
            features: SurfaceFeatures {
                texture: SurfaceTexture::Hard,
                reflexivity: SurfaceReflexivity::Strong,
                passage: SurfacePassage::Porous,
            },
            rigid_body: RigidBody::Static,
            collider: Collider::circle(4.0),
            collision_layers: CollisionLayers::new(
                PhysicLayer::Beacon,
                [PhysicLayer::Creature, PhysicLayer::Drone],
            ),
        }
    }
}

#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
pub struct BeaconReceiver {
    sensitivity_db: f32,
}

#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
struct BeaconEmitter {
    range: f32,
    timer: Timer,
}

impl BeaconEmitter {
    fn new(range: f32) -> Self {
        let mut timer = Timer::from_seconds(0.0, TimerMode::Once);
        timer.finish();

        Self { range, timer }
    }

    fn emit_with_delay(&mut self, delay: f32) {
        self.timer = Timer::from_seconds(delay, TimerMode::Once);
    }
}

fn detect_sound(
    mut noise_events: MessageReader<NoiseEvent>,
    mut beacons: Query<(&BeaconReceiver, &mut BeaconEmitter, &Position)>,
) {
    for noise in noise_events.read() {
        if noise.noise_type != NoiseType::Sonar {
            continue;
        }

        for (receiver, mut emitter, position) in &mut beacons {
            if noise.db_at(position.0) >= receiver.sensitivity_db {
                emitter.emit_with_delay(noise.time_to_reach(position.0));
            }
        }
    }
}

fn emit_noise_beacon(
    time: Res<Time>,
    mut noise_event: MessageWriter<NoiseEvent>,
    beacons: Query<(&mut BeaconEmitter, &Position)>,
) {
    let delta = time.delta();
    for (mut emitter, position) in beacons {
        if emitter.timer.tick(delta).just_finished() {
            noise_event.write(NoiseEvent::from_loudness(
                position.0,
                NoiseType::Beacon,
                emitter.range,
            ));
        }
    }
}
