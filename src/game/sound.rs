//! Code for spatial noise events, visualized as expanding rings.
//!
//! Any system can fire a [`NoiseEvent`] (engine thrust, collisions, ...) to make
//! something audible in the world; this module only takes care of the visuals.

use bevy::prelude::*;

use crate::theme::palette;

/// How long a noise ring takes to expand to its max radius and fade out.
const RING_LIFETIME: f32 = 0.6;
/// Converts a [`NoiseEvent`] intensity into the radius (px) of its ring.
const INTENSITY_TO_RADIUS_SCALE: f32 = 0.3;

pub(super) fn plugin(app: &mut App) {
    app.add_message::<NoiseEvent>();
    app.add_systems(Update, (spawn_noise_rings, draw_noise_rings).chain());
}

/// A noise made at a world position, with an arbitrary `intensity` (e.g. summed
/// engine power) that this module converts into the radius of the ring it draws.
#[derive(Message, Clone, Copy, Debug)]
pub struct NoiseEvent {
    pub position: Vec2,
    pub intensity: f32,
}

/// A single expanding, fading ring drawn at the position it was emitted at,
/// independently of whatever emitted the [`NoiseEvent`].
#[derive(Component)]
struct NoiseRing {
    position: Vec2,
    timer: Timer,
    max_radius: f32,
}

fn spawn_noise_rings(mut commands: Commands, mut noise_events: MessageReader<NoiseEvent>) {
    for noise in noise_events.read() {
        commands.spawn(NoiseRing {
            position: noise.position,
            timer: Timer::from_seconds(RING_LIFETIME, TimerMode::Once),
            max_radius: noise.intensity * INTENSITY_TO_RADIUS_SCALE,
        });
    }
}

fn draw_noise_rings(
    mut commands: Commands,
    time: Res<Time>,
    mut gizmos: Gizmos,
    mut rings: Query<(Entity, &mut NoiseRing)>,
) {
    for (entity, mut ring) in &mut rings {
        if ring.timer.tick(time.delta()).is_finished() {
            commands.entity(entity).despawn();
            continue;
        }

        let progress = ring.timer.elapsed_secs() / ring.timer.duration().as_secs_f32();
        gizmos.circle_2d(
            ring.position,
            ring.max_radius * progress,
            palette::PHOSPHORE_HIGH.with_alpha(1.0 - progress),
        );
    }
}
