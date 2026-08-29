//! The notes archive

use bevy::prelude::*;

use crate::{
    game::{
        note::{Note, NoteCatalog, NoteStatus},
        ui::widget::{
            HudButton, HudPanel, Side, badge, fill_list, hrule, hud_center, hud_rail, label,
            label_dim, vrule,
        },
    },
    screens::Screen,
    theme::{interaction::InteractionPalette, palette::*},
};

/// Window size.
const WINDOW_SIZE: Vec2 = Vec2::new(800.0, 520.0);
/// Width of the title column.
const LIST_WIDTH: f32 = 200.0;
/// Stands in for the list when the drone hasn't recovered anything yet.
const EMPTY_LIST_LABEL: &str = "NO DATA RECOVERED";
/// Stands in for the reading pane while no note is picked.
const NO_SELECTION_LABEL: &str = "SELECT A NOTE";
/// Shown as a title while a note's file is still on its way in.
const LOADING_TITLE: &str = "...";

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<NotesView>();
    app.add_systems(OnEnter(Screen::Gameplay), spawn_notes_ui);
    app.add_systems(Update, rebuild_notes.run_if(in_state(Screen::Gameplay)));
}

/// What the archive is showing. Everything in the window is rebuilt from this.
#[derive(Resource, Default)]
struct NotesView {
    open: bool,
    /// Index into [`NoteCatalog::notes`].
    selected: Option<usize>,
}

/// The window the rail button opens and closes.
#[derive(Component, Default, Clone)]
struct NotesWindow;

/// The column of titles.
#[derive(Component, Default, Clone)]
struct NoteList;

/// Everything currently under [`NoteList`], as one despawnable subtree.
#[derive(Component, Default, Clone)]
struct NoteRows;

/// The pane the picked note is read in.
#[derive(Component, Default, Clone)]
struct NoteReader;

/// Everything currently under [`NoteReader`], as one despawnable subtree.
#[derive(Component, Default, Clone)]
struct ReaderContent;

/// Ties a row to the note it opens.
#[derive(Component, Default, Clone)]
struct NoteRow(usize);

fn spawn_notes_ui(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        template_value(DespawnOnExit(Screen::Gameplay))
        {hud_rail(Side::Right, bsn_list![
            (
                @HudButton {
                    @label: "[ NOTES ]",
                    @on_click: {bsn!(on(toggle_notes))},
                }
            ),
        ])}
    });

    commands.spawn_scene(bsn! {
        template_value(DespawnOnExit(Screen::Gameplay))
        {hud_center(bsn_list![
            (
                @HudPanel {
                    @title: "ARCHIVE :: NOTES",
                    @size: {WINDOW_SIZE},
                    @body: {Box::new(bsn_list![
                        (
                            Node {
                                flex_grow: 1.0,
                                column_gap: px(12.0),
                                min_height: px(0.0),
                            }
                            Children [
                                ({fill_list(bsn_list!())} NoteList Node { width: {px(LIST_WIDTH)}, flex_grow: 0.0 }),
                                ({vrule()} Node { height: percent(100) }),
                                ({fill_list(bsn_list!())} NoteReader Node { row_gap: px(6.0), min_width: px(0.0) }),
                            ]
                        ),
                    ]) as Box<dyn SceneList>},
                    @footer: {Box::new(bsn_list![
                        (
                            Node { justify_content: JustifyContent::End }
                            Children [
                                (
                                    @HudButton {
                                        @label: "[ CLOSE ]",
                                        @on_click: {bsn!(on(toggle_notes))},
                                    }
                                ),
                            ]
                        ),
                    ]) as Box<dyn SceneList>},
                }
                NotesWindow
                Node { display: Display::None }
            ),
        ])}
    });
}

fn toggle_notes(_: On<Pointer<Click>>, mut view: ResMut<NotesView>) {
    view.open = !view.open;
}

fn select_note(
    click: On<Pointer<Click>>,
    rows: Query<&NoteRow>,
    mut view: ResMut<NotesView>,
    mut catalog: ResMut<NoteCatalog>,
) {
    let Ok(row) = rows.get(click.event_target()) else {
        return;
    };

    view.selected = Some(row.0);
    catalog.mark_read(row.0);
}

fn rebuild_notes(
    mut commands: Commands,
    view: Res<NotesView>,
    catalog: Res<NoteCatalog>,
    notes: Res<Assets<Note>>,
    mut loaded: MessageReader<AssetEvent<Note>>,
    window: Single<(&mut Node, Ref<NotesWindow>)>,
    list: Single<Entity, With<NoteList>>,
    reader: Single<Entity, With<NoteReader>>,
    contents: Query<Entity, Or<(With<NoteRows>, With<ReaderContent>)>>,
) {
    let assets_changed = !loaded.is_empty();
    loaded.clear();

    let (mut window, marker) = window.into_inner();
    if !marker.is_added() && !view.is_changed() && !catalog.is_changed() && !assets_changed {
        return;
    }

    window.display = match view.open {
        true => Display::Flex,
        false => Display::None,
    };

    for content in &contents {
        commands.entity(content).despawn();
    }

    commands
        .entity(*list)
        .queue_spawn_related_scenes::<Children>(bsn_list![(
            Node { flex_direction: FlexDirection::Column, row_gap: px(2.0) }
            NoteRows
            Children [ {note_rows(&view, &catalog, &notes)} ]
        )]);

    commands
        .entity(*reader)
        .queue_spawn_related_scenes::<Children>(bsn_list![(
            Node { flex_direction: FlexDirection::Column, row_gap: px(6.0) }
            ReaderContent
            Children [ {reader_content(&view, &catalog, &notes)} ]
        )]);
}

fn note_rows(view: &NotesView, catalog: &NoteCatalog, notes: &Assets<Note>) -> Vec<Box<dyn Scene>> {
    let mut rows: Vec<Box<dyn Scene>> = Vec::new();

    for (index, item) in catalog.notes().iter().enumerate() {
        let NoteStatus::Unlocked { unread } = item.status else {
            continue;
        };

        let title = match notes.get(&item.note) {
            Some(note) => note.metadata.title.clone(),
            None => LOADING_TITLE.to_string(),
        };
        rows.push(Box::new(note_row(
            index,
            title,
            unread,
            view.selected == Some(index),
        )));
    }

    if rows.is_empty() {
        rows.push(Box::new(label_dim(EMPTY_LIST_LABEL)));
    }
    rows
}

fn note_row(index: usize, title: String, unread: bool, selected: bool) -> impl Scene {
    let background = match selected {
        true => HUD_BACKGROUND_HOVERED,
        false => HUD_BACKGROUND,
    };
    let border = match selected {
        true => HUD_ACCENT,
        false => Color::NONE,
    };
    let mark: Vec<_> = unread.then(|| badge("NEW")).into_iter().collect();

    bsn! {
        Button
        Node {
            padding: {UiRect::axes(px(6), px(4))},
            column_gap: px(6.0),
            align_items: AlignItems::Center,
            border: {UiRect::left(px(2))},
            border_radius: {BorderRadius::all(px(2))},
        }
        template_value(BackgroundColor(background))
        BorderColor::all(border)
        template_value(InteractionPalette {
            none: background,
            hovered: HUD_BACKGROUND_HOVERED,
            pressed: HUD_BACKGROUND_HOVERED,
        })
        template_value(NoteRow(index))
        on(select_note)
        Children [
            // Don't bubble picking events from the row's contents up to the row.
            (
                {label(title)}
                Pickable::IGNORE
                TextLayout::no_wrap()
                Node { flex_grow: 1.0, min_width: px(0.0), overflow: {Overflow::clip()} }
            ),
            {mark},
        ]
    }
}

fn reader_content(
    view: &NotesView,
    catalog: &NoteCatalog,
    notes: &Assets<Note>,
) -> Vec<Box<dyn Scene>> {
    let selected = view
        .selected
        .and_then(|index| catalog.notes().get(index))
        .and_then(|item| notes.get(&item.note));

    let Some(note) = selected else {
        return vec![Box::new(label_dim(NO_SELECTION_LABEL))];
    };

    vec![
        Box::new(bsn! {
            {label(note.metadata.title.clone())}
            template_value(TextColor(HEADER_TEXT))
        }),
        Box::new(label_dim(note.metadata.date.clone())),
        Box::new(bsn! { {hrule()} Node { margin: {UiRect::vertical(px(4))} } }),
        Box::new(label(note.text.clone())),
    ]
}
