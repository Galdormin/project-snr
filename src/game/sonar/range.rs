//! How far the sonar reaches, and the ring that previews it.

use bevy::prelude::*;

use crate::{
    game::{drone::Drone, sonar::PIXELS_PER_METER},
    screens::Screen,
    theme::palette::HUD_ACCENT,
};

/// Dashes making up the preview ring.
const PREVIEW_DASHES: usize = 64;
/// How much of each dash's slice is actually drawn.
const PREVIEW_DASH_FILL: f32 = 0.55;
/// How long the ring stays up after the last adjustment.
const PREVIEW_LINGER: f32 = 1.2;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<ScanRange>();
    app.init_resource::<RangePreview>();

    app.add_systems(
        Update,
        draw_range_preview.run_if(in_state(Screen::Gameplay)),
    );
}

/// How far the next ping reaches, in meters. The player sets it from the HUD.
#[derive(Resource, Reflect, Clone, Copy, Debug)]
#[reflect(Resource)]
pub struct ScanRange(pub f32);

impl ScanRange {
    /// The ends of the range the HUD's slider offers.
    pub const MIN: f32 = 20.0;
    pub const MAX: f32 = 200.0;

    /// The same distance in world units, which is what a scan is cast in.
    pub fn in_pixels(self) -> f32 {
        self.0 * PIXELS_PER_METER
    }
}

impl Default for ScanRange {
    fn default() -> Self {
        Self(150.0)
    }
}

/// Keeps the preview ring up for a moment after the last adjustment.
#[derive(Resource)]
pub struct RangePreview(Timer);

impl Default for RangePreview {
    fn default() -> Self {
        let mut timer = Timer::from_seconds(PREVIEW_LINGER, TimerMode::Once);
        // The ring starts fade out.
        timer.finish();
        Self(timer)
    }
}

impl RangePreview {
    /// Call on every adjustment
    pub fn show(&mut self) {
        self.0.reset();
    }
}

/// Draws the range the player is dialling in as a dashed ring around the drone.
fn draw_range_preview(
    mut gizmos: Gizmos,
    time: Res<Time>,
    mut preview: ResMut<RangePreview>,
    range: Res<ScanRange>,
    drone: Single<&GlobalTransform, With<Drone>>,
) {
    if preview.0.tick(time.delta()).is_finished() {
        return;
    }

    let transform = *drone;

    let color = HUD_ACCENT.with_alpha(preview.0.fraction_remaining());
    let center = transform.translation().truncate();
    let dash = std::f32::consts::TAU / PREVIEW_DASHES as f32;
    for index in 0..PREVIEW_DASHES {
        let start = Rot2::radians(index as f32 * dash);
        gizmos.arc_2d(
            Isometry2d::new(center, start),
            dash * PREVIEW_DASH_FILL,
            range.in_pixels(),
            color,
        );
    }
}
