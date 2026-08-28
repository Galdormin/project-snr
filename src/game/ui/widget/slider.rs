//! A horizontal slider in the HUD.

use bevy::{
    prelude::*,
    ui_widgets::{Slider, SliderOrientation, SliderRange, SliderValue, TrackClick},
};

use crate::theme::palette::*;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, style_sliders);
}

/// Props for [`HudSlider`].
pub struct HudSliderProps {
    pub value: f32,
    pub min: f32,
    pub max: f32,
    /// Track width, in px.
    pub width: f32,
    /// Track height, in px
    pub height: f32,
    /// Runs on every adjustment, dragging included.
    pub on_change: Box<dyn Scene>,
}

impl Default for HudSliderProps {
    fn default() -> Self {
        Self {
            value: 0.0,
            min: 0.0,
            max: 1.0,
            width: 120.0,
            height: 10.0,
            on_change: Box::new(bsn!()),
        }
    }
}

/// A filled-track slider.
///
/// Bevy's slider is headless and doesn't own its value: it reports adjustments
/// as [`ValueChange<f32>`] and leaves writing [`SliderValue`] back to the
/// handler, which is what lets a panel keep the value in its own state.
///
/// ```ignore
/// (@HudSlider { @min: 0.0, @max: 10.0, @value: 5.0, @on_change: {bsn!(on(my_observer))} })
/// ```
///
/// [`ValueChange<f32>`]: bevy::ui_widgets::ValueChange
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(HudSliderProps)]
#[require(Slider)]
#[reflect(Component, Clone, Default)]
pub struct HudSlider;

impl HudSlider {
    fn scene(props: HudSliderProps) -> impl Scene {
        bsn! {
            template_value(Slider {
                track_click: TrackClick::Snap,
                orientation: SliderOrientation::Horizontal,
            })
            template_value(SliderValue(props.value))
            template_value(SliderRange::new(props.min, props.max))
            Node {
                width: {px(props.width)},
                height: {px(props.height)},
                border: {UiRect::all(px(1))},
                border_radius: {BorderRadius::all(px(2))},
                align_items: AlignItems::Center,
                // The fill is sized in percent of the track, so it has to be
                // cut off at the border rather than rounded past it.
                overflow: {Overflow::clip()},
            }
            BackgroundColor(HUD_FIELD_BACKGROUND)
            BorderColor::all(HUD_BORDER)
            {props.on_change}
            Children [
                (
                    HudSliderFill
                    Node { width: percent(0.0), height: percent(100.0) }
                    BackgroundColor(HUD_ACCENT)
                    // Let the drag land on the slider itself.
                    Pickable::IGNORE
                ),
            ]
        }
    }
}

/// The part of a [`HudSlider`]'s track that is filled in, up to its value.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct HudSliderFill;

/// Sizes each slider's fill to its value. Bevy's slider styles nothing itself.
fn style_sliders(
    sliders: Query<
        (&SliderValue, &SliderRange, &Children),
        (
            With<HudSlider>,
            Or<(
                Changed<SliderValue>,
                Changed<SliderRange>,
                Changed<Children>,
            )>,
        ),
    >,
    mut fills: Query<&mut Node, With<HudSliderFill>>,
) {
    for (value, range, children) in &sliders {
        let filled = range.thumb_position(value.0);
        for child in children {
            if let Ok(mut fill) = fills.get_mut(*child) {
                fill.width = percent(filled * 100.0);
            }
        }
    }
}
