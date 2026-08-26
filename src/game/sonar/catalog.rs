//! The player's log of everything the sonar has returned so far.
//!
//! One row per distinct [`Signature`]. Rows appear by being scanned, never
//! up-front, so the catalog doubles as a record of what has been found.

use bevy::{platform::collections::HashMap, prelude::*};

use crate::{game::sonar::feature::SonarSignature, theme::palette};

/// One row of the calibration table.
#[derive(Clone, Debug)]
pub struct CatalogEntry {
    /// Player-editable. Starts as an auto-assigned designation.
    pub name: String,
    /// Registered color for the row
    pub color: Color,
    /// Set on discovery, cleared once the player has opened the panel.
    pub is_new: bool,
    /// Discovery order, so the table keeps a stable row order.
    order: u32,
    /// A finer signature replaced this one after a feature unlock. Hidden from
    /// the table, but kept so its siblings can still inherit from it.
    superseded: bool,
}

/// Change detection on this resource means "the table changed *shape*": a row
/// appeared, was superseded, or lost its NEW badge.
///
/// Renaming and recoloring deliberately don't count, so they must go through
/// [`ResMut::bypass_change_detection`]. Rebuilding the table despawns its rows,
/// and doing that mid-rename would destroy the entity holding keyboard focus.
///
/// [`ResMut::bypass_change_detection`]: bevy::ecs::change_detection::DetectChangesMut::bypass_change_detection
#[derive(Resource, Debug, Default)]
pub struct SonarCatalog {
    entries: HashMap<SonarSignature, CatalogEntry>,
    next_order: u32,
}

impl SonarCatalog {
    /// Color used by a return whose signature has never been logged.
    pub const DEFAULT_COLOR: Color = palette::PHOSPHORE_HIGH;

    /// Color already on file for this signature, if any.
    pub fn color_of(&self, signature: &SonarSignature) -> Option<Color> {
        self.entries.get(signature).map(|entry| entry.color)
    }

    /// Files a return whose signature isn't on record yet, and hands back the
    /// color to draw it with.
    pub fn observe(&mut self, signature: &SonarSignature) -> Color {
        if let Some(entry) = self.entries.get(signature) {
            return entry.color;
        }

        let ancestor = self
            .entries
            .iter()
            .filter(|(candidate, _)| candidate.is_ancestor_of(signature))
            .max_by_key(|(candidate, _)| candidate.count_features())
            .map(|(candidate, entry)| (*candidate, entry.name.clone(), entry.color));

        let (name, color) = match &ancestor {
            Some((_, name, color)) => (name.clone(), *color),
            None => (self.next_designation(), Self::DEFAULT_COLOR),
        };

        // The coarser row can never be produced again, so it only stays on file
        // as an inheritance source for the siblings still to be found.
        if let Some((ancestor, ..)) = ancestor
            && let Some(entry) = self.entries.get_mut(&ancestor)
        {
            entry.superseded = true;
        }

        let order = self.next_order;
        self.next_order += 1;
        self.entries.insert(
            *signature,
            CatalogEntry {
                name,
                color,
                is_new: true,
                order,
                superseded: false,
            },
        );

        color
    }

    fn next_designation(&self) -> String {
        format!("CONTACT-{:02}", self.next_order + 1)
    }

    /// The rows to display, in discovery order.
    pub fn rows(&self) -> Vec<(SonarSignature, CatalogEntry)> {
        let mut rows: Vec<_> = self
            .entries
            .iter()
            .filter(|(_, entry)| !entry.superseded)
            .map(|(signature, entry)| (*signature, entry.clone()))
            .collect();
        rows.sort_by_key(|(_, entry)| entry.order);
        rows
    }

    pub fn get(&self, signature: &SonarSignature) -> Option<&CatalogEntry> {
        self.entries.get(signature)
    }

    /// How many rows are still flagged as new, for the drawer's badge.
    pub fn new_count(&self) -> usize {
        self.entries
            .values()
            .filter(|entry| entry.is_new && !entry.superseded)
            .count()
    }

    /// Leaves the table's shape alone: call it through
    /// `bypass_change_detection()` so it doesn't trigger a rebuild.
    pub fn set_name(&mut self, signature: &SonarSignature, name: String) {
        if let Some(entry) = self.entries.get_mut(signature) {
            entry.name = name;
        }
    }

    /// Leaves the table's shape alone: call it through
    /// `bypass_change_detection()` so it doesn't trigger a rebuild.
    pub fn set_color(&mut self, signature: &SonarSignature, color: Color) {
        if let Some(entry) = self.entries.get_mut(signature) {
            entry.color = color;
        }
    }

    /// Called once the player has seen the new rows. Guard it with
    /// [`Self::new_count`] so it only marks the resource changed when it has
    /// something to clear.
    pub fn clear_new_flags(&mut self) {
        for entry in self.entries.values_mut() {
            entry.is_new = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::sonar::feature::{SonarSignature, SurfaceReflexivity, SurfaceTexture};

    fn hard() -> SonarSignature {
        SonarSignature {
            texture: Some(SurfaceTexture::Hard),
            ..default()
        }
    }

    fn soft() -> SonarSignature {
        SonarSignature {
            texture: Some(SurfaceTexture::Soft),
            ..default()
        }
    }

    fn hard_reflective() -> SonarSignature {
        SonarSignature {
            texture: Some(SurfaceTexture::Hard),
            reflexivity: Some(SurfaceReflexivity::Strong),
            ..default()
        }
    }

    fn hard_absorbent() -> SonarSignature {
        SonarSignature {
            texture: Some(SurfaceTexture::Hard),
            reflexivity: Some(SurfaceReflexivity::Weak),
            ..default()
        }
    }

    #[test]
    fn a_finer_row_does_not_inherit_from_an_unrelated_one() {
        let mut catalog = SonarCatalog::default();
        catalog.observe(&hard());
        catalog.set_name(&hard(), "ROCK".to_string());
        catalog.set_color(&hard(), palette::AZURE_HIGH);

        // Reflexivity unlocks, then a *creature* is scanned. It shares no
        // reading with the rock, so it must not inherit its name or color, and
        // must not push the rock's row off the table.
        let soft_strong = SonarSignature {
            texture: Some(SurfaceTexture::Soft),
            reflexivity: Some(SurfaceReflexivity::Strong),
            ..default()
        };
        assert_eq!(catalog.observe(&soft_strong), SonarCatalog::DEFAULT_COLOR);

        let rows = catalog.rows();
        assert_eq!(rows.len(), 2, "the rock's row must survive");
        assert!(rows.iter().any(|(_, entry)| entry.name == "ROCK"));
    }

    #[test]
    fn first_return_creates_a_row() {
        let mut catalog = SonarCatalog::default();

        assert_eq!(catalog.observe(&hard()), SonarCatalog::DEFAULT_COLOR);

        let rows = catalog.rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].1.name, "CONTACT-01");
        assert!(rows[0].1.is_new);
    }

    #[test]
    fn known_return_does_not_create_a_second_row() {
        let mut catalog = SonarCatalog::default();
        catalog.observe(&hard());
        catalog.set_color(&hard(), palette::AZURE_HIGH);

        assert_eq!(catalog.observe(&hard()), palette::AZURE_HIGH);
        assert_eq!(catalog.rows().len(), 1);
    }

    #[test]
    fn unlocking_a_feature_splits_the_row_and_hands_down_name_and_color() {
        let mut catalog = SonarCatalog::default();
        catalog.observe(&hard());
        catalog.set_name(&hard(), "ROCK".to_string());
        catalog.set_color(&hard(), palette::AZURE_HIGH);

        // A feature unlocks, so the same rock now reads one notch finer.
        assert_eq!(catalog.observe(&hard_reflective()), palette::AZURE_HIGH);

        let rows = catalog.rows();
        assert_eq!(rows.len(), 1, "the coarser row steps aside");
        assert_eq!(rows[0].0, hard_reflective());
        assert_eq!(rows[0].1.name, "ROCK", "the name is inherited");
        assert_eq!(rows[0].1.color, palette::AZURE_HIGH);
        assert!(rows[0].1.is_new, "the split is flagged for the player");
    }

    #[test]
    fn both_siblings_inherit_from_the_same_superseded_row() {
        let mut catalog = SonarCatalog::default();
        catalog.observe(&hard());
        catalog.set_name(&hard(), "ROCK".to_string());
        catalog.set_color(&hard(), palette::AZURE_HIGH);

        catalog.observe(&hard_reflective());
        // The second sibling is found later and must still inherit.
        assert_eq!(catalog.observe(&hard_absorbent()), palette::AZURE_HIGH);

        let rows = catalog.rows();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|(_, entry)| entry.name == "ROCK"));
        assert!(
            rows.iter()
                .all(|(_, entry)| entry.color == palette::AZURE_HIGH)
        );
    }

    #[test]
    fn an_unrelated_signature_gets_a_fresh_row() {
        let mut catalog = SonarCatalog::default();
        catalog.observe(&hard());
        catalog.set_color(&hard(), palette::AZURE_HIGH);

        // TEXTURE=SOFT shares no reading with HARD, so it inherits nothing.
        assert_eq!(catalog.observe(&soft()), SonarCatalog::DEFAULT_COLOR);
        assert_eq!(catalog.rows().len(), 2);
    }

    #[test]
    fn opening_the_panel_clears_the_new_markers() {
        let mut catalog = SonarCatalog::default();
        catalog.observe(&hard());
        assert_eq!(catalog.new_count(), 1);

        catalog.clear_new_flags();
        assert_eq!(catalog.new_count(), 0);
        assert!(catalog.rows().iter().all(|(_, entry)| !entry.is_new));
    }
}
