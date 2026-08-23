//! Development tools for the game. This plugin is only enabled in dev builds.

use avian2d::prelude::{PhysicsDebugPlugin, PhysicsGizmos};
use bevy::{
    dev_tools::states::log_transitions, input::common_conditions::input_just_pressed, prelude::*,
};

use crate::{
    game::creatures::steer_glide::{ContextMapDanger, ContextMapInterest, SLOT_ANGLE, STEER_SLOTS},
    screens::Screen,
};

pub(super) fn plugin(app: &mut App) {
    // Log `Screen` state transitions.
    app.add_systems(Update, log_transitions::<Screen>);

    // Toggle the debug overlay for UI.
    app.add_systems(
        Update,
        toggle_debug_ui.run_if(input_just_pressed(TOGGLE_KEY)),
    );

    // Toggle Avian2d's hitbox/collider debug rendering.
    app.add_plugins(PhysicsDebugPlugin).insert_gizmo_config(
        PhysicsGizmos::default(),
        GizmoConfig {
            enabled: false,
            ..default()
        },
    );
    app.add_systems(
        Update,
        toggle_debug_physics.run_if(input_just_pressed(TOGGLE_PHYSICS_KEY)),
    );

    // Toggle the creatures' context steering debug gizmos.
    app.insert_gizmo_config(
        SteeringGizmos,
        GizmoConfig {
            enabled: false,
            ..default()
        },
    );
    app.add_systems(
        Update,
        (
            draw_context_maps,
            toggle_debug_steering.run_if(input_just_pressed(TOGGLE_STEERING_KEY)),
        ),
    );
}

const TOGGLE_KEY: KeyCode = KeyCode::Backquote;
const TOGGLE_PHYSICS_KEY: KeyCode = KeyCode::F1;
const TOGGLE_STEERING_KEY: KeyCode = KeyCode::F2;

/// GizmoGroup for the Context Steering algorithm
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct SteeringGizmos;

fn toggle_debug_ui(mut options: ResMut<GlobalUiDebugOptions>) {
    options.toggle();
}

fn toggle_debug_physics(mut gizmo_config: ResMut<GizmoConfigStore>) {
    let config = gizmo_config.config_mut::<PhysicsGizmos>().0;
    config.enabled = !config.enabled;
}

fn toggle_debug_steering(mut gizmo_config: ResMut<GizmoConfigStore>) {
    let config = gizmo_config.config_mut::<SteeringGizmos>().0;
    config.enabled = !config.enabled;
}

/// Green = interest, red = danger.
fn draw_context_maps(
    mut gizmos: Gizmos<SteeringGizmos>,
    creatures: Query<(&ContextMapDanger, &ContextMapInterest, &GlobalTransform)>,
) {
    for (danger, interest, transform) in &creatures {
        let pos = transform.translation().truncate();
        for s in 0..STEER_SLOTS {
            let d = Dir2::from_angle(s as f32 * SLOT_ANGLE).as_vec2();
            gizmos.line_2d(
                pos,
                pos + d * interest.0[s] * 20.0,
                Color::srgb(0.0, 1.0, 0.6),
            );
            gizmos.line_2d(
                pos,
                pos + d * danger.0[s] * -20.0,
                Color::srgb(1.0, 0.2, 0.2),
            );
        }
    }
}
