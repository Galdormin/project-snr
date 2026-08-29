//! Code for the drone

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_ecs_ldtk::LdtkEntity;
use bevy_enhanced_input::{
    action::{Action, events::Start},
    actions,
    binding::{Binding, relationship::Bindings},
    bindings,
    context::InputContextAppExt,
    prelude::InputAction,
    preset::bidirectional::Bidirectional,
};

use crate::{
    AppSystems, PausableSystems,
    game::{
        PhysicLayer,
        sonar::{
            range::ScanRange,
            scan::{ActiveScan, Sonar, SonarCooldown},
        },
        sound::{NoiseEvent, NoiseType},
    },
    screens::Screen,
};

/// Drone sprite size
const SPRITE_SIZE: f32 = 8.0;
/// Forward/backward thrust applied while holding Z/S, in m/s².
const LINEAR_THRUST: f32 = 100.0;
/// Turning thrust applied while holding Q/D, in rad/s².
const ANGULAR_THRUST: f32 = 5.0;

/// How often the engine emits a noise event while active.
const NOISE_EMIT_INTERVAL: f32 = 0.3;

pub(super) fn plugin(app: &mut App) {
    app.add_input_context::<Drone>();

    app.add_observer(bind_drone).add_observer(launch_scan);

    app.add_systems(
        Update,
        apply_input_to_drone
            .in_set(AppSystems::RecordInput)
            .run_if(in_state(Screen::Gameplay))
            .in_set(PausableSystems),
    );
}

/// Marker component for the Drone
#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component)]
pub struct Drone;

/// LDtk Bundle for the Drone
#[derive(Bundle, Default, LdtkEntity)]
pub(super) struct DroneBundle {
    drone: Drone,
    sonar: Sonar,
    sonar_cooldown: SonarCooldown,
    physic: PhysicDroneBundle,
    engine_power: EnginePower,
    engine_noise_timer: EngineNoiseTimer,
}

/// The drone's engine power
#[derive(Component, Reflect, Clone, Copy, Debug)]
#[reflect(Component)]
pub struct EnginePower {
    pub linear: f32,
    pub angular: f32,
}

impl Default for EnginePower {
    fn default() -> Self {
        Self {
            linear: LINEAR_THRUST,
            angular: ANGULAR_THRUST,
        }
    }
}

/// Tracks when the engine should emit its next noise event.
#[derive(Component)]
struct EngineNoiseTimer(Timer);

impl Default for EngineNoiseTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(
            NOISE_EMIT_INTERVAL,
            TimerMode::Repeating,
        ))
    }
}

/// Physic bundle for the Drone
#[derive(Bundle)]
pub(super) struct PhysicDroneBundle {
    rigid_body: RigidBody,
    collider: Collider,
    linear_damping: LinearDamping,
    angular_damping: AngularDamping,
    collision_layers: CollisionLayers,
    linear_acceleration: ConstantLocalLinearAcceleration,
    angular_acceleration: ConstantAngularAcceleration,
}

impl Default for PhysicDroneBundle {
    fn default() -> Self {
        Self {
            rigid_body: RigidBody::Dynamic,
            collider: Collider::triangle(
                0.6 * Vec2::new(-SPRITE_SIZE, -SPRITE_SIZE),
                0.6 * Vec2::new(0., SPRITE_SIZE),
                0.6 * Vec2::new(SPRITE_SIZE, -SPRITE_SIZE),
            ),
            linear_damping: LinearDamping(1.0),
            angular_damping: AngularDamping(4.0),
            collision_layers: CollisionLayers::new(
                PhysicLayer::Drone,
                [PhysicLayer::Terrain, PhysicLayer::Creature],
            ),
            linear_acceleration: ConstantLocalLinearAcceleration::default(),
            angular_acceleration: ConstantAngularAcceleration::default(),
        }
    }
}

#[derive(InputAction)]
#[action_output(bool)]
struct Thrust;

#[derive(InputAction)]
#[action_output(f32)]
struct Rotation;

#[derive(InputAction)]
#[action_output(bool)]
struct Scan;

fn bind_drone(add: On<Add, Drone>, mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.entity(add.entity).insert((
        Sprite {
            image: asset_server.load("images/drone.png"),
            custom_size: Some(Vec2::splat(SPRITE_SIZE)),
            ..default()
        },
        actions!(Drone[
            (Action::<Scan>::new(), bindings![KeyCode::Space]),
            (Action::<Thrust>::new(), bindings![KeyCode::KeyW]),
            (Action::<Rotation>::new(), Bindings::spawn(Bidirectional::new(Binding::from(KeyCode::KeyA), Binding::from(KeyCode::KeyD)))),
        ]),
    ));
}

fn launch_scan(
    _: On<Start<Scan>>,
    mut commands: Commands,
    range: Res<ScanRange>,
    drone: Single<(&GlobalTransform, &mut Sonar, &mut SonarCooldown), With<Drone>>,
) {
    let (transform, mut sonar, mut cooldown) = drone.into_inner();

    if !cooldown.timer.is_finished() {
        return;
    }

    sonar.listen(range.in_pixels());
    cooldown.timer.reset();
    commands.spawn((
        ActiveScan::new(range.in_pixels()),
        Transform::from_translation(transform.translation()),
        Visibility::default(),
    ));
}

fn apply_input_to_drone(
    time: Res<Time>,
    thrust: Single<&Action<Thrust>>,
    rotation: Single<&Action<Rotation>>,
    mut drone: Single<
        (
            &mut ConstantLocalLinearAcceleration,
            &mut ConstantAngularAcceleration,
            &EnginePower,
            &mut EngineNoiseTimer,
            &GlobalTransform,
        ),
        With<Drone>,
    >,
    mut noise_events: MessageWriter<NoiseEvent>,
) {
    let (linear_acc, angular_acc, engine, noise_timer, transform) = &mut *drone;

    let new_linear_acc = thrust.then_some(engine.linear).unwrap_or_default();
    let new_angular_acc = ***rotation * engine.angular;

    linear_acc.0 = Vec2::Y * new_linear_acc;
    angular_acc.0 = new_angular_acc;

    if !noise_timer.0.tick(time.delta()).just_finished() {
        return;
    }

    let loudness = new_linear_acc + new_angular_acc.abs() * 15.;
    if loudness > 0.0 {
        noise_events.write(NoiseEvent::from_loudness(
            transform.translation().truncate(),
            NoiseType::Drone,
            loudness,
        ));
    }
}
