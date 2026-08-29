//! The sonar panel

use bevy::{
    prelude::*,
    ui_widgets::{SliderPrecision, SliderStep, SliderValue, ValueChange},
};

use crate::{
    game::{
        sonar::{
            catalog::{CatalogEntry, SonarCatalog},
            contacts::{SonarContact, SonarContactRecord},
            feature::{SonarFeaturesStatus, SonarSignature},
            range::{RangePreview, ScanRange},
            scan::{ScanCompleted, ScanStarted, SonarCooldown},
        },
        ui::widget::{
            ColorPicked, ColorPicker, Corner, HudButton, HudPanel, HudSlider, ROW_HEIGHT,
            SWATCH_SIZE, SelectedColor, TextField, badge, fill_list, grid, hud_corner, label,
            label_dim, text_field, vrule,
        },
    },
    screens::Screen,
    theme::palette::{self, *},
};

/// Colors a sonar return can be calibrated to.
const POINT_COLOR_CHOICES: [Color; 6] = [
    PHOSPHORE_HIGH,
    AZURE_HIGH,
    AMBER_HIGH,
    RED_HIGH,
    VIOLET_HIGH,
    BONE_HIGH,
];

/// Width of the editable name column.
const NAME_WIDTH: f32 = 96.0;
/// Width of the leading marker column, sized for the NEW badge.
const MARKER_WIDTH: f32 = 32.0;
/// Track width of the footer's range slider.
const RANGE_SLIDER_WIDTH: f32 = 140.0;
/// Width of the range readout, so the slider next to it doesn't shift as the
/// number gains and loses digits.
const RANGE_READOUT_WIDTH: f32 = 44.0;
/// Stands in for the table while a ping is still out.
const WAITING_LABEL: &str = "WAITING FOR SCAN...";
/// Meters the range slider moves in.
const RANGE_STEP: f32 = 10.0;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Gameplay), spawn_sonar_hud);
    app.add_observer(apply_picked_color);
    app.add_systems(
        Update,
        (
            apply_view,
            clean_contacts,
            sync_range_readout,
            rebuild_table,
            rebuild_contacts,
            sync_names,
            sync_contact_names,
            sync_selected_colors,
            sync_sonar_cooldown,
        )
            .chain()
            .run_if(in_state(Screen::Gameplay)),
    );
}

/// The scrolling body of the table. Its contents are rebuilt wholesale whenever
/// the catalog gains a row or a feature unlocks.
#[derive(Component, Default, Clone)]
struct CatalogList;

/// Everything currently under [`CatalogList`], as one despawnable subtree.
#[derive(Component, Default, Clone)]
struct CatalogRows;

/// The body listing what the sonar last heard back from.
#[derive(Component, Default, Clone)]
struct ContactList;

/// Everything currently under [`ContactList`], as one despawnable subtree.
#[derive(Component, Default, Clone)]
struct ContactRows;

/// Ties a widget to the catalog row it edits or displays.
#[derive(Component, Default, Clone)]
struct RowOf(SonarSignature);

/// The footer's live readout of the scan range.
#[derive(Component, Default, Clone)]
struct RangeReadout;

/// Sonar cooldown infos
#[derive(Component, Default, Clone)]
struct SonarCooldownInfo;

/// Which body the panel is showing. Lives on the footer button that flips it.
#[derive(Component, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Clone)]
enum SonarView {
    #[default]
    Contacts,
    Settings,
}

impl SonarView {
    fn flipped(self) -> Self {
        match self {
            Self::Contacts => Self::Settings,
            Self::Settings => Self::Contacts,
        }
    }

    /// The button always names the body it would take you to, not the one
    /// you're on.
    fn button_label(self) -> &'static str {
        match self {
            Self::Contacts => "[ SETTINGS ]",
            Self::Settings => "[ CONTACTS ]",
        }
    }
}

fn spawn_sonar_hud(mut commands: Commands, range: Res<ScanRange>) {
    commands.spawn_scene(bsn! {
        template_value(DespawnOnExit(Screen::Gameplay))
        {hud_corner(Corner::BottomLeft, bsn_list![
            (
                @HudPanel {
                    @title: "SONAR :: RETURNS",
                    @infos: {Box::new(bsn_list![(
                        label_dim("")
                        template_value(TextColor(palette::AMBER_HIGH))
                        SonarCooldownInfo
                    )]) as Box<dyn SceneList>},
                    @body: {Box::new(bsn_list![
                        ({fill_list(bsn_list!())} ContactList),
                        ({fill_list(bsn_list!())} CatalogList Node { display: Display::None }),
                    ]) as Box<dyn SceneList>},
                    @footer: {Box::new(bsn_list![
                        (
                            Node { align_items: AlignItems::Center, column_gap: px(8.0) }
                            Children [
                                (label_dim("RANGE")),
                                (
                                    @HudSlider {
                                        @value: {range.0},
                                        @min: {ScanRange::MIN},
                                        @max: {ScanRange::MAX},
                                        @width: {RANGE_SLIDER_WIDTH},
                                        @on_change: {bsn!(on(apply_range))},
                                    }
                                    SliderStep(RANGE_STEP)
                                    SliderPrecision(0)
                                ),
                                (
                                    Node { width: {px(RANGE_READOUT_WIDTH)}, justify_content: JustifyContent::End }
                                    Children [ ({label(format!("{:.0} m", range.0))} RangeReadout) ]
                                ),
                                (
                                    @HudButton {
                                        @label: {SonarView::default().button_label().to_string()},
                                        @on_click: {bsn!(on(toggle_view))},
                                    }
                                    SonarView
                                    Node { margin: {UiRect::left(auto())} }
                                ),
                            ]
                        ),
                    ]) as Box<dyn SceneList>},
                }

            )
        ])}
    });
}

/// Rebuilds the whole table when its *shape* changes: a row discovered, a row
/// superseded, or a feature unlocked.
///
/// Renaming and recoloring go through `bypass_change_detection()` precisely so
/// they don't land here: a rebuild despawns the rows, and doing that mid-rename
/// would destroy the entity holding keyboard focus.
fn rebuild_table(
    mut commands: Commands,
    catalog: Res<SonarCatalog>,
    feat_status: Res<SonarFeaturesStatus>,
    rows: Query<Entity, With<CatalogRows>>,
    list: Single<(Entity, Ref<CatalogList>)>,
) {
    let (list, marker) = list.into_inner();
    // `is_added` covers the very first build, when the panel has just spawned
    // and neither resource has changed since.
    if !marker.is_added() && !catalog.is_changed() && !feat_status.is_changed() {
        return;
    }

    for rows in &rows {
        commands.entity(rows).despawn();
    }

    let feature_names = feat_status.unlocked_names();
    let column_count = column_count(feature_names.len());

    let mut cells: Vec<Box<dyn Scene>> = Vec::new();
    push_header(&mut cells, &feature_names);
    for (signature, entry) in catalog.rows() {
        push_row(&mut cells, &feat_status, signature, &entry);
    }

    commands
        .entity(list)
        .queue_spawn_related_scenes::<Children>(bsn_list![(
            {grid(column_count, cells)}
            CatalogRows
        )]);
}

/// `marker | name | rule | one per unlocked feature | rule | colors`
fn column_count(feature_count: usize) -> u16 {
    5 + feature_count as u16
}

/// Every row, header included, pushes exactly [`column_count`] cells in the
/// same order, which is what lets the grid place them without any per-row node.
fn push_header(cells: &mut Vec<Box<dyn Scene>>, feature_names: &[&'static str]) {
    cells.push(Box::new(bsn! { Node { width: {px(MARKER_WIDTH)} } }));
    cells.push(Box::new(label_dim("NAME")));
    cells.push(Box::new(vrule()));
    for name in feature_names {
        cells.push(Box::new(label_dim(*name)));
    }
    cells.push(Box::new(vrule()));
    cells.push(Box::new(label_dim("RETURN")));
}

fn push_row(
    cells: &mut Vec<Box<dyn Scene>>,
    feat_status: &SonarFeaturesStatus,
    signature: SonarSignature,
    entry: &CatalogEntry,
) {
    let marker: Vec<_> = entry.is_new.then(|| badge("NEW")).into_iter().collect();

    cells.push(Box::new(bsn! {
        Node {
            width: {px(MARKER_WIDTH)},
            height: {px(ROW_HEIGHT)},
            align_items: AlignItems::Center,
        }
        Children [ {marker} ]
    }));
    cells.push(Box::new(bsn! {
        {text_field(entry.name.clone(), NAME_WIDTH)}
        template_value(RowOf(signature))
    }));
    cells.push(Box::new(vrule()));
    for value in feat_status.features_of(&signature) {
        // A row found before this feature unlocked has no reading for it yet.
        cells.push(Box::new(label(value.unwrap_or("--"))));
    }
    cells.push(Box::new(vrule()));
    cells.push(Box::new(bsn! {
        @ColorPicker { @colors: {POINT_COLOR_CHOICES.to_vec()} }
        template_value(RowOf(signature))
        template_value(SelectedColor(entry.color))
    }));
}

/// Writes a picked color back into the catalog.
fn apply_picked_color(
    picked: On<ColorPicked>,
    pickers: Query<&RowOf>,
    mut catalog: ResMut<SonarCatalog>,
) {
    let Ok(row) = pickers.get(picked.entity) else {
        return;
    };

    // Recoloring doesn't change the table's shape; see `rebuild_table`.
    catalog
        .bypass_change_detection()
        .set_color(&row.0, picked.color);
}

/// Takes the range the player is dialling in, and puts the preview ring up
/// around the drone for as long as they keep moving.
///
/// The slider is headless and doesn't own its value, so writing [`SliderValue`]
/// back is part of the job.
fn apply_range(
    change: On<ValueChange<f32>>,
    mut commands: Commands,
    mut range: ResMut<ScanRange>,
    mut preview: ResMut<RangePreview>,
) {
    commands
        .entity(change.source)
        .insert(SliderValue(change.value));

    range.0 = change.value;
    preview.show();
}

/// Keeps the footer's readout on the current range, whoever set it.
fn sync_range_readout(range: Res<ScanRange>, mut readouts: Query<&mut Text, With<RangeReadout>>) {
    if !range.is_changed() {
        return;
    }

    for mut readout in &mut readouts {
        readout.0 = format!("{:.0} m", range.0);
    }
}

/// Flips the footer button's view; [`apply_view`] does the rest.
fn toggle_view(click: On<Pointer<Click>>, mut buttons: Query<&mut SonarView>) {
    let Ok(mut view) = buttons.get_mut(click.event_target()) else {
        return;
    };

    *view = view.flipped();
}

/// Shows the body the button asks for, hides the other, and relabels the button
/// to match. Also runs on the frame the panel spawns, which is what puts the
/// two bodies in a known state to begin with.
///
/// Switching back to the contacts drops the NEW badges: they stay up for the
/// whole time the table is on screen, not just the frame it opens.
fn apply_view(
    buttons: Query<(&SonarView, &Children), Changed<SonarView>>,
    // One query over both bodies: two `Single`s filtered on different markers
    // would look like conflicting access to `Node`.
    mut bodies: Query<(&mut Node, Has<CatalogList>), Or<(With<ContactList>, With<CatalogList>)>>,
    mut labels: Query<&mut Text>,
    mut catalog: ResMut<SonarCatalog>,
) {
    for (view, children) in &buttons {
        let settings = *view == SonarView::Settings;

        for (mut body, is_catalog) in &mut bodies {
            body.display = match is_catalog == settings {
                true => Display::Flex,
                false => Display::None,
            };
        }

        // A `HudButton`'s only child is its label.
        if let Some(&label) = children.first()
            && let Ok(mut label) = labels.get_mut(label)
        {
            label.0 = view.button_label().to_string();
        }

        // Guarded so leaving an already-seen table doesn't mark the catalog
        // changed and trigger a pointless rebuild.
        if !settings && catalog.new_count() > 0 {
            catalog.clear_new_flags();
        }
    }
}

/// Clears the reading as a ping goes out: what the last one heard is stale from
/// the moment the sonar pings again, and the table only fills back in once the
/// new ping is done listening.
fn clean_contacts(
    mut commands: Commands,
    mut started: MessageReader<ScanStarted>,
    rows: Query<Entity, With<ContactRows>>,
    list: Single<Entity, With<ContactList>>,
) {
    if started.is_empty() {
        return;
    }
    started.clear();

    for rows in &rows {
        commands.entity(rows).despawn();
    }

    // Tagged as the list's body like the table is, so the next rebuild clears
    // it the same way.
    commands
        .entity(*list)
        .queue_spawn_related_scenes::<Children>(bsn_list![(
            {label_dim(WAITING_LABEL)}
            ContactRows
        )]);
}

/// Rebuilds the contact list whenever the sonar hears something new
fn rebuild_contacts(
    mut commands: Commands,
    mut completed: MessageReader<ScanCompleted>,
    record: Res<SonarContactRecord>,
    catalog: Res<SonarCatalog>,
    feat_status: Res<SonarFeaturesStatus>,
    rows: Query<Entity, With<ContactRows>>,
    list: Single<(Entity, Ref<ContactList>)>,
) {
    let (list, marker) = list.into_inner();
    // The reading lands in one piece once the ping is done listening; the wipe
    // on its way out is `clean_contacts`'s job.
    let scan_complete = !completed.is_empty();
    completed.clear();

    if !marker.is_added() && !scan_complete && !feat_status.is_changed() {
        return;
    }

    for rows in &rows {
        commands.entity(rows).despawn();
    }

    let feature_names = feat_status.unlocked_names();
    let column_count = contact_column_count(feature_names.len());

    let mut cells: Vec<Box<dyn Scene>> = Vec::new();
    push_contact_header(&mut cells, &feature_names);
    for contact in sorted_contacts(&record, &catalog) {
        push_contact_row(&mut cells, contact, &catalog, &feat_status);
    }

    commands
        .entity(list)
        .queue_spawn_related_scenes::<Children>(bsn_list![(
            {grid(column_count, cells)}
            ContactRows
        )]);
}

/// The reading in the order the player wants to read it: anything the sonar has
/// never resolved before on top, the rest by how solid the return was.
fn sorted_contacts<'a>(
    record: &'a SonarContactRecord,
    catalog: &SonarCatalog,
) -> Vec<&'a SonarContact> {
    let mut contacts = record.contacts();
    contacts.sort_by_key(|contact| !is_new(catalog, &contact.signature));
    contacts
}

fn is_new(catalog: &SonarCatalog, signature: &SonarSignature) -> bool {
    catalog.get(signature).is_some_and(|entry| entry.is_new)
}

/// `marker | return | name | rule | one per unlocked feature | rule | echoes | distance`
fn contact_column_count(feature_count: usize) -> u16 {
    7 + feature_count as u16
}

/// Every row, header included, pushes exactly [`contact_column_count`] cells in
/// the same order, which is what lets the grid place them without any per-row
/// node.
fn push_contact_header(cells: &mut Vec<Box<dyn Scene>>, feature_names: &[&'static str]) {
    cells.push(Box::new(bsn! { Node { width: {px(MARKER_WIDTH)} } }));
    cells.push(Box::new(bsn! { Node { width: {px(SWATCH_SIZE)} } }));
    cells.push(Box::new(label_dim("NAME")));
    cells.push(Box::new(vrule()));
    for name in feature_names {
        cells.push(Box::new(label_dim(*name)));
    }
    cells.push(Box::new(vrule()));
    cells.push(Box::new(number_cell(label_dim("ECHOES"))));
    cells.push(Box::new(number_cell(label_dim("DIST"))));
}

fn push_contact_row(
    cells: &mut Vec<Box<dyn Scene>>,
    contact: &SonarContact,
    catalog: &SonarCatalog,
    feat_status: &SonarFeaturesStatus,
) {
    let signature = contact.signature;
    let marker: Vec<_> = is_new(catalog, &signature)
        .then(|| badge("NEW"))
        .into_iter()
        .collect();
    let color = catalog.color_of(&signature).unwrap_or_default();
    let name = catalog.name_of(&signature).unwrap_or_default();
    let (counts, distance) = (contact.counts, contact.distance);

    cells.push(Box::new(bsn! {
        Node {
            width: {px(MARKER_WIDTH)},
            height: {px(ROW_HEIGHT)},
            align_items: AlignItems::Center,
        }
        Children [ {marker} ]
    }));
    // The color the return is painted in on screen, so a row can be matched to
    // the dots it came from.
    cells.push(Box::new(bsn! {
        Node {
            width: {px(SWATCH_SIZE)},
            height: {px(SWATCH_SIZE)},
            border_radius: {BorderRadius::all(px(2))},
        }
        template_value(BackgroundColor(color))
    }));
    cells.push(Box::new(bsn! {
        {label(name)}
        template_value(RowOf(signature))
        TextLayout::no_wrap()
        Node { width: {px(NAME_WIDTH)}, overflow: {Overflow::clip()} }
    }));
    cells.push(Box::new(vrule()));
    for value in feat_status.features_of(&signature) {
        // A contact resolved before this feature unlocked has no reading for it.
        cells.push(Box::new(label(value.unwrap_or("--"))));
    }
    cells.push(Box::new(vrule()));
    cells.push(Box::new(number_cell(label(format!("{counts}")))));
    cells.push(Box::new(number_cell(label(format!("{distance:.0} m")))));
}

/// A right-aligned cell, so a column of readings lines up on its units rather
/// than on its first digit.
fn number_cell(content: impl Scene) -> impl Scene {
    bsn! {
        Node { justify_self: JustifySelf::End }
        Children [ ({content}) ]
    }
}

/// Pushes edited names back into the catalog.
fn sync_names(
    mut catalog: ResMut<SonarCatalog>,
    fields: Query<(&RowOf, &TextField)>,
    labels: Query<&Text>,
) {
    for (row, field) in &fields {
        let Ok(label) = labels.get(field.label) else {
            continue;
        };

        if catalog
            .get(&row.0)
            .is_some_and(|entry| entry.name != label.0)
        {
            // Renaming doesn't change the table's shape; see `rebuild_table`.
            catalog
                .bypass_change_detection()
                .set_name(&row.0, label.0.clone());
        }
    }
}

/// Keeps the contact list showing whatever the rows are currently *called*.
fn sync_contact_names(catalog: Res<SonarCatalog>, mut names: Query<(&RowOf, &mut Text)>) {
    for (row, mut name) in &mut names {
        let Some(entry) = catalog.get(&row.0) else {
            continue;
        };

        if name.0 != entry.name {
            name.0 = entry.name.clone();
        }
    }
}

/// Keeps each picker's highlighted swatch in step with the catalog
fn sync_selected_colors(
    catalog: Res<SonarCatalog>,
    mut pickers: Query<(&RowOf, &mut SelectedColor)>,
) {
    for (row, mut selected) in &mut pickers {
        if let Some(entry) = catalog.get(&row.0) {
            selected.0 = entry.color;
        }
    }
}

/// Sync sonar cooldown info
fn sync_sonar_cooldown(
    sonar: Single<&SonarCooldown>,
    texts: Query<&mut Text, With<SonarCooldownInfo>>,
) {
    for mut text in texts {
        if sonar.timer.is_finished() {
            *text = Text::new("> SONAR READY <");
        } else {
            *text = Text::new(format!(
                "SONAR READY IN {:.1} SECONDS",
                sonar.timer.remaining_secs()
            ));
        }
    }
}
