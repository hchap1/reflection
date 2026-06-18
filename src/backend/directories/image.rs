use std::path::Path;

use image::DynamicImage;
use image::GenericImageView;
use image::ImageBuffer;
use rkyv::Archive;
use rkyv::Deserialize;
use rkyv::Serialize;

use crate::backend::directories::storage::Storage;
use crate::error::Error;
use crate::error::Res;
use crate::backend::database::sql::Photo;

/// Px * Px size of thumbnails
pub const THUMBNAIL_SIZE: u32 = 128;

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

    /// Attempt to write this image to the correct location, including thumbnail
    /// Consumes self
    pub async fn save(self, photo: Photo) -> Res<()> {
        let (path, thumbnail_path) = Storage::get_storage()?
            .get_photo_path(&photo.user_id, &photo.album_id, &photo.name)
            .await?;

        let dynamic_image = std::sync::Arc::new(tokio::task::spawn_blocking(
            move ||
            Ok::<DynamicImage, Error>(
                DynamicImage::ImageRgba8(
                    ImageBuffer::from_raw(self.width, self.height, self.data)
                        .ok_or(Error::FailedToParseImage)?
                )
            )
        ).await??);

        let dynamic_image_clone = dynamic_image.clone();

        // Create a copy of the image downsized to THUMBNAIL_SIZE
        let thumbnail_image = tokio::task::spawn_blocking(
            move ||
            dynamic_image_clone.resize(
                THUMBNAIL_SIZE,
                THUMBNAIL_SIZE,
                image::imageops::FilterType::CatmullRom
            )
        ).await?;

        tokio::task::spawn_blocking(
            move || {
                dynamic_image
                    .save(path)?;

                thumbnail_image
                    .save(thumbnail_path)?;

                Ok::<(), Error>(())
            }
        ).await??;

        Ok(())
    }

    /// Attempt to download the given photo, and then parse into a ReflectionImage
    pub async fn download(photo: Photo) -> Res<Self> {
        todo!("Implement");
    }
}
