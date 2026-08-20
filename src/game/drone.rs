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
    app.add_systems(
        PostUpdate,
        sync_camera_to_drone
            .after(TransformSystems::Propagate)
            .run_if(in_state(Screen::Gameplay)),
    );
}

/// Marker component for the Drone
#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component)]
pub struct Drone;

/// LDtk Bundle for the Drone
#[derive(Bundle, Default, LdtkEntity)]
pub(super) struct DroneBundle {
    #[sprite_sheet]
    sprite: Sprite,
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
            collider: Collider::circle(8.0),
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

fn bind_drone(add: On<Add, Drone>, mut commands: Commands) {
    commands.entity(add.entity).insert(actions!(Drone[
        (Action::<Scan>::new(), bindings![KeyCode::Space]),
        (Action::<Thrust>::new(), bindings![KeyCode::KeyW]),
        (Action::<Rotation>::new(), Bindings::spawn(Bidirectional::new(Binding::from(KeyCode::KeyA), Binding::from(KeyCode::KeyD)))),
    ]));
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

/// Keeps the camera locked on the drone's position and facing, so it translates
/// and rotates together with it.
fn sync_camera_to_drone(
    drone: Single<&GlobalTransform, With<Drone>>,
    mut camera: Single<&mut Transform, (With<Camera2d>, Without<Drone>)>,
) {
    let (_, rotation, translation) = drone.to_scale_rotation_translation();
    camera.translation.x = translation.x;
    camera.translation.y = translation.y;
    camera.rotation = rotation;
}
