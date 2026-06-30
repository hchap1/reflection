use std::collections::HashMap;
use std::sync::Arc;

use onedrive_albums::api::photos::get_photos;
use tokio::sync::Semaphore;

use crate::backend::database::authentication_storage::Authentication;
use crate::backend::database::sql::{Photo, SQL};
use crate::backend::directories::image::ReflectionImage;
use crate::backend::directories::storage::Storage;
use crate::error::{Error, Res};

/// Synchronise images for all photos
/// Download if missing
pub async fn synchronise_files(semaphore: Arc<Semaphore>) -> Res<Vec<(String, Res<()>)>> {
    println!("Synchronising files");
    
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
            issues.push((photo.id.clone(), res));
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
            issues.push((photo.id.clone(), res));
        }
    }

    // Finally, download each that didn't have both files
    let tasks: Vec<_> = to_be_downloaded
        .into_iter()
        .filter_map(
            |photo|
            if let Some(access_token) = user_access_tokens.get(&photo.user_id) {
                let access_token_owned = access_token.to_owned();
                let semaphore_clone = semaphore.clone();
                Some(
                    tokio::spawn(
                        async move {
                            let permit = semaphore_clone
                                .acquire_owned() .await?;

                            ReflectionImage::download(permit, access_token_owned, photo)
                                .await
                        }
                    )
                )
            } else { None }
        ).collect();

    for task in tasks {
        let res = task.await;
        match res {
            Ok(res) => match res {
                Ok(()) => (),
                Err(e) => issues.push((String::from("DOWNLOAD"), Err(e)))
            },
            Err(e) => issues.push((String::from("THREAD"), Err(e.into())))
        }
    }

    Ok(issues)
}

/// Synchronise photos for all albums into database
/// Pulls down every photo for every album and compares
/// Does not download image data, just metadata
pub async fn synchronise_photos() -> Res<Vec<(String, Res<()>)>> {
    println!("Synchronising photos");
    
    // First, retrieve the list of all albums in the database
    let albums = SQL::select_all_albums()
        .await?;

    // Second, record the access_token for each unique user
    let mut issues = Vec::new();
    let mut user_access_tokens = HashMap::new();

    for album in &albums {
        if user_access_tokens.contains_key(&album.user_id) {
            continue;
        }

        let res: Res<()> = async {
            let mut user = SQL::select_user_by_id(album.user_id.clone())
                .await?
                .ok_or(Error::NoSuchUserInDatabase)?;

            let token = Authentication::get_access_token(&mut user)
                .await?;

            user_access_tokens.insert(album.user_id.to_owned(), token);
            Ok(())
        }.await;

        if res.is_err() {
            issues.push((album.id.clone(), res));
        }
    }

    // Third, use the recorded access tokens to pull down
    // the list of all photos in the albums from onedrive
    // TODO remove photos that don't have a corresponding album
    for album in albums {
        if let Some(access_token) = user_access_tokens.get(&album.user_id) {
            match get_photos(album.id.clone(), access_token.to_owned()).await {
                Ok(photos) => {
                    for photo in photos {
                        match SQL::insert_or_update_photo(
                            &Photo {
                                id: photo.id,
                                album_id: album.id.clone(),
                                user_id: album.user_id.clone(),
                                name: photo.name,
                                created_date_time: photo.created_date_time.timestamp(),
                                width: photo.width as i64,
                                height: photo.height as i64,
                                latitude: photo.location.as_ref().map(|loc| loc.latitude),
                                longitude: photo.location.as_ref().map(|loc| loc.longitude),
                                altitude: photo.location.as_ref().map(|loc| loc.altitude),
                                size: Some(photo.size as i64)
                            }
                        ).await {
                            Ok(_) => (),
                            Err(e) => issues.push((String::from("PHOTO_DB_FAILED"), Err(e)))
                        }
                    }
                },
                Err(e) => issues.push((String::from("ALBUM_FAILED"), Err(e.into())))
            }

        }
    }

    Ok(issues)
}
