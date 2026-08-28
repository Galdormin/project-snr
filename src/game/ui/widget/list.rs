//! Vertical lists that stay a sane size.

use bevy::{prelude::*, ui_widgets::ScrollArea};

/// Height of one table row
pub const ROW_HEIGHT: f32 = 22.0;
/// Gap between rows
const ROW_GAP: f32 = 2.0;

/// A vertical list that takes whatever space its panel has left, then scrolls.
pub fn fill_list(content: impl SceneList) -> impl Scene {
    bsn! {
        ScrollArea
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: {px(ROW_GAP)},
            flex_grow: 1.0,
            // Flex items refuse to shrink below their contents by default,
            // which would push the list straight through the panel's border
            // instead of scrolling.
            min_height: px(0.0),
            overflow: {Overflow::scroll()},
        }
        Children [ {content} ]
    }
}

/// A vertical list that grows with its contents up to `max_rows`, then scrolls.
pub fn scroll_list(max_rows: usize, content: impl SceneList) -> impl Scene {
    let max_height = max_rows as f32 * ROW_HEIGHT + (max_rows.saturating_sub(1)) as f32 * ROW_GAP;
    bsn! {
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
