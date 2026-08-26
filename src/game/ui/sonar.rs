//! The sonar calibration table: one row per surface the sonar has returned,
//! discovered by playing rather than listed up-front.
//!
//! A row is `[NEW] name │ feature │ feature ║ colors`, with one column per
//! *unlocked* feature — a locked feature has no column at all, so the table
//! never hints at how many are left to find.

use bevy::prelude::*;

use crate::{
    game::{
        sonar::{
            catalog::{CatalogEntry, SonarCatalog},
            feature::{SonarFeaturesStatus, SonarSignature},
        },
        ui::widget::{
            ColorPicked, ColorPicker, Corner, HudDrawer, HudDrawerToggled, ROW_HEIGHT,
            SelectedColor, TextField, badge, hud_corner, label, label_dim, rule, scroll_list,
            text_field,
        },
    },
    screens::Screen,
    theme::palette::*,
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

/// Rows visible before the list starts scrolling.
const MAX_VISIBLE_ROWS: usize = 10;
/// Width of the editable name column.
const NAME_WIDTH: f32 = 96.0;
/// Width of the leading marker column, sized for the NEW badge.
const MARKER_WIDTH: f32 = 32.0;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Gameplay), spawn_sonar_hud);
    app.add_observer(apply_picked_color);
    app.add_observer(clear_new_flags_on_close);
    app.add_systems(
        Update,
        (rebuild_table, sync_names, sync_selected_colors)
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

/// Ties a widget to the catalog row it edits.
#[derive(Component, Default, Clone)]
struct RowOf(SonarSignature);

fn spawn_sonar_hud(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        template_value(DespawnOnExit(Screen::Gameplay))
        {hud_corner(Corner::BottomRight, bsn_list![
            @HudDrawer {
                @title: "SONAR :: RETURN CALIBRATION",
                @label: "[ CAL ]",
                @body: {Box::new(bsn_list![
                    ({scroll_list(MAX_VISIBLE_ROWS, bsn_list!())} CatalogList),
                ]) as Box<dyn SceneList>},
            }
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
            CatalogRows
            // One grid rather than a stack of rows: `auto` tracks size each
            // column to its widest cell and keep every row aligned, so a long
            // feature name widens its column instead of overlapping the next.
            Node {
                display: Display::Grid,
                grid_template_columns: {RepeatedGridTrack::auto::<Vec<_>>(column_count)},
                column_gap: px(6.0),
                row_gap: px(2.0),
                align_items: AlignItems::Center,
            }
            Children [ {cells} ]
        )]);
}

/// `marker | name | rule | one per unlocked feature | rule | colors`
fn column_count(feature_count: usize) -> u16 {
    4 + feature_count as u16 + 1
}

/// Every row, header included, pushes exactly [`column_count`] cells in the
/// same order, which is what lets the grid place them without any per-row node.
fn push_header(cells: &mut Vec<Box<dyn Scene>>, feature_names: &[&'static str]) {
    cells.push(Box::new(bsn! { Node { width: {px(MARKER_WIDTH)} } }));
    cells.push(Box::new(label_dim("NAME")));
    cells.push(Box::new(rule()));
    for name in feature_names {
        cells.push(Box::new(label_dim(*name)));
    }
    cells.push(Box::new(rule()));
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
    cells.push(Box::new(rule()));
    for value in feat_status.features_of(&signature) {
        // A row found before this feature unlocked has no reading for it yet.
        cells.push(Box::new(label(value.unwrap_or("--"))));
    }
    cells.push(Box::new(rule()));
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

/// Drops the NEW badges once the player closes the panel again: they stay up
/// for the whole time the table is on screen, not just the frame it opens.
fn clear_new_flags_on_close(toggled: On<HudDrawerToggled>, mut catalog: ResMut<SonarCatalog>) {
    // Guarded so closing an already-seen panel doesn't mark the catalog changed
    // and trigger a pointless rebuild.
    if !toggled.open && catalog.new_count() > 0 {
        catalog.clear_new_flags();
    }
}

/// Pushes edited names back into the catalog.
///
/// The field's label always carries the committed value, edited or not, so this
/// doesn't care whether the row is currently unlocked.
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

/// Keeps each picker's highlighted swatch in step with the catalog, whoever
/// changed it.
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
