//! The settings menu.
//!
//! Additional settings and accessibility options should go here.

use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use bevy_seedling::{
    firewheel::Volume, node::label::MainBus, nodes::core::VolumeNode,
    utils::perceptual_volume::PerceptualVolume,
};

use crate::{menus::Menu, screens::Screen, theme::prelude::*};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Menu::Settings), spawn_settings_menu);
    app.add_systems(
        Update,
        go_back.run_if(in_state(Menu::Settings).and_then(input_just_pressed(KeyCode::Escape))),
    );

    app.add_systems(Update, update_master_label.run_if(in_state(Menu::Settings)));
}

fn spawn_settings_menu(mut commands: Commands) {
    commands.spawn((
        widget::ui_root("Settings Menu"),
        GlobalZIndex(2),
        DespawnOnExit(Menu::Settings),
        children![
            widget::header("Settings"),
            settings_grid(),
            widget::button("Back", go_back_on_click),
        ],
    ));
}

fn settings_grid() -> impl Bundle {
    (
        Name::new("Settings Grid"),
        Node {
            display: Display::Grid,
            row_gap: px(10),
            column_gap: px(30),
            grid_template_columns: RepeatedGridTrack::px(2, 400.0),
            ..default()
        },
        children![
            (
                widget::label("Master Volume"),
                Node {
                    justify_self: JustifySelf::End,
                    ..default()
                }
            ),
            global_volume_widget(),
        ],
    )
}

fn global_volume_widget() -> impl Bundle {
    (
        Name::new("Global Volume Widget"),
        Node {
            justify_self: JustifySelf::Start,
            ..default()
        },
        children![
            widget::button_small("-", lower_master),
            (
                Name::new("Current Volume"),
                Node {
                    padding: UiRect::horizontal(px(10)),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                children![(widget::label(""), MasterVolumeLabel)],
            ),
            widget::button_small("+", raise_master),
        ],
    )
}

const CONVERTER: PerceptualVolume = PerceptualVolume::new();

const MIN_VOLUME: f32 = 0.0;
const MAX_VOLUME: f32 = 2.0;
const STEP: f32 = 0.1;

fn increment_volume(volume: Volume) -> Volume {
    let perceptual = CONVERTER.volume_to_perceptual(volume);
    let new_perceptual = (perceptual + STEP).min(MAX_VOLUME);
    CONVERTER.perceptual_to_volume(new_perceptual)
}

fn decrement_volume(volume: Volume) -> Volume {
    let perceptual = CONVERTER.volume_to_perceptual(volume);
    let new_perceptual = (perceptual - STEP).max(MIN_VOLUME);
    CONVERTER.perceptual_to_volume(new_perceptual)
}

fn lower_master(_: On<Pointer<Click>>, mut master: Single<&mut VolumeNode, With<MainBus>>) {
    master.volume = decrement_volume(master.volume);
}

fn raise_master(_: On<Pointer<Click>>, mut master: Single<&mut VolumeNode, With<MainBus>>) {
    master.volume = increment_volume(master.volume);
}

#[derive(Component, Reflect)]
#[reflect(Component)]
struct MasterVolumeLabel;

fn update_master_label(
    master: Single<&VolumeNode, (With<MainBus>, Changed<VolumeNode>)>,
    mut label: Single<&mut Text, With<MasterVolumeLabel>>,
) {
    let percent = CONVERTER.volume_to_perceptual(master.volume) * 100.0;
    label.0 = format!("{percent:3.0}%");
}

fn go_back_on_click(
    _: On<Pointer<Click>>,
    screen: Res<State<Screen>>,
    mut next_menu: ResMut<NextState<Menu>>,
) {
    next_menu.set(if screen.get() == &Screen::Title {
        Menu::Main
    } else {
        Menu::Pause
    });
}

fn go_back(screen: Res<State<Screen>>, mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(if screen.get() == &Screen::Title {
        Menu::Main
    } else {
        Menu::Pause
    });
}
