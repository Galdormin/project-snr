//! The HUD's clickable button.

use bevy::prelude::*;

use crate::{
    game::ui::widget::label,
    theme::{interaction::InteractionPalette, palette::*},
};

/// Props for [`HudButton`].
pub struct HudButtonProps {
    pub label: String,
    /// Runs when the button is clicked.
    pub on_click: Box<dyn Scene>,
}

impl Default for HudButtonProps {
    fn default() -> Self {
        Self {
            label: String::new(),
            on_click: Box::new(bsn!()),
        }
    }
}

/// A compact bordered button in the HUD's chrome colors.
///
/// Pass behavior through `on_click`, e.g.
/// `@HudButton { @label: "PING", @on_click: {bsn!(on(|_: On<Pointer<Click>>| { .. }))} }`.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(HudButtonProps)]
#[reflect(Component, Clone, Default)]
pub struct HudButton;

impl HudButton {
    fn scene(props: HudButtonProps) -> impl Scene {
        bsn! {
            Button
            Node {
                padding: {UiRect::axes(px(10), px(5))},
                border: {UiRect::all(px(1))},
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: {BorderRadius::all(px(2))},
            }
            BackgroundColor(HUD_BACKGROUND)
            BorderColor::all(HUD_BORDER)
            template_value(InteractionPalette {
                none: HUD_BACKGROUND,
                hovered: HUD_BACKGROUND_HOVERED,
                pressed: HUD_BACKGROUND_HOVERED,
            })
            {props.on_click}
            Children [
                // Don't bubble picking events from the text up to the button.
                ({label(props.label)} Pickable::IGNORE),
            ]
        }
    }
}
