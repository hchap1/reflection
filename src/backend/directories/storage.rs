use directories::ProjectDirs;

use std::sync::OnceLock;
use std::path::PathBuf;
use std::fs::create_dir_all;

use crate::error::{Error, Res};

const QUALIFIER: &str = "com";
const ORGANISATION: &str = "hchap1";
const APPLICATION: &str = "reflection";

pub static STORAGE: OnceLock<Storage> = OnceLock::new();

pub struct Storage {

    /// The root folder of the data directory
    root: PathBuf,

    /// The specific database file (.db)
    database: PathBuf,

    /// Root directory of photo storage
    /// Actual photos should be <ROOT>/photos/<USER_ID>/<ALBUM_ID>/<PHOTO_ID>.ext
    photos: PathBuf
}

impl Storage {

    /// Find a suitable location and create filestructure
    /// If it already exists, do nothing
    pub fn initialise() -> Res<()> {
        
        let project_dirs = ProjectDirs::from(QUALIFIER, ORGANISATION, APPLICATION)
            .ok_or(Error::FailedToFindStorageLocation)?;

        let root = project_dirs
            .data_dir()
            .to_path_buf();

        let database = root.join("database.db");
        let photos = root.join("photos");

        if !root.try_exists().map_err(|_| Error::CheckFileExistsError)? {
            create_dir_all(&root)
                .map_err(|_| Error::FailedToCreateDirectory(root.clone()))?;
        }

        if !photos.try_exists().map_err(|_| Error::CheckFileExistsError)? {
            create_dir_all(&photos)
                .map_err(|_| Error::FailedToCreateDirectory(photos.clone()))?;
        }

        STORAGE.get_or_init(|| Storage { root, database, photos });
        Ok(())
    }

}
