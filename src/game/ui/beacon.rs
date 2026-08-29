//! The beacon link panel: connecting to a nearby beacon and downloading data

use bevy::prelude::*;

use crate::{
    game::{
        beacon::{Beacon, BeaconConnection, BeaconData, ConnectionToggleRequested},
        ui::widget::{Corner, HudButton, HudPanel, hud_corner, label, label_dim, row},
    },
    screens::Screen,
    theme::palette::*,
};

/// Panel size.
const PANEL_SIZE: Vec2 = Vec2::new(380., 170.);
/// Height of the download bar.
const BAR_HEIGHT: f32 = 8.0;
/// Stands in for a readout the link has nothing to put in it yet.
const EMPTY_READOUT: &str = "--";
/// Fixed width of the connect button, so it doesn't jump around as its label
/// goes from CONNECT to SEARCHING... to DISCONNECT.
const CONNECTION_BUTTON_WIDTH: f32 = 150.0;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Gameplay), spawn_beacon_hud);
    app.add_systems(
        Update,
        sync_connection_panel.run_if(in_state(Screen::Gameplay)),
    );
}

/// The panel the toggle button opens and closes.
#[derive(Component, Default, Clone)]
struct BeaconPanel;

/// Tagged with what it reads out.
#[derive(Component, Clone, Copy, PartialEq, Eq, Default)]
enum Readout {
    #[default]
    BeaconId,
    Downloaded,
    Status,
}

/// The download bar's frame, up only while the payload is coming down.
#[derive(Component, Default, Clone)]
struct DownloadBarFrame;

/// The filled part of the download bar.
#[derive(Component, Default, Clone)]
struct DownloadBar;

/// The button that opens and drops the connection.
#[derive(Component, Default, Clone)]
struct ConnectionButton;

/// The button that starts the download.
#[derive(Component, Default, Clone)]
struct DownloadButton;

fn spawn_beacon_hud(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        template_value(DespawnOnExit(Screen::Gameplay))
        {hud_corner(Corner::BottomRight, bsn_list![
            (
                @HudPanel {
                    @title: "BEACON :: CONNECTION",
                    @size: {PANEL_SIZE},
                    @infos: {Box::new(bsn_list![(
                        label_dim("")
                        template_value(Readout::Status)
                        template_value(TextColor(AMBER_HIGH))
                    )]) as Box<dyn SceneList>},
                    @body: {Box::new(bsn_list![
                        ({row(6.0, bsn_list![
                            (label_dim("BEACON ID: ")),
                            ({label(EMPTY_READOUT)} template_value(Readout::BeaconId)),
                        ])}),
                        ({row(6.0, bsn_list![
                            (label_dim("DATA DOWNLOADED: ")),
                            ({label(EMPTY_READOUT)} template_value(Readout::Downloaded)),
                        ])}),
                        (
                            Node {
                                display: Display::None,
                                width: percent(100),
                                height: {px(BAR_HEIGHT)},
                                border: {UiRect::all(px(1))},
                                border_radius: {BorderRadius::all(px(2))},
                                margin: {UiRect::top(px(2))},
                            }
                            BorderColor::all(HUD_BORDER)
                            DownloadBarFrame
                            Children [
                                (
                                    Node { width: percent(0), height: percent(100) }
                                    BackgroundColor(HUD_ACCENT)
                                    DownloadBar
                                ),
                            ]
                        ),
                    ]) as Box<dyn SceneList>},
                    @footer: {Box::new(bsn_list![
                        (
                            Node { column_gap: px(8.0) }
                            Children [
                                (
                                    @HudButton {
                                        @label: "[ DOWNLOAD ]",
                                        @on_click: {bsn!(on(start_download))},
                                    }
                                    DownloadButton
                                    Node { display: Display::None, margin: {UiRect::right(auto())} }
                                ),
                                (
                                    @HudButton {
                                        @label: "[ CONNECT ]",
                                        @on_click: {bsn!(on(toggle_connection))},
                                    }
                                    ConnectionButton
                                    Node { margin: {UiRect::left(auto())}, width: {px(CONNECTION_BUTTON_WIDTH)} }
                                ),
                            ]
                        ),
                    ]) as Box<dyn SceneList>},
                }
                BeaconPanel
                Node { display: Display::None }
            ),
            (
                @HudButton {
                    @label: "[ BEACON ]",
                    @on_click: {bsn!(on(toggle_panel))},
                }
            ),
        ])}
    });
}

fn toggle_panel(_: On<Pointer<Click>>, mut panel: Single<&mut Node, With<BeaconPanel>>) {
    panel.display = match panel.display {
        Display::None => Display::Flex,
        _ => Display::None,
    };
}

fn toggle_connection(
    _: On<Pointer<Click>>,
    mut requests: MessageWriter<ConnectionToggleRequested>,
) {
    requests.write(ConnectionToggleRequested);
}

fn start_download(_: On<Pointer<Click>>, mut connection: ResMut<BeaconConnection>) {
    connection.start_download();
}

fn sync_connection_panel(
    connection: Res<BeaconConnection>,
    beacons: Query<(&Beacon, &BeaconData)>,
    mut readouts: Query<(&Readout, &mut Text)>,
    mut labels: Query<(&mut Text, &mut TextColor), Without<Readout>>,
    mut frame: Single<
        &mut Node,
        (
            With<DownloadBarFrame>,
            Without<DownloadBar>,
            Without<DownloadButton>,
        ),
    >,
    mut bar: Single<&mut Node, (With<DownloadBar>, Without<DownloadButton>)>,
    download_button: Single<&mut Node, With<DownloadButton>>,
    connection_button: Single<&Children, With<ConnectionButton>>,
) {
    let connected = connection
        .connected()
        .and_then(|beacon| beacons.get(beacon).ok());
    let progress = connected
        .map(|(_, data)| data.progress())
        .unwrap_or_default();

    for (readout, mut text) in &mut readouts {
        text.0 = match (readout, connected) {
            (Readout::Status, _) => connection.status().to_string(),
            (Readout::BeaconId, Some((beacon, _))) => beacon.id(),
            (Readout::Downloaded, Some(_)) => format!("{:.0} %", progress.floor() * 100.0),
            _ => EMPTY_READOUT.to_string(),
        };
    }

    // The bar is only displayed while downloading
    frame.display = display_if(connection.is_downloading());
    bar.width = percent(progress * 100.0);

    let mut download_button = download_button.into_inner();
    let downloadable =
        !connection.is_downloading() && connected.is_some_and(|(_, data)| !data.is_complete());
    download_button.display = display_if(downloadable);

    if let Some(&label) = connection_button.first()
        && let Ok((mut text, mut color)) = labels.get_mut(label)
    {
        text.0 = connection_button_label(&connection).to_string();
        color.0 = match connection.is_searching() {
            true => HUD_TEXT_DIM,
            false => HUD_TEXT,
        };
    }
}

fn connection_button_label(connection: &BeaconConnection) -> &'static str {
    if connection.connected().is_some() {
        "[ DISCONNECT ]"
    } else if connection.is_handshaking() {
        "[ CONNECTING... ]"
    } else if connection.is_searching() {
        "[ SEARCHING... ]"
    } else {
        "[ CONNECT ]"
    }
}

fn display_if(shown: bool) -> Display {
    match shown {
        true => Display::Flex,
        false => Display::None,
    }
}
