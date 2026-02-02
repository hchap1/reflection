use std::path::PathBuf;

use image::{GenericImageView, Rgba};
use reqwest::Client;
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;

use crate::onedrive::api::AccessToken;
use crate::error::Res;
use crate::onedrive::get_album_children::Photo;

const CONTENT_URL: &str = "https://graph.microsoft.com/v1.0/me/drive/items/";

#[derive(Clone, Debug)]
pub enum DownloadError {
    CouldNotParseExtension
}

pub async fn download_drive_item(
    access_token: AccessToken,
    photo_file: Photo,
    album_root_dir: PathBuf,
    album_id: String
) -> Res<(PathBuf, Option<PathBuf>)> {

    let directory = album_root_dir.join(&album_id);
    if !directory.exists() {
        tokio::fs::create_dir_all(&directory).await?;
    };

    let original_path = PathBuf::from(&photo_file.name);
    let extension = original_path.extension().ok_or(DownloadError::CouldNotParseExtension)?;

    let file_path = directory.join(&photo_file.onedrive_id).with_extension(extension);

    let thumbnail_path = match (file_path.parent(), file_path.extension(), file_path.file_prefix()) {
        (Some(parent), Some(extension), Some(name)) => {
            let name = name.to_string_lossy().to_string();
            let extension = extension.to_string_lossy().to_string();
            Some(parent.join(format!("{name}-thumbnail.{extension}")))
        }
        _ => None
    };

    if file_path.exists() {
        if let Some(thumbnail_path) = thumbnail_path {
            if thumbnail_path.exists() {
                return Ok((file_path, Some(thumbnail_path)));
            } else {
                return Ok((file_path, None))
            }
        }
        return Ok((file_path, None));
    }

    let client = Client::new();
    let content_url = format!("{CONTENT_URL}{}/content", photo_file.onedrive_id);
    let response = client
        .get(&content_url)
        .bearer_auth(access_token.get())
        .send()
        .await?
        .error_for_status()?;

    let mut file = tokio::fs::File::create(&file_path).await?;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        file.write_all(&bytes).await?;
    }

    if let Some(thumbnail_path) = thumbnail_path.clone() {
        let file_path_clone = file_path.clone();
        tokio::task::spawn_blocking(move || {
            if let Ok(img) = image::open(&file_path_clone) {
                let target_size = 128;
                let resized = img.resize(
                    target_size,
                    target_size,
                    image::imageops::FilterType::Lanczos3,
                );
                let (w, h) = resized.dimensions();
                let mut canvas = image::RgbaImage::from_pixel(
                    target_size,
                    target_size,
                    Rgba([0, 0, 0, 255])
                );

                let x_offset = (target_size - w) / 2;
                let y_offset = (target_size - h) / 2;

                image::imageops::overlay(&mut canvas, &resized, x_offset.into(), y_offset.into());

                let _ = canvas.save(&thumbnail_path);
            }
        })
        .await?;
    }

    Ok((file_path, thumbnail_path))
}
