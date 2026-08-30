//! The gameplay.

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_seedling::{
    context::graph::MusicPool,
    firewheel::Volume,
    sample::{AudioSample, SamplePlayer},
};

use crate::{asset_tracking::LoadResource, screens::Screen};

pub mod beacon;
mod camera;
pub mod creatures;
pub mod drone;
mod ldtk;
mod note;
mod sonar;
pub mod sound;
mod terrain;
mod ui;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(PhysicsPlugins::default());
    app.insert_resource(Gravity(Vec2::ZERO));

    app.load_resource::<SoundAssets>();

    app.add_plugins((
        beacon::plugin,
        camera::plugin,
        creatures::plugin,
        drone::plugin,
        ldtk::plugin,
        note::plugin,
        sonar::plugin,
        sound::plugin,
        terrain::plugin,
        ui::plugin,
    ));

    app.add_systems(OnEnter(Screen::Gameplay), start_ambient_music);
}

#[derive(Default, PhysicsLayer)]
pub enum PhysicLayer {
    #[default]
    Terrain,
    Drone,
    Creature,
    Beacon,
}

// Music

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
struct SoundAssets {
    #[dependency]
    music: Handle<AudioSample>,
}

impl FromWorld for SoundAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            music: assets.load("audio/ambient/abyss_bed_loop.ogg"),
        }
    }
}

fn start_ambient_music(mut commands: Commands, credits_music: Res<SoundAssets>) {
    commands.spawn((
        Name::new("Ambient Music"),
        DespawnOnExit(Screen::Gameplay),
        MusicPool,
        SamplePlayer::new(credits_music.music.clone())
            .looping()
            .with_volume(Volume::Decibels(-15.)),
    ));
}
