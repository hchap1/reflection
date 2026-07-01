use std::collections::HashMap;
use onedrive_albums::authentication::oauth2::refresh::refresh_tokenset;
use tokio::sync::OnceCell;
use tokio::sync::Mutex;

use crate::backend::database::sql::SQL;
use crate::backend::database::sql::User;
use crate::error::Error;
use crate::error::Res;

pub static AUTHENTICATION: OnceCell<Mutex<Authentication>> = OnceCell::const_new();

pub struct Authentication {
    
    /// Hash the user_id into the access token
    hash: HashMap<String, String>
}

impl Authentication {

    /// Call every refresh token in the database
    /// Overwrite old refresh tokens once new is acquired
    /// Also save the access token into the AUTHENTICATION singleton
    pub async fn create_initial() -> Res<()> {
        let users = SQL::select_all_users().await?;

        let mut hash: HashMap<String, String> = HashMap::new();

        // Update tokens & store active access token
        for mut user in users {
            let tokenset = refresh_tokenset(user.refresh_token).await?;
            hash.insert(user.id.clone(), tokenset.access_token);
            user.refresh_token = tokenset.refresh_token;
            user.expiry_date_time = tokenset.absolute_expiration as i64;
            SQL::insert_or_update_user(&user).await?;
        }

        // Build singleton
        AUTHENTICATION.set(Mutex::new(Authentication { hash }))
            .map_err(|_| Error::SingletonSetError)?;

        Ok(())
    }

    /// Check if an access token exists for the specified user
    /// If not, create one (if possible)
    pub async fn get_access_token(user: &mut User) -> Res<String> {
        
        {
            let authentication = AUTHENTICATION
                .get()
                .ok_or(Error::AuthenticationSingletonDead)?;

            let authentication = authentication
                .lock()
                .await;

            if let Some(access_token) = authentication.get(&user.id) {
                return Ok(access_token.clone());
            }
        }

        // Otherwise, we must find one
        let tokenset = refresh_tokenset(user.refresh_token.clone()).await?;
        {
            let authentication = AUTHENTICATION
                .get()
                .ok_or(Error::AuthenticationSingletonDead)?;

            let mut authentication = authentication
                .lock()
                .await;

            authentication.insert(user.id.clone(), tokenset.access_token.clone());
        }
        user.refresh_token = tokenset.refresh_token;
        user.expiry_date_time = tokenset.absolute_expiration as i64;
        SQL::insert_or_update_user(user).await?;

        // If we got to this point, return new access_token
        Ok(tokenset.access_token)
    }

    /// Internally, search hashmap
    fn get(&self, user_id: &String) -> Option<&String> {
        self.hash.get(user_id)
    }

    /// Internally, insert id/token
    fn insert(&mut self, user_id: String, access_token: String) {
        self.hash.insert(user_id, access_token);
    }
}
