//! The log of the last sonar scan.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::game::sonar::feature::SonarSignature;

/// One logged contact.
#[derive(Clone, Debug)]
pub struct SonarContact {
    /// What the sonar resolved the surface as.
    pub signature: SonarSignature,
    /// Distance of the closest return with this signature, in meters.
    pub distance: f32,
    /// Number of point scanned
    pub counts: usize,
}

/// Record all [`SonarContact`] for the last scan
#[derive(Resource, Debug, Default)]
pub struct SonarContactRecord {
    contacts: HashMap<SonarSignature, SonarContact>,
}

impl SonarContactRecord {
    /// Clear the current record
    pub fn clear(&mut self) {
        self.contacts.clear();
    }

    /// Files an arriving echo
    pub fn record(&mut self, signature: SonarSignature, distance: f32) {
        self.contacts
            .entry(signature)
            .and_modify(|c| {
                c.distance = distance.min(c.distance);
                c.counts += 1;
            })
            .or_insert(SonarContact {
                signature,
                distance,
                counts: 1,
            });
    }

    /// The logged contacts, most occured first, then closest.
    pub fn contacts(&self) -> Vec<&SonarContact> {
        let mut contacts: Vec<_> = self.contacts.values().collect();
        contacts.sort_by(|left, right| {
            right
                .counts
                .cmp(&left.counts)
                .then(left.distance.total_cmp(&right.distance))
        });
        contacts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::sonar::feature::{SonarSignature, SurfaceTexture};

    fn signature(texture: SurfaceTexture) -> SonarSignature {
        SonarSignature {
            texture: Some(texture),
            ..default()
        }
    }

    #[test]
    fn test_count_increase_and_minimal_distance() {
        let mut contacts = SonarContactRecord::default();

        contacts.record(signature(SurfaceTexture::Hard), 90.0);
        assert_eq!(contacts.contacts().len(), 1);
        assert_eq!(contacts.contacts()[0].distance, 90.0);
        assert_eq!(contacts.contacts()[0].counts, 1);

        contacts.record(signature(SurfaceTexture::Hard), 40.0);
        assert_eq!(contacts.contacts().len(), 1);
        assert_eq!(contacts.contacts()[0].distance, 40.0);
        assert_eq!(contacts.contacts()[0].counts, 2);
    }

    #[test]
    fn test_two_signatures() {
        let mut contacts = SonarContactRecord::default();
        contacts.record(signature(SurfaceTexture::Hard), 40.0);
        contacts.record(signature(SurfaceTexture::Soft), 18.0);

        assert_eq!(contacts.contacts().len(), 2);
        assert!(
            contacts
                .contacts
                .contains_key(&signature(SurfaceTexture::Hard))
        );
        assert!(
            contacts
                .contacts
                .contains_key(&signature(SurfaceTexture::Soft))
        );
    }
}
