//! Code for the Context Steering algorithm and the Burst/Glide movement

use core::f32;

use avian2d::{
    collision::collider::Collider,
    dynamics::rigid_body::{LinearDamping, LinearVelocity},
    physics_transform::Rotation,
    spatial_query::{ShapeCastConfig, SpatialQuery, SpatialQueryFilter},
};
use bevy::prelude::*;

use crate::{
    AppSystems, PausableSystems,
    game::{PhysicLayer, creatures::Creature},
};

/// Number of slots for the Context Steering algorithm.
pub const STEER_SLOTS: usize = 12;
/// Angle between each slot. 12 slots = 30° arc
pub const SLOT_ANGLE: f32 = f32::consts::TAU / STEER_SLOTS as f32;
/// Above this danger, a direction is masked out entirely.
const MASK_THRESHOLD: f32 = 0.35;

pub(super) fn plugin(app: &mut App) {
    app.configure_sets(
        Update,
        (SteeringSystems::Danger, SteeringSystems::Interest).before(SteeringSystems::Decide),
    );

    app.add_systems(
        Update,
        (
            synchronise_burst_glide.in_set(AppSystems::Synchronize),
            (
                build_danger_collision.in_set(SteeringSystems::Danger),
                pick_direction.in_set(SteeringSystems::Decide),
                apply_burst,
            )
                .in_set(AppSystems::Update)
                .in_set(PausableSystems),
        ),
    );
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SteeringSystems {
    /// Fill the [`ContextMapDanger`]
    Danger,
    /// Fill the [`ContextMapInterest`]
    Interest,
    /// Update the [`DesiredDir`] based on [`ContextMapDanger`] and [`ContextMapInterest`]
    Decide,
}

/// Marker component for context map algorithm
#[derive(Component, Reflect, Clone, Copy, Debug)]
#[reflect(Component)]
#[require(ContextMapDanger, ContextMapInterest, DesiredDir)]
pub struct ContextMap;

/// List the danger value for each slots
#[derive(Component, Reflect, Clone, Debug, Default)]
#[reflect(Component)]
pub struct ContextMapDanger(pub [f32; STEER_SLOTS]);

/// List the interest value for each slots
#[derive(Component, Reflect, Clone, Debug, Default)]
#[reflect(Component)]
pub struct ContextMapInterest(pub [f32; STEER_SLOTS]);

/// Desired direction computed by the Context Steering algorithm
#[derive(Component, Reflect, Clone, Copy, Debug)]
#[reflect(Component)]
pub struct DesiredDir(pub Dir2);

impl Default for DesiredDir {
    fn default() -> Self {
        Self(Dir2::X)
    }
}

/// Component used to fill [`ContextMapDanger`] to avoid [`PhysicLayer::Terrain`].
#[derive(Component, Debug, Clone, Copy)]
pub struct ObstacleAvoidance {
    /// Distance (px) to avoid objects.
    /// This needs to be computed based non [`BurstGlide`] values to avoid collision in glide.
    pub distance: f32,
}

impl ObstacleAvoidance {
    pub fn new(distance: f32) -> Self {
        Self { distance }
    }
}

/// Burst-and-glide locomotion: an irregular impulse towards `DesiredDir`.
/// `glide_damping` is automatically synchronized with [`LinearDamping`].
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
#[require(BurstTimer, LinearDamping)]
pub struct BurstGlide {
    /// Velocity (px/s) added by a single burst
    pub burst_impulse: f32,
    /// Range (s) (choosen randomly) between each burst.
    pub burst_interval: (f32, f32),
    /// Dragged applied between burst
    pub glide_damping: f32,
}

/// Timer between each burst
#[derive(Component, Reflect, Default, Debug)]
#[reflect(Component)]
struct BurstTimer(Timer);

impl BurstTimer {
    fn from_interval(interval: (f32, f32)) -> Self {
        Self(Timer::from_seconds(
            rand::random_range(interval.0..=interval.1),
            TimerMode::Once,
        ))
    }
}

/// Synchronise [`BurstTimer`] and [`LinearDamping`] on changed (and add) of [`BurstGlide`]
fn synchronise_burst_glide(
    creatures: Query<(&BurstGlide, &mut LinearDamping, &mut BurstTimer), Changed<BurstGlide>>,
) {
    for (burst_glide, mut linear_damping, mut burst_timer) in creatures {
        linear_damping.0 = burst_glide.glide_damping;
        *burst_timer = BurstTimer::from_interval(burst_glide.burst_interval);
    }
}

/// Apply burst of the Burst and Glide
fn apply_burst(
    time: Res<Time>,
    creatures: Query<(
        &DesiredDir,
        &mut Rotation,
        &mut LinearVelocity,
        &BurstGlide,
        &mut BurstTimer,
    )>,
) {
    for (desired_dir, mut rotation, mut velocity, burst_data, mut burst_timer) in creatures {
        if burst_timer.0.tick(time.delta()).is_finished() {
            *rotation = Rotation::radians(desired_dir.0.to_angle());
            velocity.0 += desired_dir.0 * burst_data.burst_impulse;
            *burst_timer = BurstTimer::from_interval(burst_data.burst_interval);
        }
    }
}

/// Build the [`ContextMapDanger`] with collision only
fn build_danger_collision(
    spatial: SpatialQuery,
    creatures: Query<
        (
            &Collider,
            &GlobalTransform,
            &ObstacleAvoidance,
            &mut ContextMapDanger,
        ),
        With<Creature>,
    >,
) -> Result<()> {
    for (collider, transform, avoidance, mut danger) in creatures {
        let mut shape = collider.clone();
        shape.set_scale(Vec2::splat(1.1), 1);
        let origin = transform.translation().truncate();
        let filter = SpatialQueryFilter::from_mask(PhysicLayer::Terrain);
        let config = ShapeCastConfig::from_max_distance(avoidance.distance);

        danger.0 = (0..STEER_SLOTS)
            .map(|s| {
                let direction = Dir2::from_angle(s as f32 * SLOT_ANGLE);
                match spatial.cast_shape(
                    &shape,
                    origin,
                    s as f32 * SLOT_ANGLE,
                    direction,
                    &config,
                    &filter,
                ) {
                    Some(hit) => 1.0 - (hit.distance / avoidance.distance).clamp(0.0, 1.0),
                    None => 0.0,
                }
            })
            .collect::<Vec<_>>()
            .try_into()
            .expect("mapped from 0..STEER_SLOTS, so exactly STEER_SLOTS items");
    }
    Ok(())
}

/// Combine [`ContextMapDanger`] and [`ContextMapInterest`] into a [`DesiredDir`]:
/// mask out anything too dangerous, take the best interest among what's left.
fn pick_direction(mut creatures: Query<(&ContextMapDanger, &ContextMapInterest, &mut DesiredDir)>) {
    for (danger, interest, mut desired) in &mut creatures {
        let min_danger = danger.0.iter().copied().fold(f32::MAX, f32::min);
        let threshold = (min_danger + 0.05).max(MASK_THRESHOLD);

        // Weighted vector average of the surviving slots: avoids the "robot
        // turning in 30° increments" look.
        let mut sum = Vec2::ZERO;
        for s in 0..STEER_SLOTS {
            if danger.0[s] <= threshold {
                sum += Dir2::from_angle(s as f32 * SLOT_ANGLE).as_vec2() * interest.0[s];
            }
        }

        // Everything is masked (tight dead end): take the least bad direction.
        if sum == Vec2::ZERO {
            let (best, _) = danger
                .0
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.total_cmp(b))
                .unwrap();
            sum = Dir2::from_angle(best as f32 * SLOT_ANGLE).as_vec2();
        }

        if let Ok(d) = Dir2::new(sum) {
            desired.0 = d;
        }
    }
}
