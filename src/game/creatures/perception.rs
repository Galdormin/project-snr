//! Code for creature perception

use bevy::prelude::*;

use crate::game::{
    creatures::{Creature, steer_glide::SteeringSet},
    sound::NoiseEvent,
};

pub(super) fn plugin(app: &mut App) {
    app.configure_sets(
        Update,
        PerceptionSet::Hear
            .before(SteeringSet::Danger)
            .before(SteeringSet::Interest),
    );

    app.add_systems(Update, update_perception.in_set(PerceptionSet::Hear));
}

/// How long (s) a heard sound stays worth chasing before it's forgotten.
const HEARD_MEMORY: f32 = 6.0;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PerceptionSet {
    /// Set for processing hearing
    Hear,
}

/// The last [`NoiseEvent`] a creature heard, if any, and how sensitive its
/// hearing is. The only way it learns about anything happening in the world.
#[derive(Component, Debug)]
pub struct Heard {
    /// dB threshold above which a noise is perceived; lower = more sensitive.
    pub sensitivity_db: f32,
    pub last: Option<HeardSound>,
}

impl Heard {
    pub fn with_sensitivity(sensitivity_db: f32) -> Self {
        Self {
            sensitivity_db,
            last: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HeardSound {
    position: Vec2,
    heard_at: f32,
}

impl HeardSound {
    pub fn position(&self) -> Vec2 {
        self.position
    }
}

/// Records incoming [`NoiseEvent`]s a creature is sensitive enough to pick up,
/// and forgets them once stale.
fn update_perception(
    time: Res<Time>,
    mut noise_events: MessageReader<NoiseEvent>,
    mut creatures: Query<(&GlobalTransform, &mut Heard), With<Creature>>,
) {
    let now = time.elapsed_secs();
    let events: Vec<NoiseEvent> = noise_events.read().copied().collect();

    for (transform, mut heard) in &mut creatures {
        let position = transform.translation().truncate();

        for noise in &events {
            if noise.db_at(position) >= heard.sensitivity_db {
                heard.last = Some(HeardSound {
                    position: noise.position,
                    heard_at: now,
                });
            }
        }

        if let Some(sound) = heard.last
            && now - sound.heard_at > HEARD_MEMORY
        {
            heard.last = None;
        }
    }
}
