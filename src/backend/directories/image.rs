use std::path::Path;
use std::time::Duration;

use iced::widget::image::Handle;
use image::DynamicImage;
use image::GenericImageView;
use image::ImageBuffer;
use onedrive_albums::api::download::get_download_handle;
use rkyv::Archive;
use rkyv::Deserialize;
use rkyv::Serialize;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::io::BufWriter;
use tokio::sync::OwnedSemaphorePermit;
use tokio_stream::StreamExt;

use crate::backend::directories::storage::Storage;
use crate::error::Error;
use crate::error::Res;
use crate::backend::database::sql::Photo;

/// Px * Px size of thumbnails
pub const THUMBNAIL_SIZE: u32 = 128;

/// Packed RGBA8 format Image compatible with ICED and RKYV
#[derive(Clone, Debug, Archive, Serialize, Deserialize)]
#[rkyv(derive(Debug))]
pub struct ReflectionImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>
}

impl ReflectionImage {

    async fn from_file(path: &Path) -> Res<Self> {
        // Decoding + RGBA conversion is CPU/IO bound and can easily take hundreds of
        // milliseconds for a full resolution photo — run it off the async runtime's
        // worker threads so it can't stall other tasks (e.g. the slideshow timer).
        let path = path.to_owned();

        tokio::task::spawn_blocking(move || {
            let raw_image = image::open(&path)?;
            let rgba = raw_image.to_rgba8();
            let (width, height) = raw_image.dimensions();

            Ok::<Self, Error>(
                Self {
                    width,
                    height,
                    data: rgba.into_raw()
                }
            )
        }).await?
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

    /// Attempt to download the given photo
    /// Then, load the file to save the thumbnail
    /// Hold a permit which belongs to a Sempaphore limiting concurrency
    pub async fn download(_permit: OwnedSemaphorePermit, access_token: String, photo: Photo) -> Res<()> {

        let download_target = Storage::get_storage()?
            .get_temporary_path(&photo.user_id, &photo.album_id, &photo.name)
            .await?;

        let file = File::create(&download_target).await?;
        let mut buf_writer = BufWriter::new(file);
        let mut byte_stream = get_download_handle(access_token, photo.id.clone())
            .await?;

        while let Some(bytes) = byte_stream.next().await {
            let bytes = bytes?;
            buf_writer.write_all(&bytes).await?;
        }

        buf_writer.flush()
            .await?;

        // Give OS time to register new file
        tokio::time::sleep(Duration::from_millis(100)).await;

        let reflection_image = ReflectionImage::from_file(&download_target)
            .await?;

        reflection_image.save(photo).await?;

        // If we get to this point, remove the temporary file
        tokio::fs::remove_file(download_target).await?;

        Ok(())
    }

    /// Convert this image into an iced image handle
    pub fn into_iced(self) -> Handle {
        Handle::from_rgba(self.width, self.height, self.data)
    }

    /// Produce a small, heavily blurred handle of this image, suitable for use as a
    /// full-screen background behind the sharp, centered foreground image.
    ///
    /// The image is downscaled before blurring so the (otherwise expensive) blur pass
    /// runs over a handful of pixels rather than the full resolution photo.
    pub fn blurred_background(&self) -> Handle {
        const MAX_DIMENSION: u32 = 64;
        const BLUR_SIGMA: f32 = 6.0;

        let buffer = match ImageBuffer::<image::Rgba<u8>, _>::from_raw(
            self.width, self.height, self.data.as_slice()
        ) {
            Some(buffer) => buffer,
            None => return Handle::from_rgba(self.width, self.height, self.data.clone())
        };

        let longest_side = self.width.max(self.height).max(1);
        let scale = MAX_DIMENSION as f32 / longest_side as f32;
        let small_width = ((self.width as f32 * scale).round() as u32).max(1);
        let small_height = ((self.height as f32 * scale).round() as u32).max(1);

        let small = image::imageops::resize(
            &buffer, small_width, small_height, image::imageops::FilterType::Triangle
        );
        let blurred = image::imageops::blur(&small, BLUR_SIGMA);

        let (width, height) = blurred.dimensions();
        Handle::from_rgba(width, height, blurred.into_raw())
    }
}
