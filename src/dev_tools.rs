//! Development tools for the game. This plugin is only enabled in dev builds.

use avian2d::prelude::{PhysicsDebugPlugin, PhysicsGizmos};
use bevy::{
    dev_tools::states::log_transitions, input::common_conditions::input_just_pressed, prelude::*,
};

use crate::screens::Screen;

pub(super) fn plugin(app: &mut App) {
    // Log `Screen` state transitions.
    app.add_systems(Update, log_transitions::<Screen>);

    // Toggle the debug overlay for UI.
    app.add_systems(
        Update,
        toggle_debug_ui.run_if(input_just_pressed(TOGGLE_KEY)),
    );

    // Toggle Avian2d's hitbox/collider debug rendering.
    app.add_plugins(PhysicsDebugPlugin);
    app.add_systems(
        Update,
        toggle_debug_physics.run_if(input_just_pressed(TOGGLE_PHYSICS_KEY)),
    );
}

const TOGGLE_KEY: KeyCode = KeyCode::Backquote;
const TOGGLE_PHYSICS_KEY: KeyCode = KeyCode::F1;

fn toggle_debug_ui(mut options: ResMut<GlobalUiDebugOptions>) {
    options.toggle();
}

fn toggle_debug_physics(mut gizmo_config: ResMut<GizmoConfigStore>) {
    let config = gizmo_config.config_mut::<PhysicsGizmos>().0;
    config.enabled = !config.enabled;
}
