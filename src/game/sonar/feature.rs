//! Surface properties the sonar can learn to tell apart.
//!
//! Features are unlocked one at a time over the course of the game. A locked
//! feature is completely invisible: it never reaches a [`SonarSignature`] and never
//! gets a column in the calibration table.

use std::hash::Hash;

use bevy::prelude::*;

/// A property of a scanned surface that the sonar can distinguish once unlocked.
pub trait SonarFeature: Copy + Eq + Hash + Send + Sync + 'static {
    /// Label of the Feature.
    const LABEL: &'static str;

    /// Every variant of the feature.
    fn variants() -> &'static [Self];

    /// Label of the current variant.
    fn label(&self) -> &'static str;
}

/// Component to insert on Wall or Creature to specify the value of the Features
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
pub struct SurfaceFeatures {
    pub texture: SurfaceTexture,
    pub reflexivity: SurfaceReflexivity,
    pub passage: SurfacePassage,
}

/// Signature for the [`SonarCatalog`].
///
/// If a feature is locked, then the field in the signature is `None`.
///
/// [`SonarCatalog`]: crate::game::sonar::catalog::SonarCatalog
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SonarSignature {
    pub texture: Option<SurfaceTexture>,
    pub reflexivity: Option<SurfaceReflexivity>,
    pub passage: Option<SurfacePassage>,
}

impl SonarSignature {
    /// True if `self` is a strictly coarser reading *of the same surface*: it
    /// resolves fewer features, and every feature it does resolve agrees with
    /// `other`.
    pub(super) fn is_ancestor_of(&self, other: &Self) -> bool {
        fn agrees<T: PartialEq>(mine: Option<T>, theirs: Option<T>) -> bool {
            mine.is_none() || mine == theirs
        }

        self.count_features() < other.count_features()
            && agrees(self.texture, other.texture)
            && agrees(self.reflexivity, other.reflexivity)
            && agrees(self.passage, other.passage)
    }

    pub(super) fn count_features(&self) -> usize {
        self.texture.is_some() as usize
            + self.reflexivity.is_some() as usize
            + self.passage.is_some() as usize
    }
}

/// Resource that describe all unlocked features
#[derive(Resource, Reflect, Clone, Debug)]
#[reflect(Resource)]
pub struct SonarFeaturesStatus {
    pub texture: bool,
    pub reflexivity: bool,
    pub passage: bool,
}

impl Default for SonarFeaturesStatus {
    fn default() -> Self {
        Self {
            texture: true,
            reflexivity: true,
            passage: true,
        }
    }
}

impl SonarFeaturesStatus {
    /// Return [`SonarSignature`] based on the component [`SurfaceFeatures`] and unlocked features
    pub fn signature_of(&self, features: &SurfaceFeatures) -> SonarSignature {
        SonarSignature {
            texture: self.texture.then_some(features.texture),
            reflexivity: self.reflexivity.then_some(features.reflexivity),
            passage: self.passage.then_some(features.passage),
        }
    }

    /// Return the `LABEL` of [`SonarFeature`] for every unlocked features
    pub fn unlocked_names(&self) -> Vec<&'static str> {
        let mut unlocked_features = Vec::new();

        if self.texture {
            unlocked_features.push(SurfaceTexture::LABEL);
        }
        if self.reflexivity {
            unlocked_features.push(SurfaceReflexivity::LABEL);
        }
        if self.passage {
            unlocked_features.push(SurfacePassage::LABEL);
        }
        unlocked_features
    }

    /// Return the name of the variant for each unlocked features or None if the feature was locked
    /// when the [`SonarSignature`] was created.
    pub fn features_of(&self, signature: &SonarSignature) -> Vec<Option<&'static str>> {
        let mut unlocked_features = Vec::new();

        if self.texture {
            unlocked_features.push(signature.texture.map(|f| f.label()));
        }
        if self.reflexivity {
            unlocked_features.push(signature.reflexivity.map(|f| f.label()));
        }
        if self.passage {
            unlocked_features.push(signature.passage.map(|f| f.label()));
        }
        unlocked_features
    }
}

// -----
// List of all features
// -----

/// Physical texture of the surface, as perceived by the sonar.
///
/// Creature skin is soft and rock is hard.
#[derive(Reflect, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceTexture {
    Hard,
    Soft,
}

impl SonarFeature for SurfaceTexture {
    const LABEL: &'static str = "TEXTURE";

    fn variants() -> &'static [Self] {
        &[Self::Soft, Self::Hard]
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Soft => "SOFT",
            Self::Hard => "HARD",
        }
    }
}

/// Reflexivity of the surface
#[derive(Reflect, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceReflexivity {
    Strong,
    Medium,
    Weak,
}

impl SonarFeature for SurfaceReflexivity {
    const LABEL: &'static str = "REFLEXIVITY";

    fn variants() -> &'static [Self] {
        &[Self::Strong, Self::Medium, Self::Weak]
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Strong => "STRONG",
            Self::Medium => "MEDIUM",
            Self::Weak => "WEAK",
        }
    }
}

/// Can the surface be pass through
#[derive(Reflect, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfacePassage {
    Solid,
    Porous,
}

impl SonarFeature for SurfacePassage {
    const LABEL: &'static str = "PASSAGE";

    fn variants() -> &'static [Self] {
        &[Self::Solid, Self::Porous]
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Solid => "SOLID",
            Self::Porous => "POROUS",
        }
    }
}
