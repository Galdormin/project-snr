//! Panels: the bordered container every group of HUD readouts sits in.

use bevy::prelude::*;

use crate::{
    game::ui::widget::{hrule, label},
    theme::palette::*,
};

/// The default panel size. Fixed, so the HUD doesn't resize itself every changes.
const PANEL_WIDTH: f32 = 560.0;
const PANEL_HEIGHT: f32 = 340.0;
/// Inner padding of the HUD.
/// Also how far the horizontal rules have to bleed back out to reach the panel's border.
const PADDING: f32 = 12.0;

/// Props for [`HudPanel`].
pub struct HudPanelProps {
    /// Shown in the panel's title bar.
    pub title: String,
    /// Readouts pinned to the right of the title bar.
    pub infos: Box<dyn SceneList>,
    /// The panel's contents, stacked vertically. Takes every bit of height the
    /// title bar and the footer leave it.
    pub body: Box<dyn SceneList>,
    /// The strip below the body, ruled off from it. For the panel's controls.
    pub footer: Box<dyn SceneList>,
    /// Outer size of the panel. Fixed, so its contents can't resize the HUD.
    pub size: Vec2,
}

impl Default for HudPanelProps {
    fn default() -> Self {
        Self {
            title: String::new(),
            infos: Box::new(bsn_list!()),
            body: Box::new(bsn_list!()),
            footer: Box::new(bsn_list!()),
            size: Vec2::new(PANEL_WIDTH, PANEL_HEIGHT),
        }
    }
}

/// A bordered, titled panel: the standard container for a group of HUD readouts.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(HudPanelProps)]
#[reflect(Component, Clone, Default)]
pub struct HudPanel;

impl HudPanel {
    fn scene(props: HudPanelProps) -> impl Scene {
        bsn! {
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(8.0),
                padding: {UiRect::all(px(PADDING))},
                border: {UiRect::all(px(1))},
                border_radius: {BorderRadius::all(px(2))},
                width: {px(props.size.x)},
                height: {px(props.size.y)},
            }
            BackgroundColor(HUD_BACKGROUND)
            BorderColor::all(HUD_BORDER)
            Children [
                (
                    Node { align_items: AlignItems::Center, column_gap: px(6.0), margin: {UiRect::bottom(px(2))} }
                    Children [
                        (Node { width: px(3), height: percent(100) } BackgroundColor(HUD_ACCENT)),
                        ({label(props.title)}),
                        (Node { margin: {UiRect::left(auto())}, align_items: AlignItems::Center, column_gap: px(6)} Children [ {props.infos} ] )
                    ]
                ),
                ({hrule()} Node { margin: {UiRect::horizontal(px(-PADDING))} }),
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(8.0),
                        flex_grow: 1.0,
                        min_height: px(0.0),
                    }
                    Children [ {props.body} ]
                ),
                ({hrule()} Node { margin: {UiRect::horizontal(px(-PADDING))} }),
                {props.footer}
            ]
        }
    }
}
