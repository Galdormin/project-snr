//! The in-game HUD, overlaid on the gameplay screen.
//!
//! [`widget`] holds the reusable pieces; each sibling module is one panel.

mod sonar;
pub mod widget;

use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((widget::plugin, sonar::plugin));
}
