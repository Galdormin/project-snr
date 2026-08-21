//! Code for the active sonar scan

use avian2d::spatial_query::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;

use crate::{game::PhysicLayer, theme::palette};

const DETECTED_POINTS_DISPLAY_DURATION: f32 = 3.0;
const DETECTED_POINT_RADIUS: f32 = 2.0;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, load_sonar_point_assets);
    app.add_systems(
        PostUpdate,
        handle_new_scan.after(TransformSystems::Propagate),
    );
    app.add_systems(Update, (animate_sonar_points, despawn_expired_scans));
}

#[derive(Resource)]
struct SonarPointAssets {
    mesh: Handle<Mesh>,
    color: Color,
}

fn load_sonar_point_assets(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(SonarPointAssets {
        mesh: meshes.add(Circle::new(DETECTED_POINT_RADIUS)),
        color: palette::PHOSPHORE_HIGH,
    });
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

#[derive(Component)]
struct SonarPointVisual {
    timer: Timer,
}

impl Default for SonarPointVisual {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(DETECTED_POINTS_DISPLAY_DURATION, TimerMode::Once),
        }
    }
}

fn handle_new_scan(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    sonar_point_assets: Res<SonarPointAssets>,
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

        for point in &points {
            commands.spawn((
                Mesh2d(sonar_point_assets.mesh.clone()),
                MeshMaterial2d(materials.add(ColorMaterial::from(sonar_point_assets.color))),
                Transform::from_translation(point.position.extend(1.0)),
                SonarPointVisual::default(),
            ));
        }

        commands
            .entity(entity)
            .insert((DetectedPoints::new(points), DetectedPointsTimer::default()));
    }
}

fn despawn_expired_scans(
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

fn animate_sonar_points(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut points: Query<(
        Entity,
        &mut Transform,
        &MeshMaterial2d<ColorMaterial>,
        &mut SonarPointVisual,
    )>,
) {
    for (entity, mut transform, material, mut visual) in &mut points {
        if visual.timer.tick(time.delta()).is_finished() {
            commands.entity(entity).despawn();
            continue;
        }

        let remaining = visual.timer.remaining_secs() / visual.timer.duration().as_secs_f32();
        transform.scale = Vec3::splat(remaining);
        if let Some(mut material) = materials.get_mut(&material.0) {
            material.color.set_alpha(remaining);
        }
    }
}
