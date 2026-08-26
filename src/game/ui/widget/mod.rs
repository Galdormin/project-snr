//! Reusable building blocks for the in-game HUD.
//!
//! Everything here is theme-only: widgets never reach into gameplay state.
//! Stateful widgets ([`ColorPicker`]) instead expose a component to write to
//! and an event to observe, so each panel binds them to its own resource.
//!
//! One module per widget family; everything is re-exported here, so callers
//! only ever need `crate::game::ui::widget::*`.

// Widgets are built for reuse, so not every variant is used yet.
#![allow(dead_code)]

mod button;
mod color_picker;
mod layout;
mod list;
mod panel;
mod text;

pub use button::*;
pub use color_picker::*;
pub use layout::*;
pub use list::*;
pub use panel::*;
pub use text::*;

use bevy::{prelude::*, text::FontSize};

/// Body text size across the HUD.
pub const FONT_SIZE: FontSize = FontSize::Px(13.0);
/// Side of a square swatch, and the height of a [`rule`].
pub const SWATCH_SIZE: f32 = 18.0;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((color_picker::plugin, text::plugin));
}
