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
    game::{PhysicLayer, scan::ActiveScan},
    screens::Screen,
};

/// Drone sprite size
const SPRITE_SIZE: f32 = 8.0;
/// Forward/backward thrust applied while holding Z/S, in m/s².
const LINEAR_THRUST: f32 = 400.0;
/// Turning thrust applied while holding Q/D, in rad/s².
const ANGULAR_THRUST: f32 = 20.0;

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
    physic: PhysicDroneBundle,
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
                Vec2::new(-SPRITE_SIZE, -SPRITE_SIZE),
                Vec2::new(0., SPRITE_SIZE),
                Vec2::new(SPRITE_SIZE, -SPRITE_SIZE),
            ),
            linear_damping: LinearDamping(1.0),
            angular_damping: AngularDamping(4.0),
            collision_layers: CollisionLayers::new(PhysicLayer::Drone, [PhysicLayer::Terrain]),
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
    drone: Single<&GlobalTransform, With<Drone>>,
) {
    commands.spawn((
        ActiveScan::new(300.0),
        Transform::from_translation(drone.translation()),
    ));
}

fn apply_input_to_drone(
    thrust: Single<&Action<Thrust>>,
    rotation: Single<&Action<Rotation>>,
    mut drone: Single<
        (
            &mut ConstantLocalLinearAcceleration,
            &mut ConstantAngularAcceleration,
        ),
        With<Drone>,
    >,
) {
    let (linear_acc, angular_acc) = &mut *drone;

    linear_acc.0 = Vec2::Y * thrust.then_some(1.0f32).unwrap_or_default() * LINEAR_THRUST;
    angular_acc.0 = ***rotation * ANGULAR_THRUST;
}
