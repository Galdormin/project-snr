//! Code for the active sonar scan

use avian2d::spatial_query::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;

use crate::{
    AppSystems, PausableSystems,
    game::{
        PhysicLayer,
        sonar::{
            PIXELS_PER_METER,
            catalog::SonarCatalog,
            contacts::SonarContactRecord,
            feature::{SonarFeaturesStatus, SurfaceFeatures},
        },
        sound::{NoiseEvent, NoiseType},
    },
};

/// Speed of sound in the game world, in meters per second.
const SOUND_SPEED: f32 = 500.0;
/// How long a sonar point stays visible once activated.
const DETECTED_POINT_DISPLAY_DURATION: f32 = 6.0;
const DETECTED_POINT_RADIUS: f32 = 2.5;
/// Random jitter applied to a point's rendered position, so a flat wall doesn't
/// render as a perfectly straight line of dots.
const DETECTED_POINT_JITTER: f32 = 3.0;
/// Seconds of echo delay per pixel of distance: round tripa,
pub const ECHO_DELAY_PER_PIXEL: f32 = 2.0 / (PIXELS_PER_METER * SOUND_SPEED);

pub(super) fn plugin(app: &mut App) {
    app.add_message::<ScanStarted>();
    app.add_message::<ScanCompleted>();

    app.add_systems(Startup, load_sonar_point_assets);
    app.add_systems(
        Update,
        tick_sonar_cooldown_timer
            .in_set(AppSystems::TickTimers)
            .in_set(PausableSystems),
    );
    app.add_systems(
        PostUpdate,
        (handle_new_scan, despawn_childless_scans)
            .chain()
            .after(TransformSystems::Propagate),
    );

    app.add_systems(
        Update,
        (
            finish_listening,
            spawn_sonar_point_visuals,
            animate_sonar_points,
        )
            .chain(),
    );
}

/// A sonar set, on whatever carries it.
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct Sonar {
    /// Runs down the echo travel time of the ping in flight
    listening: Timer,
}

impl Default for Sonar {
    fn default() -> Self {
        // Nothing is in flight until the first ping.
        let mut listening = Timer::from_seconds(0.0, TimerMode::Once);
        listening.finish();
        Self { listening }
    }
}

impl Sonar {
    /// Starts listening for the echoes of a ping of the given range.
    pub fn listen(&mut self, range: f32) {
        self.listening = Timer::from_seconds(range * ECHO_DELAY_PER_PIXEL, TimerMode::Once);
    }
}

/// Sonar cooldown
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
pub struct SonarCooldown {
    pub timer: Timer,
}

impl Default for SonarCooldown {
    fn default() -> Self {
        Self::new(3.0)
    }
}

impl SonarCooldown {
    fn new(cooldown: f32) -> Self {
        let mut timer = Timer::from_seconds(cooldown, TimerMode::Once);
        timer.finish();

        Self { timer }
    }
}

/// Fired when a scan started.
#[derive(Message, Clone, Copy, Debug)]
pub struct ScanStarted;

/// Fired once every echo of a scan has had time to come back.
#[derive(Message, Clone, Copy, Debug)]
pub struct ScanCompleted;

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

#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
pub struct SonarPoint {
    pub(super) entity: Entity,
    position: Vec2,
    distance_to_sonar: f32,
}

impl SonarPoint {
    fn new(entity: Entity, position: Vec2, distance_to_sonar: f32) -> Self {
        Self {
            entity,
            position,
            distance_to_sonar,
        }
    }

    /// How long the sonar takes to hear this point's echo.
    fn echo_delay(&self) -> f32 {
        self.distance_to_sonar * ECHO_DELAY_PER_PIXEL
    }

    /// How far away the point is, in the units the HUD reads out.
    fn distance_in_meters(&self) -> f32 {
        self.distance_to_sonar / PIXELS_PER_METER
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
    mut started: MessageWriter<ScanStarted>,
    mut record: ResMut<SonarContactRecord>,
    scans: Query<(Entity, &ActiveScan, &GlobalTransform), Added<ActiveScan>>,
) {
    for (entity, scan, transform) in scans {
        // The reading is per ping: whatever the last one heard is stale now.
        record.clear();
        started.write(ScanStarted);

        let filter = SpatialQueryFilter::from_mask([
            PhysicLayer::Terrain,
            PhysicLayer::Creature,
            PhysicLayer::Beacon,
        ]);
        let origin = transform.translation().truncate();

        // The ping itself is a noise: audible exactly as far as the scan can see.
        noise_events.write(NoiseEvent::from_loudness(
            origin,
            NoiseType::Sonar,
            scan.range,
        ));

        let points: Vec<SonarPoint> = (0..360)
            .filter(|angle| angle % 2 == 0)
            .map(|angle| Dir2::from_angle((angle as f32).to_radians()))
            .filter_map(|dir| {
                if let Some(hit_data) =
                    spatial_query.cast_ray(origin, dir, scan.range, false, &filter)
                {
                    let position = origin + dir * hit_data.distance;
                    Some(SonarPoint::new(
                        hit_data.entity,
                        position,
                        hit_data.distance,
                    ))
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

fn tick_sonar_cooldown_timer(time: Res<Time>, sonars: Query<&mut SonarCooldown>) {
    let delta = time.delta();
    for mut cooldown in sonars {
        cooldown.timer.tick(delta);
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

/// Announces a ping whose echoes have all come back
fn finish_listening(
    time: Res<Time>,
    mut sonar: Single<&mut Sonar>,
    mut completed: MessageWriter<ScanCompleted>,
) {
    if sonar.listening.tick(time.delta()).just_finished() {
        completed.write(ScanCompleted);
    }
}

/// Spawns a sonar point's mesh once its echo has had time to come back.
fn spawn_sonar_point_visuals(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    feat_status: Res<SonarFeaturesStatus>,
    sonar_point_assets: Res<SonarPointAssets>,
    mut catalog: ResMut<SonarCatalog>,
    mut contacts: ResMut<SonarContactRecord>,
    surfaces: Query<&SurfaceFeatures>,
    mut points: Query<
        (Entity, &SonarPoint, &mut SonarPointVisual),
        Without<MeshMaterial2d<ColorMaterial>>,
    >,
) {
    for (entity, point, mut visual) in &mut points {
        if !visual.activation_timer.tick(time.delta()).is_finished() {
            continue;
        }

        let Ok(signature) = surfaces
            .get(point.entity)
            .map(|features| feat_status.signature_of(features))
        else {
            error!(
                "Entity {} was scanned by sonar but has no SurfaceFeatures.",
                point.entity
            );
            continue;
        };

        // Only an unknown signature is a structural change: going through
        // `observe` for a known one would mark the catalog changed on every
        // arriving echo, and rebuild the table out from under a rename.
        let color = catalog
            .color_of(&signature)
            .unwrap_or_else(|| catalog.observe(&signature));

        // Record the contact
        contacts.record(signature, point.distance_in_meters());

        let material = materials.add(ColorMaterial::from(color));

        commands.entity(entity).insert((
            Mesh2d(sonar_point_assets.mesh.clone()),
            MeshMaterial2d(material),
        ));
    }
}

/// Fades out and grows a displayed sonar point until its display duration runs out, then despawns it.
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
        if visual.display_timer.tick(time.delta()).is_finished() {
            commands.entity(entity).despawn();
            continue;
        }

        let remaining =
            visual.display_timer.remaining_secs() / visual.display_timer.duration().as_secs_f32();
        transform.scale = Vec3::splat(remaining);
        if let Some(mut material) = materials.get_mut(&material.0) {
            material.color.set_alpha(remaining);
        }
    }
}
