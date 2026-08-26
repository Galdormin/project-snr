//! Text, from plain labels to the lockable field used for renaming rows.

use bevy::{
    input_focus::{FocusCause, FocusLost, InputFocus},
    prelude::*,
    text::{EditableText, TextCursorStyle},
};

use crate::{game::ui::widget::FONT_SIZE, theme::palette::*};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, mirror_editor_into_label);
    app.add_observer(lock_text_field);
    app.add_observer(release_focus_on_outside_press);
}

/// HUD body text.
pub fn label(text: impl Into<String>) -> impl Scene {
    let text = text.into();
    bsn! { Text({text}) TextFont { font_size: FONT_SIZE } TextColor(HUD_TEXT) }
}

/// HUD body text, one step dimmer than [`label`]. For row captions and units.
pub fn label_dim(text: impl Into<String>) -> impl Scene {
    let text = text.into();
    bsn! {
        Text({text})
        TextFont { font_size: FONT_SIZE }
        TextColor(HUD_TEXT_DIM)
    }
}

/// A small filled tag, for drawing attention to a row.
pub fn badge(text: impl Into<String>) -> impl Scene {
    let text = text.into();
    bsn! {
        Node {
            padding: {UiRect::axes(px(4), px(1))},
            border_radius: {BorderRadius::all(px(2))},
            align_items: AlignItems::Center,
        }
        BackgroundColor(HUD_ACCENT)
        Children [
            (
                Text({text})
                TextFont { font_size: {FontSize::Px(10.0)} }
                TextColor(HUD_BACKGROUND)
                Pickable::IGNORE
            ),
        ]
    }
}

/// A single-line text field that starts locked.
///
/// It only accepts typing after a double-click, and locks itself again the
/// moment focus moves elsewhere.
#[derive(Component, Clone, FromTemplate, Reflect)]
#[reflect(Component, Clone)]
pub struct TextField {
    /// Read-only label, shown while locked. Always holds the committed value.
    pub label: Entity,
    /// The editor, shown only while unlocked.
    pub editor: Entity,
}

impl Default for TextField {
    fn default() -> Self {
        Self {
            label: Entity::PLACEHOLDER,
            editor: Entity::PLACEHOLDER,
        }
    }
}

/// On a [`TextField`]'s editor: points back at the field that owns it.
#[derive(Component, Clone, FromTemplate, Reflect)]
#[reflect(Component, Clone)]
pub struct TextFieldEditor(pub Entity);

impl Default for TextFieldEditor {
    fn default() -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

pub fn text_field(text: impl Into<String>, width: f32) -> impl Scene {
    let text = text.into();
    let initial = text.clone();
    bsn! {
        #Field
        TextField { label: #Label, editor: #Editor }
        Node {
            width: {px(width)},
            padding: {UiRect::axes(px(4), px(1))},
            border: {UiRect::all(px(1))},
            border_radius: {BorderRadius::all(px(2))},
            align_items: AlignItems::Center,
            // A value longer than the field is cut off rather than drawn over
            // whatever sits next to it.
            overflow: {Overflow::clip()},
        }
        on(unlock_text_field)
        Children [
            (
                #Label
                Text({text})
                TextLayout::no_wrap()
                TextFont { font_size: FONT_SIZE }
                TextColor(HUD_TEXT)
                // Let the double-click land on the field itself.
                Pickable::IGNORE
            ),
            (
                #Editor
                TextFieldEditor(#Field)
                template_value(EditableText {
                    visible_width: Some(width),
                    allow_newlines: false,
                    ..EditableText::new(initial)
                })
                TextLayout::no_wrap()
                TextCursorStyle
                TextFont { font_size: FONT_SIZE }
                TextColor(HUD_TEXT)
                Node { display: Display::None }
            ),
        ]
    }
}

/// Double-clicking a locked field reveals its editor and focuses it.
fn unlock_text_field(
    click: On<Pointer<Click>>,
    mut commands: Commands,
    mut focus: ResMut<InputFocus>,
    fields: Query<&TextField>,
    mut nodes: Query<&mut Node>,
) {
    if click.count < 2 {
        return;
    }

    let field_entity = click.event_target();
    let Ok(field) = fields.get(field_entity) else {
        return;
    };

    if let Ok(mut label) = nodes.get_mut(field.label) {
        label.display = Display::None;
    }
    if let Ok(mut editor) = nodes.get_mut(field.editor) {
        editor.display = Display::Flex;
    }

    focus.set(field.editor, FocusCause::Pressed);
    commands.entity(field_entity).insert((
        BorderColor::all(HUD_ACCENT),
        BackgroundColor(HUD_FIELD_BACKGROUND),
    ));
}

/// Losing focus hides the editor again and leaves the label showing the result.
fn lock_text_field(
    lost: On<FocusLost>,
    mut commands: Commands,
    editors: Query<&TextFieldEditor>,
    fields: Query<&TextField>,
    mut nodes: Query<&mut Node>,
) {
    let Ok(editor) = editors.get(lost.entity) else {
        return;
    };
    let Ok(field) = fields.get(editor.0) else {
        return;
    };

    if let Ok(mut label) = nodes.get_mut(field.label) {
        label.display = Display::Flex;
    }
    if let Ok(mut editor) = nodes.get_mut(field.editor) {
        editor.display = Display::None;
    }

    commands
        .entity(editor.0)
        .remove::<BorderColor>()
        .remove::<BackgroundColor>();
}

/// Nothing in Bevy drops focus when you click away from a widget, so pressing
/// anywhere that isn't the focused editor does it here, which locks the field.
fn release_focus_on_outside_press(
    press: On<Pointer<Press>>,
    mut focus: ResMut<InputFocus>,
    editors: Query<(), With<TextFieldEditor>>,
) {
    let Some(focused) = focus.get() else {
        return;
    };

    // Compare against the entity actually hit: `Pointer` events also bubble to
    // ancestors, and those would otherwise all look like outside clicks.
    if editors.contains(focused) && press.original_event_target() != focused {
        focus.clear();
    }
}

/// Mirrors the live editor value into the locked label, so the label is always
/// the field's committed value and callers only ever need to read one place.
fn mirror_editor_into_label(
    editors: Query<(&TextFieldEditor, &EditableText)>,
    fields: Query<&TextField>,
    mut labels: Query<&mut Text>,
) {
    for (editor, editable) in &editors {
        let Ok(field) = fields.get(editor.0) else {
            continue;
        };
        let Ok(mut label) = labels.get_mut(field.label) else {
            continue;
        };

        let value = editable.value().to_string();
        if label.0 != value {
            label.0 = value;
        }
    }
}
