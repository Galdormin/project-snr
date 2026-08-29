//! Code for beacon

use bevy::prelude::*;

use avian2d::prelude::*;
use bevy_ecs_ldtk::{EntityInstance, LdtkEntity, ldtk::ldtk_fields::LdtkFields};

use crate::{
    game::{
        PhysicLayer,
        drone::Drone,
        sonar::{
            PIXELS_PER_METER,
            feature::{SurfaceFeatures, SurfacePassage, SurfaceReflexivity, SurfaceTexture},
        },
        sound::{NoiseEvent, NoiseType},
    },
    screens::Screen,
};

/// How close the drone has to be, in meters, before it can connect to a beacon.
pub const CONNECTION_RANGE: f32 = 10.0;
/// How long a full data download takes, in seconds.
const DOWNLOAD_DURATION: f32 = 20.0;
/// How long the handshake takes when a beacon is already in range.
const HANDSHAKE_DURATION: f32 = 0.6;
/// How long the drone keeps calling out with nothing in range before it gives up.
const SEARCH_DURATION: f32 = 3.0;
/// Speeds below which the drone counts as holding still, in px/s and rad/s.
const STILL_LINEAR_SPEED: f32 = 3.0;
const STILL_ANGULAR_SPEED: f32 = 0.2;

pub(super) fn plugin(app: &mut App) {
    app.add_message::<BeaconDownloadCompleted>();
    app.add_message::<ConnectionToggleRequested>();
    app.init_resource::<BeaconConnection>();

    app.add_systems(Update, (detect_sound, respond_to_scan));
    app.add_systems(
        Update,
        (
            handle_connection_requests,
            drop_connection_on_movement,
            advance_search,
            download_beacon_data,
        )
            .chain()
            .run_if(in_state(Screen::Gameplay)),
    );
}

pub type BeaconId = String;

/// Marker component for all beacons
#[derive(Component, Default, Reflect, Clone, Debug)]
#[reflect(Component)]
pub struct Beacon(BeaconId);

impl Beacon {
    pub fn id(&self) -> String {
        format!("BCN-{}", self.0)
    }
}

impl From<&EntityInstance> for Beacon {
    fn from(value: &EntityInstance) -> Self {
        let id = value.get_string_field("id").unwrap().clone();
        Self(id)
    }
}

/// LDtk Bundle for a Silent creature.
#[derive(Bundle, LdtkEntity)]
pub struct BeaconBundle {
    #[from_entity_instance]
    beacon: Beacon,
    receiver: BeaconReceiver,
    emitter: BeaconEmitter,
    data: BeaconData,
    features: SurfaceFeatures,
    physics: BeaconPhysicsBundle,
}

impl Default for BeaconBundle {
    fn default() -> Self {
        Self {
            beacon: Default::default(),
            receiver: Default::default(),
            emitter: Default::default(),
            data: Default::default(),
            features: SurfaceFeatures {
                texture: SurfaceTexture::Hard,
                reflexivity: SurfaceReflexivity::Strong,
                passage: SurfacePassage::Porous,
            },
            physics: Default::default(),
        }
    }
}

/// LDtk Bundle for a Silent creature.
#[derive(Bundle, LdtkEntity)]
struct BeaconPhysicsBundle {
    rigid_body: RigidBody,
    collider: Collider,
    collision_layers: CollisionLayers,
}

impl Default for BeaconPhysicsBundle {
    fn default() -> Self {
        Self {
            rigid_body: RigidBody::Static,
            collider: Collider::circle(4.0),
            collision_layers: CollisionLayers::new(
                PhysicLayer::Beacon,
                [PhysicLayer::Creature, PhysicLayer::Drone],
            ),
        }
    }
}

#[derive(Component, Default, Reflect, Clone, Debug)]
#[reflect(Component)]
pub struct BeaconReceiver {
    sensitivity_db: f32,
}

#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
struct BeaconEmitter {
    range: f32,
    timer: Timer,
}

impl Default for BeaconEmitter {
    fn default() -> Self {
        Self::new(200.0)
    }
}

impl BeaconEmitter {
    fn new(range: f32) -> Self {
        let mut timer = Timer::from_seconds(0.0, TimerMode::Once);
        timer.finish();

        Self { range, timer }
    }

    fn emit_with_delay(&mut self, delay: f32) {
        self.timer = Timer::from_seconds(delay, TimerMode::Once);
    }
}

fn detect_sound(
    mut noise_events: MessageReader<NoiseEvent>,
    mut beacons: Query<(&BeaconReceiver, &mut BeaconEmitter, &Position)>,
) {
    for noise in noise_events.read() {
        if noise.noise_type != NoiseType::Sonar {
            continue;
        }

        for (receiver, mut emitter, position) in &mut beacons {
            if noise.db_at(position.0) >= receiver.sensitivity_db {
                emitter.emit_with_delay(noise.time_to_reach(position.0));
            }
        }
    }
}

fn respond_to_scan(
    time: Res<Time>,
    mut noise_event: MessageWriter<NoiseEvent>,
    beacons: Query<(&mut BeaconEmitter, &Position)>,
) {
    let delta = time.delta();
    for (mut emitter, position) in beacons {
        if emitter.timer.tick(delta).just_finished() {
            noise_event.write(NoiseEvent::from_loudness(
                position.0,
                NoiseType::Beacon,
                emitter.range,
            ));
        }
    }
}

/// The payload a beacon holds, and how much of it the drone has downloaded
/// TODO: Add a link to the downloaded data
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
pub struct BeaconData {
    downloaded: Timer,
    /// Emit several [`NoiseEvent`] during the download
    signal_emitter: Timer,
}

impl Default for BeaconData {
    fn default() -> Self {
        Self {
            downloaded: Timer::from_seconds(DOWNLOAD_DURATION, TimerMode::Once),
            signal_emitter: Timer::from_seconds(DOWNLOAD_DURATION / 10., TimerMode::Repeating),
        }
    }
}

impl BeaconData {
    /// How much of the payload is down, in `0.0..=1.0`.
    pub fn progress(&self) -> f32 {
        self.downloaded.fraction()
    }

    pub fn is_complete(&self) -> bool {
        self.downloaded.is_finished()
    }
}

/// The drone's data connection
#[derive(Resource, Default, Debug)]
pub struct BeaconConnection {
    state: ConnectionState,
}

#[derive(Default, Debug)]
enum ConnectionState {
    #[default]
    Idle,
    Searching {
        timer: Timer,
        target: Option<Entity>,
    },
    Failed,
    /// The connection is lost because the drone moved.
    Lost,
    /// The drone was moving when the player asked for a connection.
    Moving,
    Connected {
        beacon: Entity,
        downloading: bool,
    },
}

impl BeaconConnection {
    pub fn connected(&self) -> Option<Entity> {
        match self.state {
            ConnectionState::Connected { beacon, .. } => Some(beacon),
            _ => None,
        }
    }

    pub fn is_downloading(&self) -> bool {
        matches!(
            self.state,
            ConnectionState::Connected {
                downloading: true,
                ..
            }
        )
    }

    pub fn is_searching(&self) -> bool {
        matches!(self.state, ConnectionState::Searching { .. })
    }

    pub fn is_handshaking(&self) -> bool {
        matches!(
            self.state,
            ConnectionState::Searching {
                target: Some(_),
                ..
            }
        )
    }

    pub fn status(&self) -> &'static str {
        match self.state {
            ConnectionState::Idle => "",
            ConnectionState::Searching {
                target: Some(_), ..
            } => "CONNECTING...",
            ConnectionState::Searching { .. } => "SEARCHING FOR BEACON...",
            ConnectionState::Failed => "NO BEACON FOUND",
            ConnectionState::Lost => "CONNECTION LOST",
            ConnectionState::Moving => "HOLD STILL TO CONNECT",
            ConnectionState::Connected { .. } => "CONNECTED",
        }
    }

    pub fn start_download(&mut self) {
        if let ConnectionState::Connected { downloading, .. } = &mut self.state {
            *downloading = true;
        }
    }

    fn search(&mut self, target: Option<Entity>) {
        self.state = ConnectionState::Searching {
            timer: Timer::from_seconds(
                match target.is_some() {
                    true => HANDSHAKE_DURATION,
                    false => SEARCH_DURATION,
                },
                TimerMode::Once,
            ),
            target,
        };
    }
}

/// The player asking the connection to open or close.
#[derive(Message, Debug, Clone, Copy)]
pub struct ConnectionToggleRequested;

/// Raised the frame a beacon's payload finishes coming down.
#[derive(Message, Debug, Clone)]
pub struct BeaconDownloadCompleted {
    pub beacon_id: BeaconId,
}

impl BeaconDownloadCompleted {
    fn new(beacon_id: BeaconId) -> Self {
        Self { beacon_id }
    }
}

/// The nearest beacon the drone could talk to from where it is.
fn nearest_beacon_in_range(
    drone: Vec2,
    beacons: &Query<(Entity, &Position), With<Beacon>>,
) -> Option<Entity> {
    let range = CONNECTION_RANGE * PIXELS_PER_METER;
    beacons
        .iter()
        .map(|(beacon, position)| (beacon, position.0.distance(drone)))
        .filter(|(_, distance)| *distance <= range)
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(beacon, _)| beacon)
}

/// The connection only holds while the drone is "still"
fn is_still(linear: &LinearVelocity, angular: &AngularVelocity) -> bool {
    linear.0.length() <= STILL_LINEAR_SPEED && angular.0.abs() <= STILL_ANGULAR_SPEED
}

/// Opens or closes the connection on the player's word.
fn handle_connection_requests(
    mut requests: MessageReader<ConnectionToggleRequested>,
    mut connection: ResMut<BeaconConnection>,
    drone: Single<(&Position, &LinearVelocity, &AngularVelocity), With<Drone>>,
    beacons: Query<(Entity, &Position), With<Beacon>>,
) {
    if requests.is_empty() {
        return;
    }
    requests.clear();

    match connection.state {
        ConnectionState::Searching { .. } => return,
        ConnectionState::Connected { .. } => {
            connection.state = ConnectionState::Idle;
            return;
        }
        _ => {}
    }

    let (position, linear, angular) = *drone;
    if !is_still(linear, angular) {
        connection.state = ConnectionState::Moving;
        return;
    }

    connection.search(nearest_beacon_in_range(position.0, &beacons));
}

fn drop_connection_on_movement(
    mut connection: ResMut<BeaconConnection>,
    drone: Single<(&LinearVelocity, &AngularVelocity), With<Drone>>,
) {
    if !connection.is_searching() && connection.connected().is_none() {
        return;
    }

    let (linear, angular) = *drone;
    if !is_still(linear, angular) {
        connection.state = ConnectionState::Lost;
    }
}

fn advance_search(time: Res<Time>, mut connection: ResMut<BeaconConnection>) {
    let ConnectionState::Searching { timer, target } = &mut connection.state else {
        return;
    };

    if !timer.tick(time.delta()).just_finished() {
        return;
    }

    connection.state = match *target {
        Some(beacon) => ConnectionState::Connected {
            beacon,
            downloading: false,
        },
        None => ConnectionState::Failed,
    };
}

fn download_beacon_data(
    time: Res<Time>,
    mut connection: ResMut<BeaconConnection>,
    mut completed: MessageWriter<BeaconDownloadCompleted>,
    mut noise: MessageWriter<NoiseEvent>,
    mut beacons: Query<(&Beacon, &mut BeaconData, &Position)>,
) {
    if !connection.is_downloading() {
        return;
    }

    let Some(beacon_entity) = connection.connected() else {
        return;
    };
    let Ok((beacon, mut data, position)) = beacons.get_mut(beacon_entity) else {
        return;
    };

    if data.signal_emitter.tick(time.delta()).just_finished() {
        noise.write(NoiseEvent::from_loudness(
            position.0,
            NoiseType::Beacon,
            200.0,
        ));
    }

    if data.downloaded.tick(time.delta()).just_finished() {
        // The connection stays open, but there is nothing left to download
        connection.state = ConnectionState::Connected {
            beacon: beacon_entity,
            downloading: false,
        };
        completed.write(BeaconDownloadCompleted::new(beacon.0.clone()));
    }
}
