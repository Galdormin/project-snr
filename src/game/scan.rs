//! Code for the active sonar scan

use avian2d::spatial_query::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;

use crate::{
    game::{PhysicLayer, creatures::Creature, sound::NoiseEvent},
    theme::palette,
};

/// Pixels per in-game meter, used to convert distances into real-world units
/// for the echo delay computation.
const PIXELS_PER_METER: f32 = 2.0;
/// Speed of sound in the game world, in meters per second.
const SOUND_SPEED: f32 = 500.0;
/// How long a sonar point stays visible once activated.
const DETECTED_POINT_DISPLAY_DURATION: f32 = 6.0;
const DETECTED_POINT_RADIUS: f32 = 2.5;
/// Random jitter applied to a point's rendered position, so a flat wall doesn't
/// render as a perfectly straight line of dots.
const DETECTED_POINT_JITTER: f32 = 3.0;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, load_sonar_point_assets);
    app.add_systems(
        PostUpdate,
        (handle_new_scan, despawn_childless_scans)
            .chain()
            .after(TransformSystems::Propagate),
    );
    app.add_systems(Update, animate_sonar_points);
}

#[derive(Resource)]
struct SonarPointAssets {
    mesh: Handle<Mesh>,
}

fn load_sonar_point_assets(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(SonarPointAssets {
        mesh: meshes.add(Circle::new(DETECTED_POINT_RADIUS)),
    });
}

#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
pub struct ActiveScan {
    pub range: f32,
}

impl ActiveScan {
    pub fn new(range: f32) -> Self {
        Self { range }
    }
}

/// Kind of entity scan by the sonar
#[derive(Reflect, Debug, Clone, Copy)]
pub enum SonarKind {
    Terrain,
    Creature(Entity),
}

impl SonarKind {
    fn get_point_color(&self) -> Color {
        match self {
            SonarKind::Terrain => palette::PHOSPHORE_HIGH,
            SonarKind::Creature(_) => palette::RED_HIGH,
        }
    }
}

#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
pub struct SonarPoint {
    pub position: Vec2,
    pub distance_to_sonar: f32,
    pub kind: SonarKind,
}

impl SonarPoint {
    /// Seconds of echo delay per pixel of distance: round tripa,
    const ECHO_DELAY_PER_PIXEL: f32 = 2.0 / (PIXELS_PER_METER * SOUND_SPEED);

    fn new(position: Vec2, distance_to_sonar: f32, kind: SonarKind) -> Self {
        Self {
            position,
            distance_to_sonar,
            kind,
        }
    }

    /// How long the sonar takes to hear this point's echo.
    fn echo_delay(&self) -> f32 {
        self.distance_to_sonar * Self::ECHO_DELAY_PER_PIXEL
    }
}

/// A sonar point's visibility
#[derive(Component)]
struct SonarPointVisual {
    /// Delay before the point becomes visible, based on its echo travel time.
    activation_timer: Timer,
    /// How long the point stays visible (fading out) once activated.
    display_timer: Timer,
}

impl SonarPointVisual {
    fn with_activation_delay(activation_delay: f32) -> Self {
        Self {
            activation_timer: Timer::from_seconds(activation_delay, TimerMode::Once),
            display_timer: Timer::from_seconds(DETECTED_POINT_DISPLAY_DURATION, TimerMode::Once),
        }
    }
}

fn handle_new_scan(
    mut commands: Commands,
    spatial_query: SpatialQuery,
    mut noise_events: MessageWriter<NoiseEvent>,
    scans: Query<(Entity, &ActiveScan, &GlobalTransform), Added<ActiveScan>>,
    creatures: Query<Entity, With<Creature>>,
) {
    for (entity, scan, transform) in scans {
        let filter = SpatialQueryFilter::from_mask([PhysicLayer::Terrain, PhysicLayer::Creature]);
        let origin = transform.translation().truncate();

        // The ping itself is a noise: audible exactly as far as the scan can see.
        noise_events.write(NoiseEvent::from_loudness(origin, scan.range));

        let points: Vec<SonarPoint> = (0..360)
            .filter(|angle| angle % 2 == 0)
            .map(|angle| Dir2::from_angle((angle as f32).to_radians()))
            .filter_map(|dir| {
                if let Some(hit_data) =
                    spatial_query.cast_ray(origin, dir, scan.range, false, &filter)
                {
                    let position = origin + dir * hit_data.distance;
                    let kind = if creatures.contains(hit_data.entity) {
                        SonarKind::Creature(hit_data.entity)
                    } else {
                        SonarKind::Terrain
                    };

                    Some(SonarPoint::new(position, hit_data.distance, kind))
                } else {
                    None
                }
            })
            .collect();

        for point in points {
            let local_position = point.position - origin
                + Vec2::new(
                    DETECTED_POINT_JITTER * rand::random_range(-1.0..=1.0),
                    DETECTED_POINT_JITTER * rand::random_range(-1.0..=1.0),
                );

            commands.spawn((
                ChildOf(entity),
                Transform::from_translation(local_position.extend(1.0)),
                SonarPointVisual::with_activation_delay(point.echo_delay()),
                point,
            ));
        }
    }
}

/// Despawns scan entities once all their sonar points have expired (or none
/// were ever detected), instead of tracking the scan's own lifetime.
fn despawn_childless_scans(
    mut commands: Commands,
    scans: Query<Entity, (With<ActiveScan>, Without<Children>)>,
) {
    for entity in scans {
        commands.entity(entity).despawn();
    }
}

fn animate_sonar_points(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    sonar_point_assets: Res<SonarPointAssets>,
    mut points: Query<(
        Entity,
        &mut Transform,
        Option<&MeshMaterial2d<ColorMaterial>>,
        &SonarPoint,
        &mut SonarPointVisual,
    )>,
) {
    for (entity, mut transform, material, point, mut visual) in &mut points {
        if !visual.activation_timer.tick(time.delta()).is_finished() {
            continue;
        }

        let material = match material {
            Some(material) => material.0.clone(),
            None => {
                let material = materials.add(ColorMaterial::from(point.kind.get_point_color()));
                commands.entity(entity).insert((
                    Mesh2d(sonar_point_assets.mesh.clone()),
                    MeshMaterial2d(material.clone()),
                ));
                material
            }
        };

        if visual.display_timer.tick(time.delta()).is_finished() {
            commands.entity(entity).despawn();
            continue;
        }

        let remaining =
            visual.display_timer.remaining_secs() / visual.display_timer.duration().as_secs_f32();
        transform.scale = Vec3::splat(remaining);
        if let Some(mut material) = materials.get_mut(&material) {
            material.color.set_alpha(remaining);
        }
    }
}
