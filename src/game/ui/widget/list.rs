//! Vertical lists that stay a sane size.

use bevy::{prelude::*, ui_widgets::ScrollArea};

/// Height of one table row, and the unit [`scroll_list`] sizes itself in.
pub const ROW_HEIGHT: f32 = 22.0;
/// Gap between rows in a [`scroll_list`].
const ROW_GAP: f32 = 2.0;

/// A vertical list that grows with its contents up to `max_rows`, then scrolls.
///
/// Sizing to the contents rather than to a fixed frame keeps the panel from
/// advertising how many rows the player has yet to fill in.
pub fn scroll_list(max_rows: usize, content: impl SceneList) -> impl Scene {
    let max_height = max_rows as f32 * ROW_HEIGHT + (max_rows.saturating_sub(1)) as f32 * ROW_GAP;
    bsn! {
        // Enables trackpad / wheel scrolling once the contents overflow.
        ScrollArea
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: {px(ROW_GAP)},
            max_height: {px(max_height)},
            overflow: {Overflow::scroll_y()},
        }
        Children [ {content} ]
    }
}
