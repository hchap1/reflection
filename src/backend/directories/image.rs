use std::path::Path;

use image::GenericImageView;
use rkyv::Archive;
use rkyv::Deserialize;
use rkyv::Serialize;

use crate::backend::directories::storage::Storage;
use crate::error::Res;
use crate::backend::database::sql::Photo;

/// Packed RGBA8 format Image compatible with ICED and RKYV
#[derive(Clone, Debug, Archive, Serialize, Deserialize)]
pub struct ReflectionImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>
}

impl ReflectionImage {

    async fn from_file(path: &Path) -> Res<Self> {
        let raw_image = image::open(path)?;
        let rgba = raw_image.to_rgba8();
        let (width, height) = raw_image.dimensions();

        Ok(
            Self {
                width,
                height,
                data: rgba.into_raw()
            }
        )
    }
    
    pub async fn load(photo: Photo) -> Res<Self> {
        let (image_path, _) = Storage::get_storage()?
            .get_photo_path(&photo.user_id, &photo.album_id, &photo.name)
            .await?;

        Self::from_file(&image_path).await
    }

    pub async fn load_thumbnail(photo: Photo) -> Res<Self> {
        let (_, thumbnail_path) = Storage::get_storage()?
            .get_photo_path(&photo.user_id, &photo.album_id, &photo.name)
            .await?;

        Self::from_file(&thumbnail_path).await
    }
}
