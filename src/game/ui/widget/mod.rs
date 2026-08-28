//! Reusable building blocks for the in-game HUD.

// Widgets are built for reuse, so not every variant is used yet.
#![allow(dead_code)]

mod button;
mod color_picker;
mod layout;
mod list;
mod panel;
mod slider;
mod text;

pub use button::*;
pub use color_picker::*;
pub use layout::*;
pub use list::*;
pub use panel::*;
pub use slider::*;
pub use text::*;

use bevy::{prelude::*, text::FontSize};

/// Body text size across the HUD.
pub const FONT_SIZE: FontSize = FontSize::Px(13.0);
/// Side of a square swatch, and the height of a `rule`.
pub const SWATCH_SIZE: f32 = 18.0;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((color_picker::plugin, slider::plugin, text::plugin));
}
