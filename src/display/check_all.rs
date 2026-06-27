use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Semaphore;

use crate::backend::database::sql::Photo;
use crate::backend::database::authentication_storage::Authentication;
use crate::backend::database::sql::SQL;
use crate::backend::directories::image::ReflectionImage;
use crate::backend::directories::storage::Storage;
use crate::error::{Error, Res};

/// Synchronise images for all photos
/// Download if missing
pub async fn synchronise_files(semaphore: Arc<Semaphore>) -> Res<Vec<(Photo, Res<()>)>> {
    
    // First, collect all photos in the database
    let photos = SQL::select_all_photos()
        .await?;

    let mut issues = Vec::new();

    // Second, check the download status of each
    let mut to_be_downloaded = Vec::new();
    for photo in photos {
        let res = async {
            let (photo_path, thumbnail_path) = Storage::get_storage()?
                .get_photo_path(
                    &photo.user_id,
                    &photo.album_id,
                    &photo.name
                ).await?;

            let exists = tokio::fs::try_exists(photo_path).await?
                && tokio::fs::try_exists(thumbnail_path).await?;

            if !exists {
                to_be_downloaded.push(photo.clone());
            }

            Ok(())
        }.await;

        if res.is_err() {
            issues.push((photo.clone(), res));
        }
    }

    // Third, record the access_token for each unique user
    let mut user_access_tokens = HashMap::new();

    for photo in &to_be_downloaded {
        if user_access_tokens.contains_key(&photo.user_id) {
            continue;
        }

        let res: Res<()> = async {
            let mut user = SQL::select_user_by_id(photo.user_id.clone())
                .await?
                .ok_or(Error::NoSuchUserInDatabase)?;

            let token = Authentication::get_access_token(&mut user)
                .await?;

            user_access_tokens.insert(photo.user_id.to_owned(), token);
            Ok(())
        }.await;

        if res.is_err() {
            issues.push((photo.clone(), res));
        }
    }

    // Finally, download each that didn't have both files
    for photo in to_be_downloaded {
        let permit = semaphore.clone().acquire_owned().await?;
        if let Some(access_token) = user_access_tokens.get(&photo.user_id) {
            let res = ReflectionImage::download(permit, access_token.to_owned(), photo.clone())
                .await;

            if res.is_err() {
                issues.push((photo, res));
            }
        }
    }

    Ok(issues)
}

// Synchronise photos for all albums
