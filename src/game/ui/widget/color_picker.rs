//! Picking a color, and reading one back.

use bevy::prelude::*;

use crate::{game::ui::widget::SWATCH_SIZE, theme::palette::*};

/// Marks a [`readout_dot`], so panels can target it without also matching every
/// other node that happens to carry a [`BackgroundColor`].
#[derive(Component, Clone, Copy, Default, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct ReadoutDot;

/// A small dot showing a color readout. Write its [`BackgroundColor`] to update it.
pub fn readout_dot() -> impl Scene {
    bsn! {
        ReadoutDot
        Node {
            width: px(11.0),
            height: px(11.0),
            border: {UiRect::all(px(1))},
            border_radius: BorderRadius::MAX,
        }
        BorderColor::all(HUD_BORDER)
    }
}

/// Currently selected color of a [`ColorPicker`].
///
/// The picker renders from this, so write to it to drive the widget from your
/// own state; the picker never writes it itself.
#[derive(Component, Clone, Copy, Default, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct SelectedColor(pub Color);

/// Emitted on a [`ColorPicker`] when the user clicks one of its swatches.
///
/// Observe it to update your state, then write [`SelectedColor`] back.
#[derive(EntityEvent, Clone, Copy, Reflect)]
#[reflect(Event)]
pub struct ColorPicked {
    /// The [`ColorPicker`] whose swatch was clicked.
    pub entity: Entity,
    pub color: Color,
}

/// Marks a picker's swatch, so the picker can style it. Holds the swatch's color.
#[derive(Component, Clone, Copy, Default, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct ColorSwatch(pub Color);

/// A row of clickable color swatches.
///
/// Spawn it with a [`SelectedColor`] and observe [`ColorPicked`]:
/// ```ignore
/// (@ColorPicker { @colors: {vec![RED, BLUE]} } SelectedColor(RED) on(my_observer))
/// ```
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(ColorPickerProps)]
#[reflect(Component, Clone, Default)]
pub struct ColorPicker;

#[derive(Default)]
pub struct ColorPickerProps {
    /// The colors to offer, left to right.
    pub colors: Vec<Color>,
}

impl ColorPicker {
    fn scene(props: ColorPickerProps) -> impl Scene {
        let swatches: Vec<_> = props.colors.into_iter().map(swatch).collect();
        bsn! {
            Node { column_gap: px(4.0) }
            Children [ {swatches} ]
        }
    }
}

fn swatch(color: Color) -> impl Scene {
    bsn! {
        Button
        template_value(ColorSwatch(color))
        Node {
            width: {px(SWATCH_SIZE)},
            height: {px(SWATCH_SIZE)},
            border: {UiRect::all(px(2))},
            border_radius: {BorderRadius::all(px(2))},
        }
        template_value(BackgroundColor(color))
        // Styled by `style_swatches`.
        BorderColor::all(Color::NONE)
        on(|click: On<Pointer<Click>>,
            mut commands: Commands,
            swatches: Query<(&ColorSwatch, &ChildOf)>| {
            let Ok((swatch, child_of)) = swatches.get(click.event_target()) else { return };
            commands.trigger(ColorPicked { entity: child_of.parent(), color: swatch.0 });
        })
    }
}

/// Borders every swatch to show which one its picker has selected, and which
/// one the pointer is over.
fn style_swatches(
    pickers: Query<&SelectedColor>,
    mut swatches: Query<(&ColorSwatch, &ChildOf, &Interaction, &mut BorderColor)>,
) {
    for (swatch, child_of, interaction, mut border) in &mut swatches {
        let selected = pickers
            .get(child_of.parent())
            .is_ok_and(|selected| selected.0 == swatch.0);

        *border = BorderColor::all(match (selected, interaction) {
            (true, _) => HUD_ACCENT,
            (false, Interaction::None) => Color::NONE,
            (false, _) => HUD_BORDER,
        });
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, style_swatches);
}
