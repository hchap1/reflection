use std::io::ErrorKind;
use std::path::PathBuf;
use std::fs::{create_dir, create_dir_all};

use directories::ProjectDirs;

use crate::error::Res;

#[derive(Clone, Debug)]
pub enum DirectoryError {
    HomeDirError,
    FailedToCreateRoot,
    FailedToCreateAlbums,
    FailedToCreateDeps,
}

#[derive(Clone, Debug)]
pub struct Directories {
    pub root: PathBuf,
    pub albums: PathBuf,
    pub deps: PathBuf
}

impl Directories {
    pub fn create_or_load() -> Res<Directories> {
        let root = ProjectDirs::from("com", "hchap1", "Reflection").ok_or(DirectoryError::HomeDirError)?.data_dir().to_path_buf();

        let albums = root.join("albums");
        let deps = root.join("deps");

        create_dir_all(&root)?;
        if !root.exists() { return Err(DirectoryError::FailedToCreateRoot.into()); }

        match create_dir(&albums) {
            Ok(_) => (),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => (),
            Err(_) => return Err(DirectoryError::FailedToCreateAlbums.into())
        };

        match create_dir(&deps) {
            Ok(_) => (),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => (),
            Err(_) => return Err(DirectoryError::FailedToCreateDeps.into())
        };

        Ok(Directories { root, albums, deps })
    }
}
