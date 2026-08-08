use std::path::Path;
use std::time::Duration;

use iced::widget::image::Handle;
use image::DynamicImage;
use image::GenericImageView;
use image::ImageBuffer;
use image::ImageDecoder;
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

/// The screen a photo is prepared for.
///
/// Photos are drawn contained within the screen, so nothing above this size can
/// ever be seen — but every extra pixel is still decoded, uploaded to the GPU and
/// sampled on each frame of a crossfade. Fitting to the screen *box* rather than
/// to a single maximum dimension matters most for portrait photos: bounding the
/// longest side of a 2296x4080 photo to 2048 yields 1153x2048, roughly three and
/// a half times more pixels than the 608x1080 the screen can actually show.
///
/// Staying at or below 2048 in both directions also keeps each photo within one
/// `iced_wgpu` atlas layer, avoiding the fragmented-allocation path.
pub const MAX_DISPLAY_WIDTH: u32 = 1920;
pub const MAX_DISPLAY_HEIGHT: u32 = 1080;

/// Largest dimension of the active-photo preview sent to control applications.
///
/// Small enough to cross the network promptly on every photo change, while still
/// being sharp in the control application's preview panel.
pub const PREVIEW_SIZE: u32 = 512;

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
        Self::from_file_fitted(path, None).await
    }

    /// Decode `path`, optionally scaling it down to fit inside a `(width,
    /// height)` box. Aspect ratio is always preserved, and an image already
    /// within the box is left untouched rather than needlessly resampled.
    async fn from_file_fitted(path: &Path, bounds: Option<(u32, u32)>) -> Res<Self> {
        // Decoding + RGBA conversion is CPU/IO bound and can easily take hundreds of
        // milliseconds for a full resolution photo — run it off the async runtime's
        // worker threads so it can't stall other tasks (e.g. the slideshow timer).
        let path = path.to_owned();

        tokio::task::spawn_blocking(move || {
            // `image::open` ignores the Exif orientation tag, so a photo the
            // camera stored rotated (as phones routinely do) would be shown on
            // its side — a landscape shot displayed as a portrait one, with the
            // blurred backdrop filling the sides. Decode through the reader so
            // the tag can be read and applied to the pixels.
            let reader = image::ImageReader::open(&path)?
                .with_guessed_format()?;

            let mut decoder = reader.into_decoder()?;

            // A missing or unreadable orientation tag is normal (most formats
            // have none) and simply means no transform is needed.
            let orientation = decoder
                .orientation()
                .unwrap_or(image::metadata::Orientation::NoTransforms);

            let mut raw_image = image::DynamicImage::from_decoder(decoder)?;
            raw_image.apply_orientation(orientation);

            // Downscale before `to_rgba8` so the full resolution RGBA buffer is
            // never materialised for images that are only going to be shrunk.
            let raw_image = match bounds {
                Some((max_width, max_height)) => {
                    let (width, height) = raw_image.dimensions();
                    if width > max_width || height > max_height {
                        // `resize` fits within the box, preserving aspect ratio
                        raw_image.resize(
                            max_width,
                            max_height,
                            image::imageops::FilterType::Triangle
                        )
                    } else {
                        raw_image
                    }
                }
                None => raw_image
            };

            let (width, height) = raw_image.dimensions();
            let rgba = raw_image.to_rgba8();

            Ok::<Self, Error>(
                Self {
                    width,
                    height,
                    data: rgba.into_raw()
                }
            )
        }).await?
    }

    /// Load a photo at a size suitable for display. See [`MAX_TEXTURE_DIMENSION`]
    /// for why full resolution images must not reach the renderer.
    ///
    /// Reads the pre-scaled display copy when one exists, which is several times
    /// cheaper than decoding the full resolution original on every photo change.
    /// The first load of a photo that has no display copy yet writes one, so
    /// photos downloaded before this existed become fast after one showing
    /// rather than needing to be fetched again.
    pub async fn load(photo: Photo) -> Res<Self> {
        let storage = Storage::get_storage()?;

        let display_path = storage
            .get_display_path(&photo.user_id, &photo.album_id, &photo.name)
            .await?;

        if tokio::fs::try_exists(&display_path).await.unwrap_or(false) {
            // Already at display size, so no resize is needed on the way in.
            match Self::from_file(&display_path).await {
                // A copy written against a larger bound is stale; rebuilding it
                // once is cheaper than over-sized uploads on every showing.
                Ok(image) if image.width <= MAX_DISPLAY_WIDTH
                    && image.height <= MAX_DISPLAY_HEIGHT => return Ok(image),
                // Stale, truncated or corrupt — fall through and rebuild it.
                _ => {
                    let _ = tokio::fs::remove_file(&display_path).await;
                }
            }
        }

        let (image_path, _) = storage
            .get_photo_path(&photo.user_id, &photo.album_id, &photo.name)
            .await?;

        let image = Self::from_file_fitted(
            &image_path,
            Some((MAX_DISPLAY_WIDTH, MAX_DISPLAY_HEIGHT))
        ).await?;

        // Write the copy for next time. A failure here only costs speed, so it
        // must not fail the load itself.
        let _ = image.clone().write_to(&display_path).await;

        Ok(image)
    }

    /// Encode this image to `path` as a standalone file.
    async fn write_to(self, path: &Path) -> Res<()> {
        let path = path.to_owned();

        tokio::task::spawn_blocking(move || {
            let buffer = ImageBuffer::<image::Rgba<u8>, _>::from_raw(
                self.width, self.height, self.data
            ).ok_or(Error::FailedToParseImage)?;

            // Drop the alpha channel: it carries nothing for a photo and JPEG
            // cannot represent it.
            DynamicImage::ImageRgba8(buffer)
                .to_rgb8()
                .save(&path)
                .map_err(Error::from)
        }).await?
    }

    /// Load a photo at preview size, for sending to control applications.
    pub async fn load_preview(photo: Photo) -> Res<Self> {
        let (image_path, _) = Storage::get_storage()?
            .get_photo_path(&photo.user_id, &photo.album_id, &photo.name)
            .await?;

        Self::from_file_fitted(&image_path, Some((PREVIEW_SIZE, PREVIEW_SIZE))).await
    }

    /// Scale an already-decoded image down so neither side exceeds
    /// `max_dimension`, reusing the decode rather than reading the file again.
    /// Returns a clone unchanged if it already fits.
    pub fn fitted(&self, max_dimension: u32) -> Self {
        if self.width <= max_dimension && self.height <= max_dimension {
            return self.clone();
        }

        let buffer = match ImageBuffer::<image::Rgba<u8>, _>::from_raw(
            self.width, self.height, self.data.as_slice()
        ) {
            Some(buffer) => buffer,
            None => return self.clone()
        };

        let longest_side = self.width.max(self.height).max(1);
        let scale = max_dimension as f32 / longest_side as f32;
        let width = ((self.width as f32 * scale).round() as u32).max(1);
        let height = ((self.height as f32 * scale).round() as u32).max(1);

        let resized = image::imageops::resize(
            &buffer, width, height, image::imageops::FilterType::Triangle
        );

        Self { width, height, data: resized.into_raw() }
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
        let storage = Storage::get_storage()?;

        let (path, thumbnail_path) = storage
            .get_photo_path(&photo.user_id, &photo.album_id, &photo.name)
            .await?;

        let display_path = storage
            .get_display_path(&photo.user_id, &photo.album_id, &photo.name)
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

        // Pre-scale a display-sized copy now, so the first time this photo comes
        // up in the slideshow it does not have to decode the full resolution
        // original. See `MAX_TEXTURE_DIMENSION`.
        let dynamic_image_for_display = dynamic_image.clone();
        let display_image = tokio::task::spawn_blocking(
            move || dynamic_image_for_display.resize(
                MAX_DISPLAY_WIDTH,
                MAX_DISPLAY_HEIGHT,
                image::imageops::FilterType::Triangle
            )
        ).await?;

        tokio::task::spawn_blocking(
            move || {
                dynamic_image
                    .save(path)?;

                thumbnail_image
                    .save(thumbnail_path)?;

                display_image
                    .to_rgb8()
                    .save(display_path)?;

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
