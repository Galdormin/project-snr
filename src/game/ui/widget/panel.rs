//! Panels: the bordered container, and the drawer that folds one away.

use bevy::prelude::*;

use crate::{
    game::ui::widget::{HudButton, label},
    theme::palette::*,
};

/// Props for [`HudPanel`].
pub struct HudPanelProps {
    /// Shown in the panel's title bar, after the accent tick.
    pub title: String,
    /// The panel's contents, stacked vertically.
    pub body: Box<dyn SceneList>,
}

impl Default for HudPanelProps {
    fn default() -> Self {
        Self {
            title: String::new(),
            body: Box::new(bsn_list!()),
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
                padding: {UiRect::all(px(12))},
                border: {UiRect::all(px(1))},
                border_radius: {BorderRadius::all(px(2))},
            }
            BackgroundColor(HUD_BACKGROUND)
            BorderColor::all(HUD_BORDER)
            Children [
                (
                    Node { align_items: AlignItems::Center, column_gap: px(6.0), margin: {UiRect::bottom(px(2))} }
                    Children [
                        // Accent tick: the HUD's section marker.
                        (Node { width: px(3.0), height: px(11.0) } BackgroundColor(HUD_ACCENT)),
                        ({label(props.title)}),
                    ]
                ),
                {props.body},
            ]
        }
    }
}

/// Props for [`HudDrawer`].
pub struct HudDrawerProps {
    /// Title of the panel that opens.
    pub title: String,
    /// Label of the toggle button, shown while the drawer is closed.
    pub label: String,
    /// Contents of the panel that opens.
    pub body: Box<dyn SceneList>,
}

impl Default for HudDrawerProps {
    fn default() -> Self {
        Self {
            title: String::new(),
            label: String::new(),
            body: Box::new(bsn_list!()),
        }
    }
}

/// A [`HudPanel`] that stays folded into the HUD until its toggle is clicked.
///
/// The toggle sits below the panel so it keeps its place in the corner as the
/// panel grows and shrinks.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(HudDrawerProps)]
#[reflect(Component, Clone, Default)]
pub struct HudDrawer;

/// Label shown on a [`HudDrawer`] toggle while the drawer is open.
const DRAWER_CLOSE_LABEL: &str = "[  X  ]";

impl HudDrawer {
    fn scene(props: HudDrawerProps) -> impl Scene {
        let closed_label = props.label.clone();
        bsn! {
            Node { flex_direction: FlexDirection::Column, align_items: AlignItems::End, row_gap: px(6.0) }
            Pickable::IGNORE
            Children [
                (
                    #Panel
                    @HudPanel { @title: {props.title}, @body: {props.body} }
                    // Hidden until the toggle is clicked.
                    Node { display: Display::None }
                ),
                (
                    @HudButton { @label: {props.label}, @on_click: {bsn!(on(toggle_drawer))} }
                    HudDrawerToggle { panel: #Panel, closed_label: {closed_label} }
                ),
            ]
        }
    }
}

/// On a [`HudDrawer`]'s toggle button: what it opens, and how to label itself.
///
/// [`FromTemplate`] is what lets `bsn!` fill `panel` from a `#Name` reference.
#[derive(Component, Clone, FromTemplate, Reflect)]
#[reflect(Component, Clone)]
pub struct HudDrawerToggle {
    /// The panel this toggle shows and hides.
    pub panel: Entity,
    /// Label to restore when the drawer closes.
    pub closed_label: String,
}

impl Default for HudDrawerToggle {
    fn default() -> Self {
        Self {
            panel: Entity::PLACEHOLDER,
            closed_label: String::new(),
        }
    }
}

/// Fired on a [`HudDrawer`]'s panel each time the player folds it in or out.
///
/// Note that "unread" markers want to be cleared on *close*: clearing them on
/// open would wipe them in the very click that first reveals them.
#[derive(EntityEvent, Clone, Copy, Reflect)]
#[reflect(Event)]
pub struct HudDrawerToggled {
    /// The panel that was toggled.
    pub entity: Entity,
    pub open: bool,
}

/// Folds a [`HudDrawer`]'s panel in and out, and relabels its toggle to match.
fn toggle_drawer(
    click: On<Pointer<Click>>,
    mut commands: Commands,
    toggles: Query<(&HudDrawerToggle, &Children)>,
    mut panels: Query<&mut Node, With<HudPanel>>,
    mut labels: Query<&mut Text>,
) {
    let Ok((toggle, children)) = toggles.get(click.event_target()) else {
        return;
    };
    let Ok(mut panel) = panels.get_mut(toggle.panel) else {
        return;
    };

    panel.display = match panel.display {
        Display::None => Display::Flex,
        _ => Display::None,
    };

    commands.trigger(HudDrawerToggled {
        entity: toggle.panel,
        open: panel.display != Display::None,
    });

    // A `HudButton`'s only child is its label.
    let Some(&label) = children.first() else {
        return;
    };
    if let Ok(mut label) = labels.get_mut(label) {
        label.0 = match panel.display {
            Display::None => toggle.closed_label.clone(),
            _ => DRAWER_CLOSE_LABEL.to_string(),
        };
    }
}
