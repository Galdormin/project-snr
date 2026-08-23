//! Code for the "Silent" creature

use avian2d::collision::collider::Collider;
use bevy::prelude::*;
use bevy_ecs_ldtk::LdtkEntity;

use crate::game::creatures::{
    Creature, CreatureBundle,
    perception::{Heard, PerceptionSet},
    steer_glide::{
        BurstGlide, ContextMap, ContextMapInterest, DesiredDir, ObstacleAvoidance, SLOT_ANGLE,
        STEER_SLOTS, SteeringSet,
    },
};

/// Weight of the "keep going straight" bias: prevents U-turns and jitter.
const INERTIA: f32 = 0.30;
/// How far ahead (px) the creature casts avoidance shapes.
const LOOK_AHEAD: f32 = 48.0;
/// Velocity (px/s) added by a single burst.
const BURST_IMPULSE: f32 = 50.0;
/// Range (s) for the (irregular) delay between two bursts.
const BURST_INTERVAL: (f32, f32) = (1.5, 3.0);
/// Drag applied between bursts, responsible for the "glide" half of the cycle.
const GLIDE_DAMPING: f32 = 1.0;
/// dB threshold above which this creature perceives a [`NoiseEvent`].
const HEARING_SENSITIVITY_DB: f32 = 0.0;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            update_target_from_heard
                .after(PerceptionSet::Hear)
                .before(SteeringSet::Interest),
            give_up_when_stuck.before(SteeringSet::Interest),
            build_interest.in_set(SteeringSet::Interest),
        ),
    );
}

/// What the creature currently wants to do.
#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component)]
pub enum SteerTarget {
    /// Wandering: slow drift around a heading that turns on its own.
    #[default]
    Wander,
    /// Converge towards a point.
    Seek(Vec2),
    /// Move away from a point.
    #[allow(dead_code)]
    Flee(Vec2),
}

/// Anti-stuck: give up on the current [`SteerTarget`] if we stop making progress.
#[derive(Component, Reflect, Clone, Copy, Debug)]
#[reflect(Component)]
pub struct Persistence {
    pub best_distance: f32,
    pub patience: f32,
    pub timer: f32,
}

impl Default for Persistence {
    fn default() -> Self {
        Self {
            best_distance: f32::MAX,
            // Must comfortably outlast a single burst-and-glide cycle (up to
            // `BURST_INTERVAL.1` between bursts), or the creature "gives up"
            // simply because it hasn't burst yet, not because it's stuck.
            patience: BURST_INTERVAL.1 * 2.5,
            timer: 0.0,
        }
    }
}

/// LDtk Bundle for a Silent creature.
#[derive(Bundle, LdtkEntity)]
pub struct SilentBundle {
    creature: CreatureBundle,
    collider: Collider,
    avoidance: ObstacleAvoidance,
    context_map: ContextMap,
    burst_glide: BurstGlide,
    steer_target: SteerTarget,
    persistence: Persistence,
    heard: Heard,
}

impl Default for SilentBundle {
    fn default() -> Self {
        Self {
            creature: CreatureBundle::default(),
            collider: Collider::capsule_endpoints(5., Vec2::X * 5.0, Vec2::NEG_X * 5.0),
            avoidance: ObstacleAvoidance::new(LOOK_AHEAD),
            context_map: ContextMap,
            burst_glide: BurstGlide {
                burst_impulse: BURST_IMPULSE,
                burst_interval: BURST_INTERVAL,
                glide_damping: GLIDE_DAMPING,
            },
            steer_target: SteerTarget::Wander,
            persistence: Persistence::default(),
            heard: Heard::with_sensitivity(HEARING_SENSITIVITY_DB),
        }
    }
}

/// Fill the [`ContextMapInterest`]: where the creature would want to go if the
/// world were empty.
fn build_interest(
    time: Res<Time>,
    mut creatures: Query<
        (
            Entity,
            &SteerTarget,
            &GlobalTransform,
            &DesiredDir,
            &mut ContextMapInterest,
        ),
        With<Creature>,
    >,
) {
    let t = time.elapsed_secs();

    for (entity, target, transform, current, mut interest) in &mut creatures {
        let pos = transform.translation().truncate();

        let wish = match target {
            SteerTarget::Seek(p) => (*p - pos).normalize_or_zero(),
            SteerTarget::Flee(p) => (pos - *p).normalize_or_zero(),
            SteerTarget::Wander => {
                // Deterministic noise, decorrelated per entity: no `rand` dependency.
                let seed = entity.index().index() as f32 * 1.7;
                let a = (t * 0.13 + seed).sin() * 2.0 + (t * 0.31 + seed).cos();
                Vec2::from_angle(a)
            }
        };

        let wish = if wish == Vec2::ZERO {
            current.0.as_vec2()
        } else {
            wish
        };
        let heading = current.0.as_vec2();

        for s in 0..STEER_SLOTS {
            let d = Dir2::from_angle(s as f32 * SLOT_ANGLE).as_vec2();
            let want = d.dot(wish);
            let keep_going = d.dot(heading);
            interest.0[s] = (want + INERTIA * keep_going).max(0.0);
        }
    }
}

/// Give up on the current [`SteerTarget::Seek`] if we stop getting closer to it.
fn give_up_when_stuck(
    time: Res<Time>,
    mut creatures: Query<(&mut SteerTarget, &mut Persistence, &GlobalTransform)>,
) {
    let dt = time.delta_secs();

    for (mut target, mut persist, transform) in &mut creatures {
        let SteerTarget::Seek(p) = *target else {
            persist.best_distance = f32::MAX;
            persist.timer = 0.0;
            continue;
        };

        let pos = transform.translation().truncate();
        let d = pos.distance(p);
        if d < persist.best_distance - 2.0 {
            persist.best_distance = d;
            persist.timer = 0.0;
        } else {
            persist.timer += dt;
            if persist.timer > persist.patience {
                *target = SteerTarget::Wander;
                persist.best_distance = f32::MAX;
                persist.timer = 0.0;
            }
        }
    }
}

/// Turns [`Heard`] into a [`SteerTarget`]
fn update_target_from_heard(mut creatures: Query<(&Heard, &mut SteerTarget), Changed<Heard>>) {
    for (heard, mut target) in &mut creatures {
        *target = match heard.last {
            Some(sound) => SteerTarget::Seek(sound.position()),
            None => SteerTarget::Wander,
        };
    }
}
