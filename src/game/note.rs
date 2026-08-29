/// Code for the note
use bevy::{
    asset::{AssetLoader, AsyncReadExt, LoadContext, io::Reader},
    prelude::*,
};
use thiserror::Error;

use crate::game::beacon::{BeaconDownloadCompleted, BeaconId};

pub(super) fn plugin(app: &mut App) {
    app.init_asset::<Note>();
    app.init_asset_loader::<NoteLoader>();

    app.add_systems(Startup, read_all_notes);
    app.add_systems(Update, unlock_note_on_download_completed);
}

/// Fences the front matter off from the prose, on its own line at both ends.
const FRONT_MATTER_FENCE: &str = "+++";
const NOTES: &[&str] = &["notes/cave_01.note"];

#[derive(Asset, Reflect)]
pub struct Note {
    pub text: String,
    pub metadata: NoteMetadata,
}

#[derive(Reflect, serde::Deserialize)]
pub struct NoteMetadata {
    pub beacon_id: BeaconId,
    pub title: String,
    pub date: String,
}

/// Status of a [`Note`]
#[derive(Default, PartialEq, Eq)]
pub enum NoteStatus {
    #[default]
    Locked,
    Unlocked {
        unread: bool,
    },
}

/// Store a [`Handle`] to [`Note`] and the [`NoteStatus`]
pub struct NoteItem {
    pub note: Handle<Note>,
    pub status: NoteStatus,
}

/// Store all [`Note`]
#[derive(Resource)]
pub struct NoteCatalog {
    notes: Vec<NoteItem>,
}

/// Everything that can go wrong reading a `.note`.
#[derive(Error, Debug)]
pub enum NoteLoaderError {
    #[error("failed to read the note: {0}")]
    Io(#[from] std::io::Error),
    #[error("the note doesn't open with a `{FRONT_MATTER_FENCE}` front matter block")]
    MissingFrontMatter,
    #[error("the front matter block is never closed by a second `{FRONT_MATTER_FENCE}`")]
    UnterminatedFrontMatter,
    #[error("failed to parse the front matter: {0}")]
    Metadata(#[from] toml::de::Error),
}

/// Splits a `.note` into its front matter and its prose.
fn parse_note(data: &str) -> Result<Note, NoteLoaderError> {
    let opened = data
        .trim_start()
        .strip_prefix(FRONT_MATTER_FENCE)
        .ok_or(NoteLoaderError::MissingFrontMatter)?;

    let (header, text) = opened
        .split_once(FRONT_MATTER_FENCE)
        .ok_or(NoteLoaderError::UnterminatedFrontMatter)?;

    Ok(Note {
        metadata: toml::from_str(header)?,
        text: text.trim().to_owned(),
    })
}

/// [`AssetLoader`] for [`Note`]
#[derive(Default, TypePath)]
struct NoteLoader;

impl AssetLoader for NoteLoader {
    type Asset = Note;
    type Settings = ();
    type Error = NoteLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &Self::Settings,
        _: &mut LoadContext<'_>,
    ) -> Result<Note, NoteLoaderError> {
        let mut data = String::new();
        reader.read_to_string(&mut data).await?;
        parse_note(&data)
    }

    fn extensions(&self) -> &[&str] {
        &["note"]
    }
}

fn read_all_notes(mut commands: Commands, asset_server: Res<AssetServer>) {
    let notes = NOTES
        .iter()
        .map(|path| NoteItem {
            note: asset_server.load(*path),
            status: NoteStatus::default(),
        })
        .collect();

    commands.insert_resource(NoteCatalog { notes });
}

fn unlock_note_on_download_completed(
    mut downloads: MessageReader<BeaconDownloadCompleted>,
    notes: Res<Assets<Note>>,
    mut catalog: ResMut<NoteCatalog>,
) -> Result<()> {
    for download in downloads.read() {
        catalog
            .notes
            .iter_mut()
            .filter(|n| {
                notes
                    .get(n.note.id())
                    .is_some_and(|n| n.metadata.beacon_id == download.beacon_id)
            })
            .for_each(|n| {
                if n.status == NoteStatus::Locked {
                    n.status = NoteStatus::Unlocked { unread: true };
                }
            })
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTE: &str = r#"
+++
beacon_id = "15a-685"
title = "T"
date = "1975/04/23"
+++

lorem ipsum
dolor sit
"#;

    #[test]
    fn parse_front_matter_and_text() {
        let note = parse_note(NOTE).unwrap();
        assert_eq!(note.metadata.beacon_id, "15a-685");
        assert_eq!(note.metadata.date, "1975/04/23");
        assert_eq!(note.text, "lorem ipsum\ndolor sit");
    }

    #[test]
    fn error_without_front_matter() {
        assert!(matches!(
            parse_note("lorem ipsum"),
            Err(NoteLoaderError::MissingFrontMatter)
        ));
    }

    #[test]
    fn error_on_unclosed_front_matter() {
        assert!(matches!(
            parse_note("+++\ntitle = \"T\"\n"),
            Err(NoteLoaderError::UnterminatedFrontMatter)
        ));
    }

    #[test]
    fn error_on_malformed_front_matter() {
        assert!(matches!(
            parse_note("+++\nnot toml\n+++\n"),
            Err(NoteLoaderError::Metadata(_))
        ));
    }
}
