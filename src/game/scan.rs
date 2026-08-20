//! Code for the active sonar scan

use avian2d::spatial_query::{SpatialQuery, SpatialQueryFilter};
use bevy::color::palettes::css::GREEN;
use bevy::prelude::*;

use crate::game::PhysicLayer;

const DETECTED_POINTS_DISPLAY_DURATION: f32 = 3.0;
const DETECTED_POINT_GIZMO_RADIUS: f32 = 2.0;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        PostUpdate,
        handle_new_scan.after(TransformSystems::Propagate),
    );
    app.add_systems(Update, (draw_detected_points, despawn_expired_points));
}

#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
pub struct ActiveScan {
    range: f32,
}

impl ActiveScan {
    pub fn new(range: f32) -> Self {
        Self { range }
    }
}

#[derive(Debug, Reflect, Clone)]
pub struct SonarPoint {
    pub position: Vec2,
    pub distance_to_sonar: f32,
}

impl SonarPoint {
    fn new(position: Vec2, distance_to_sonar: f32) -> Self {
        Self {
            position,
            distance_to_sonar,
        }
    }
}

#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
pub struct DetectedPoints {
    points: Vec<SonarPoint>,
}

impl DetectedPoints {
    fn new(points: Vec<SonarPoint>) -> Self {
        Self { points }
    }
}

#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
struct DetectedPointsTimer(Timer);

impl Default for DetectedPointsTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(
            DETECTED_POINTS_DISPLAY_DURATION,
            TimerMode::Once,
        ))
    }
}

fn handle_new_scan(
    mut commands: Commands,
    spatial_query: SpatialQuery,
    scans: Query<(Entity, &ActiveScan, &GlobalTransform), Added<ActiveScan>>,
) {
    for (entity, scan, transform) in scans {
        let filter = SpatialQueryFilter::from_mask(PhysicLayer::Terrain);
        let origin = transform.translation().truncate();

        let points: Vec<SonarPoint> = (0..360)
            .map(|angle| Dir2::from_angle((angle as f32).to_radians()))
            .filter_map(|dir| {
                if let Some(hit_data) =
                    spatial_query.cast_ray(origin, dir, scan.range, false, &filter)
                {
                    let position = origin + dir * hit_data.distance;
                    Some(SonarPoint::new(position, hit_data.distance))
                } else {
                    None
                }
            })
            .collect();

        info!("points: {:?}", points);
        commands
            .entity(entity)
            .insert((DetectedPoints::new(points), DetectedPointsTimer::default()));
    }
}

fn draw_detected_points(mut gizmos: Gizmos, scans: Query<&DetectedPoints>) {
    for detected_points in scans {
        for point in &detected_points.points {
            gizmos.circle_2d(point.position, DETECTED_POINT_GIZMO_RADIUS, GREEN);
        }
    }
}

fn despawn_expired_points(
    mut commands: Commands,
    time: Res<Time>,
    mut scans: Query<(Entity, &mut DetectedPointsTimer)>,
) {
    for (entity, mut timer) in &mut scans {
        if timer.0.tick(time.delta()).just_finished() {
            commands.entity(entity).despawn();
        }
    }
}
