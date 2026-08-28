//! The sonar: scanning, the features it can resolve, and the player's log of
//! everything it has returned.

// `unlock` and friends are the API the next features will be wired through.
#![allow(dead_code)]

pub mod catalog;
pub mod contacts;
pub mod feature;
pub mod range;
pub mod scan;

use bevy::prelude::*;

use crate::game::sonar::{
    catalog::SonarCatalog, contacts::SonarContactRecord, feature::SonarFeaturesStatus,
};

/// Pixels per in-game meter: the sonar works in meters, the world in pixels.
pub const PIXELS_PER_METER: f32 = 2.0;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<SonarFeaturesStatus>();
    app.init_resource::<SonarCatalog>();
    app.init_resource::<SonarContactRecord>();

    app.add_plugins((range::plugin, scan::plugin));
}
