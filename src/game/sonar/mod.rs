//! The sonar: scanning, the features it can resolve, and the player's log of
//! everything it has returned.

// `unlock` and friends are the API the next features will be wired through.
#![allow(dead_code)]

pub mod catalog;
pub mod feature;
pub mod scan;

use bevy::prelude::*;

use crate::game::sonar::{catalog::SonarCatalog, feature::SonarFeaturesStatus};

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<SonarFeaturesStatus>();
    app.init_resource::<SonarCatalog>();

    app.add_plugins(scan::plugin);
}
