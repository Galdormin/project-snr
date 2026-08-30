//! The in-game HUD, overlaid on the gameplay screen.
//!
//! [`widget`] holds the reusable pieces; each sibling module is one panel.

mod beacon;
mod notes;
mod passive;
mod sonar;
pub mod widget;

use bevy::prelude::*;

use crate::{
    game::ui::widget::{Corner, hud_corner, row},
    screens::Screen,
};

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        widget::plugin,
        beacon::plugin,
        notes::plugin,
        passive::plugin,
        sonar::plugin,
    ));

    app.init_resource::<OpenRightPanel>();

    app.add_systems(OnEnter(Screen::Gameplay), spawn_right_corner);
    app.add_systems(Update, sync_right_panels.run_if(in_state(Screen::Gameplay)));
}

/// One of the panels sharing the bottom-right corner.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Reflect)]
pub enum PanelId {
    #[default]
    Beacon,
    Passive,
}

/// Marks a panel as one of the bottom-right corner's, and says which one.
#[derive(Component, Default, Clone, Copy, PartialEq, Eq, Debug, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct RightPanel(pub PanelId);

/// Which of the bottom-right panels is open, if any.
#[derive(Resource, Default, Debug)]
pub struct OpenRightPanel(Option<PanelId>);

impl OpenRightPanel {
    /// Opens `panel`, or closes it if it was already the one up.
    pub fn toggle(&mut self, panel: PanelId) {
        self.0 = (self.0 != Some(panel)).then_some(panel);
    }
}

/// Spawns the corner the panels and their buttons share, so the buttons sit
/// side by side rather than each panel anchoring its own stack.
fn spawn_right_corner(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        template_value(DespawnOnExit(Screen::Gameplay))
        {hud_corner(Corner::BottomRight, bsn_list![
            ({beacon::panel()}),
            ({passive::panel()}),
            ({row(6.0, bsn_list![
                ({passive::button()}),
                ({beacon::button()}),
            ])}),
        ])}
    });
}

/// Shows the panel that is open and hides the others.
fn sync_right_panels(open: Res<OpenRightPanel>, panels: Query<(&RightPanel, &mut Node)>) {
    if !open.is_changed() {
        return;
    }

    for (panel, mut node) in panels {
        node.display = match open.0 == Some(panel.0) {
            true => Display::Flex,
            false => Display::None,
        };
    }
}
