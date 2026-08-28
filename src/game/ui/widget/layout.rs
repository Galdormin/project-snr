//! Screen anchoring and the boxes that arrange a panel's contents.

use bevy::prelude::*;

use crate::{game::ui::widget::SWATCH_SIZE, theme::palette::*};

/// Which screen corner a [`hud_corner`] anchors its contents to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    #[default]
    BottomRight,
}

impl Corner {
    fn align(&self) -> (AlignItems, JustifyContent) {
        let horizontal = match self {
            Self::TopLeft | Self::BottomLeft => AlignItems::Start,
            Self::TopRight | Self::BottomRight => AlignItems::End,
        };
        let vertical = match self {
            Self::TopLeft | Self::TopRight => JustifyContent::Start,
            Self::BottomLeft | Self::BottomRight => JustifyContent::End,
        };
        (horizontal, vertical)
    }
}

/// A screen-filling, click-through root that stacks its children in one corner.
///
/// One of these per corner is enough: give it as many panels as you like.
pub fn hud_corner(corner: Corner, content: impl SceneList) -> impl Scene {
    let (align_items, justify_content) = corner.align();
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            width: percent(100.0),
            height: percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: {align_items},
            justify_content: {justify_content},
            padding: {UiRect::all(px(16))},
            row_gap: px(6),
        }
        // The root spans the screen: let gameplay clicks through it.
        Pickable::IGNORE
        Children [ {content} ]
    }
}

/// A table: one grid of `columns` `auto`-sized tracks, fed a flat list of cells.
pub fn grid(columns: u16, cells: impl SceneList) -> impl Scene {
    bsn! {
        Node {
            display: Display::Grid,
            grid_template_columns: {RepeatedGridTrack::auto::<Vec<_>>(columns)},
            column_gap: px(6.0),
            row_gap: px(2.0),
            align_items: AlignItems::Center,
        }
        Children [ {cells} ]
    }
}

/// A thin vertical divider
pub fn vrule() -> impl Scene {
    bsn! {
        Node { width: px(1.0), height: {px(SWATCH_SIZE)} }
        BackgroundColor(HUD_BORDER)
    }
}

/// A thin horizontal divider.
pub fn hrule() -> impl Scene {
    bsn! {
        Node { height: px(1.0) }
        BackgroundColor(HUD_BORDER)
    }
}

/// A horizontal group of widgets, vertically centered.
pub fn row(gap: f32, content: impl SceneList) -> impl Scene {
    bsn! {
        Node {
            align_items: AlignItems::Center,
            column_gap: {px(gap)}
        }
        Children [ {content} ]
    }
}
