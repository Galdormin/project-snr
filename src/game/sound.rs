//! Code for spatial noise events, visualized as expanding rings.
//!
//! Any system can fire a [`NoiseEvent`] (engine thrust, collisions, ...) to make
//! something audible in the world; this module only takes care of the visuals.

use bevy::prelude::*;

use crate::theme::palette;

/// Speed (px/s) at which a noise ring expands, fading out as it grows — so a
/// loud (large) noise takes longer to fully expand than a faint one, instead
/// of every ring taking the same fixed duration.
const RING_EXPANSION_SPEED: f32 = 500.0;
/// Reference loudness/distance at which a noise sits at exactly 0 dB
const REFERENCE_UNIT: f32 = 1.0;
/// dB level below which a noise ring is considered invisible.
const RING_CUTOFF_DB: f32 = 0.0;

pub(super) fn plugin(app: &mut App) {
    app.add_message::<NoiseEvent>();
    app.add_systems(Update, (spawn_noise_rings, draw_noise_rings).chain());
}

/// A noise made at a world position.
#[derive(Message, Clone, Copy, Debug)]
pub struct NoiseEvent {
    pub position: Vec2,
    loudness_db: f32,
}

impl NoiseEvent {
    pub fn new(position: Vec2, loudness_db: f32) -> Self {
        Self {
            position,
            loudness_db,
        }
    }

    pub fn from_loudness(position: Vec2, loudness: f32) -> Self {
        Self::new(position, 20.0 * loudness.max(REFERENCE_UNIT).log10())
    }

    /// The dB level of this noise as perceived from `position`.
    pub fn db_at(&self, position: Vec2) -> f32 {
        let distance = self.position.distance(position).max(REFERENCE_UNIT);
        self.loudness_db - 20.0 * (distance / REFERENCE_UNIT).log10()
    }

    /// Distance (px) at which this noise fades below `RING_CUTOFF_DB`, i.e.
    /// the max radius of the ring drawn for it.
    fn ring_radius(&self) -> f32 {
        REFERENCE_UNIT * 10f32.powf((self.loudness_db - RING_CUTOFF_DB) / 20.0)
    }
}

/// A single expanding, fading ring drawn at the position it was emitted at,
/// independently of whatever emitted the [`NoiseEvent`]. Grows at a constant
/// [`RING_EXPANSION_SPEED`], so `radius` alone tracks both its size and, via
/// `radius / max_radius`, its fade-out progress — no separate timer needed.
#[derive(Component)]
struct NoiseRing {
    position: Vec2,
    radius: f32,
    max_radius: f32,
}

fn spawn_noise_rings(mut commands: Commands, mut noise_events: MessageReader<NoiseEvent>) {
    for noise in noise_events.read() {
        commands.spawn(NoiseRing {
            position: noise.position,
            radius: 0.0,
            max_radius: noise.ring_radius(),
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
        ring.radius += RING_EXPANSION_SPEED * time.delta_secs();
        if ring.radius >= ring.max_radius {
            commands.entity(entity).despawn();
            continue;
        }

        let progress = ring.radius / ring.max_radius;
        gizmos.circle_2d(
            ring.position,
            ring.radius,
            palette::PHOSPHORE_HIGH.with_alpha(1.0 - progress),
        );
    }
}
