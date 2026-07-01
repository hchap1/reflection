use directories::ProjectDirs;

use std::{path::Path, sync::OnceLock};
use std::path::PathBuf;
use std::fs::create_dir_all;

use crate::error::{Error, Res};

const QUALIFIER: &str = "com";
const ORGANISATION: &str = "hchap1";
const APPLICATION: &str = "reflection";

pub static STORAGE: OnceLock<Storage> = OnceLock::new();

#[allow(dead_code)]
pub struct Storage {

    /// The root folder of the data directory
    root: PathBuf,

    /// The specific database file (.db)
    database: PathBuf,

    /// Root directory of photo storage
    /// Actual photos should be <ROOT>/photos/<USER_ID>/<ALBUM_ID>/<PHOTO_ID>.ext
    photos: PathBuf,

    /// A frequently cleaned directory where current downloads are stored
    temporary: PathBuf
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
        let temporary = root.join("temporary");

        if !root.try_exists().map_err(|_| Error::CheckFileExistsError)? {
            create_dir_all(&root)
                .map_err(|_| Error::FailedToCreateDirectory(root.clone()))?;
        }

        if !photos.try_exists().map_err(|_| Error::CheckFileExistsError)? {
            create_dir_all(&photos)
                .map_err(|_| Error::FailedToCreateDirectory(photos.clone()))?;
        }

        if !temporary.try_exists().map_err(|_| Error::CheckFileExistsError)? {
            create_dir_all(&temporary)
                .map_err(|_| Error::FailedToCreateDirectory(temporary.clone()))?;
        }

        println!("Using: {root:?}");
        STORAGE.get_or_init(|| Storage { root, database, photos, temporary });
        Ok(())
    }

    /// Unwrap the STORAGE singleton
    pub fn get_storage<'a>() -> Res<&'a Storage> {
        STORAGE.get().ok_or(Error::FailedToAccessStorage)
    }

    /// Getter for database path
    pub fn get_database_path(&self) -> &Path {
        &self.database
    }

    /// Helper method to construct Path for a photo
    /// Constructs the full path if it doesn't exist
    /// Returns PHOTO, THUMBNAIL
    pub async fn get_photo_path(
        &self,
        user_id: &str,
        album_id: &str,
        photo_name: &str
    ) -> Res<(PathBuf, PathBuf)> {

        // Create the containing album directory
        let album_directory = self.photos
            .join(user_id)
            .join(album_id);

        let exists = tokio::fs::try_exists(&album_directory)
            .await
            .map_err(|_| Error::CheckFileExistsError)?;

        // If the directory doesn't yet exist, create it
        if !exists {
            tokio::fs::create_dir_all(&album_directory)
                .await
                .map_err(|_| Error::FailedToCreateDirectory(album_directory.clone()))?;
        }

        Ok((album_directory.join(photo_name), album_directory.join(format!("thumbnail_{photo_name}"))))
    }

    pub async fn get_temporary_path(
        &self,
        user_id: &str,
        album_id: &str,
        photo_name: &str
    ) -> Res<PathBuf> {

        // Create the containing album directory
        let album_directory = self.temporary
            .join(user_id)
            .join(album_id);

        let exists = tokio::fs::try_exists(&album_directory)
            .await
            .map_err(|_| Error::CheckFileExistsError)?;

        // If the directory doesn't yet exist, create it
        if !exists {
            tokio::fs::create_dir_all(&album_directory)
                .await
                .map_err(|_| Error::FailedToCreateDirectory(album_directory.clone()))?;
        }

        Ok(album_directory.join(photo_name))
    }
}
