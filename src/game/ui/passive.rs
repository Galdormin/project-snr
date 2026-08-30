//! The passive probe listening panel: how loud channels are.

use bevy::{
    prelude::*,
    ui_widgets::{SliderValue, ValueChange},
};
use bevy_seedling::prelude::VolumeNode;

use crate::{
    game::{
        sound::{AmbientBus, DEFAULT_AMBIENT_GAIN, DEFAULT_SONAR_GAIN, SonarBus, VOLUME},
        ui::{
            OpenRightPanel, PanelId, RightPanel,
            widget::{HudButton, HudPanel, HudSlider, label, label_dim, row},
        },
    },
    screens::Screen,
};

/// Panel size.
const PANEL_SIZE: Vec2 = Vec2::new(380., 120.);
/// Track width of the gain sliders.
const SLIDER_WIDTH: f32 = 180.0;
/// Width of a row's caption, so both sliders line up under each other.
const CAPTION_WIDTH: f32 = 70.0;
/// Width of the percentage readout, so the row doesn't shift as the number
/// gains and loses digits.
const READOUT_WIDTH: f32 = 44.0;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, sync_readouts.run_if(in_state(Screen::Gameplay)));
}

/// Which bus a slider drives, and which one a readout reports.
#[derive(Component, Default, Clone, Copy, PartialEq, Eq, Debug, Reflect)]
#[reflect(Component, Default, Clone)]
enum Channel {
    /// The drone's own ping.
    #[default]
    Sonar,
    /// Everything else that reaches the hydrophone.
    Ambient,
}

impl Channel {
    fn default_gain(&self) -> f32 {
        match self {
            Channel::Sonar => DEFAULT_SONAR_GAIN,
            Channel::Ambient => DEFAULT_AMBIENT_GAIN,
        }
    }
}

/// The gain panel itself. The corner it lives in is [`crate::game::ui`]'s.
pub(super) fn panel() -> impl Scene {
    bsn! {
        (
            @HudPanel {
                @title: "PASSIVE :: HYDROPHONE",
                @size: {PANEL_SIZE},
                @body: {Box::new(bsn_list![
                    ({gain_row("SONAR", Channel::Sonar)}),
                    ({gain_row("AMBIENT", Channel::Ambient)}),
                ]) as Box<dyn SceneList>},
            }
            template_value(RightPanel(PanelId::Passive))
            Node { display: Display::None }
        )
    }
}

/// The button that opens and closes [`panel`].
pub(super) fn button() -> impl Scene {
    bsn! {
        @HudButton {
            @label: "[ PASSIVE ]",
            @on_click: {bsn!(on(toggle_panel))},
        }
    }
}

/// One captioned slider and its readout, wired to `channel`.
fn gain_row(caption: &'static str, channel: Channel) -> impl Scene {
    row(
        8.0,
        bsn_list![
            ({label_dim(caption)} Node { width: {px(CAPTION_WIDTH)} }),
            (
                @HudSlider {
                    @value: {channel.default_gain()},
                    @width: {SLIDER_WIDTH},
                    @on_change: {bsn!(on(apply_gain))},
                }
                template_value(channel)
            ),
            (
                Node { width: {px(READOUT_WIDTH)}, justify_content: JustifyContent::End }
                Children [ ({label(readout_text(channel.default_gain()))} template_value(channel)) ]
            ),
        ],
    )
}

fn toggle_panel(_: On<Pointer<Click>>, mut open: ResMut<OpenRightPanel>) {
    open.toggle(PanelId::Passive);
}

fn apply_gain(
    change: On<ValueChange<f32>>,
    mut commands: Commands,
    sliders: Query<&Channel>,
    // One query over both buses: two `Single`s filtered on different markers
    // would look like conflicting access to `VolumeNode`.
    buses: Query<(&mut VolumeNode, Has<SonarBus>), Or<(With<SonarBus>, With<AmbientBus>)>>,
) {
    let Ok(&channel) = sliders.get(change.source) else {
        return;
    };

    commands
        .entity(change.source)
        .insert(SliderValue(change.value));

    let wants_sonar = channel == Channel::Sonar;
    for (mut bus, is_sonar) in buses {
        if is_sonar == wants_sonar {
            bus.volume = VOLUME.perceptual_to_volume(change.value);
        }
    }
}

fn sync_readouts(
    buses: Query<
        (&VolumeNode, Has<SonarBus>),
        (Or<(With<SonarBus>, With<AmbientBus>)>, Changed<VolumeNode>),
    >,
    mut readouts: Query<(&Channel, &mut Text)>,
) {
    for (bus, is_sonar) in &buses {
        let changed = match is_sonar {
            true => Channel::Sonar,
            false => Channel::Ambient,
        };

        for (channel, mut text) in &mut readouts {
            if *channel == changed {
                text.0 = readout_text(VOLUME.volume_to_perceptual(bus.volume));
            }
        }
    }
}

fn readout_text(gain: f32) -> String {
    format!("{:.0} %", gain * 100.0)
}
