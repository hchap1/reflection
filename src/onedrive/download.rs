use std::path::PathBuf;

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
) -> Res<PathBuf> {

    let directory = album_root_dir.join(&album_id);
    if !directory.exists() {
        tokio::fs::create_dir_all(&directory).await?;
    };

    let original_path = PathBuf::from(&photo_file.name);
    let extension = original_path.extension().ok_or(DownloadError::CouldNotParseExtension)?;

    let file_path = directory.join(&photo_file.onedrive_id).with_extension(extension);
    if file_path.exists() {
        return Ok(file_path);
    }

    let client = Client::new();
    let content_url = format!("{CONTENT_URL}{}/content", photo_file.id);
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

    Ok(file_path)
}
