/// Storage impls to persis OAuth sessions if you are not using the memory stores
/// https://github.com/bluesky-social/statusphere-example-app/blob/main/src/auth/storage.ts
use crate::db::{AuthSession, AuthState};
use async_sqlite::Pool;
use atrium_api::types::string::Did;
use atrium_common::store::Store;
use atrium_oauth::store::session::SessionStore;
use atrium_oauth::store::state::StateStore;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::hash::Hash;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SqliteStoreError {
    #[error("Invalid session")]
    InvalidSession,
    #[error("No session found")]
    NoSessionFound,
    #[error("Database error: {0}")]
    DatabaseError(async_sqlite::Error),
}

///Persistent session store in sqlite
impl SessionStore for SqliteSessionStore {}

pub struct SqliteSessionStore {
    db_pool: Pool,
}

impl SqliteSessionStore {
    pub fn new(db: Pool) -> Self {
        Self { db_pool: db }
    }
}

impl<K, V> Store<K, V> for SqliteSessionStore
where
    K: Debug + Eq + Hash + Send + Sync + 'static + From<Did> + AsRef<str>,
    V: Debug + Clone + Send + Sync + 'static + Serialize + DeserializeOwned,
{
    type Error = SqliteStoreError;
    async fn get(&self, key: &K) -> Result<Option<V>, Self::Error> {
        let did = key.as_ref().to_string();
        match AuthSession::get_by_did(&self.db_pool, did).await {
            Ok(Some(auth_session)) => {
                let deserialized_session: V = serde_json::from_str(&auth_session.session)
                    .map_err(|_| SqliteStoreError::InvalidSession)?;
                Ok(Some(deserialized_session))
            }
            Ok(None) => Err(SqliteStoreError::NoSessionFound),
            Err(db_error) => {
                log::error!("Database error: {db_error}");
                Err(SqliteStoreError::DatabaseError(db_error))
            }
        }
    }

    async fn set(&self, key: K, value: V) -> Result<(), Self::Error> {
        let did = key.as_ref().to_string();
        let auth_session = AuthSession::new(did, value);
        auth_session
            .save_or_update(&self.db_pool)
            .await
            .map_err(SqliteStoreError::DatabaseError)?;
        Ok(())
    }

    async fn del(&self, _key: &K) -> Result<(), Self::Error> {
        let did = _key.as_ref().to_string();
        AuthSession::delete_by_did(&self.db_pool, did)
            .await
            .map_err(SqliteStoreError::DatabaseError)?;
        Ok(())
    }

    async fn clear(&self) -> Result<(), Self::Error> {
        AuthSession::delete_all(&self.db_pool)
            .await
            .map_err(SqliteStoreError::DatabaseError)?;
        Ok(())
    }
}

///Persistent session state in sqlite
impl StateStore for SqliteStateStore {}

pub struct SqliteStateStore {
    db_pool: Pool,
}

impl SqliteStateStore {
    pub fn new(db: Pool) -> Self {
        Self { db_pool: db }
    }
}

impl<K, V> Store<K, V> for SqliteStateStore
where
    K: Debug + Eq + Hash + Send + Sync + 'static + From<Did> + AsRef<str>,
    V: Debug + Clone + Send + Sync + 'static + Serialize + DeserializeOwned,
{
    type Error = SqliteStoreError;
    async fn get(&self, key: &K) -> Result<Option<V>, Self::Error> {
        let key = key.as_ref().to_string();
        match AuthState::get_by_key(&self.db_pool, key).await {
            Ok(Some(auth_state)) => {
                let deserialized_state: V = serde_json::from_str(&auth_state.state)
                    .map_err(|_| SqliteStoreError::InvalidSession)?;
                Ok(Some(deserialized_state))
            }
            Ok(None) => Err(SqliteStoreError::NoSessionFound),
            Err(db_error) => {
                log::error!("Database error: {db_error}");
                Err(SqliteStoreError::DatabaseError(db_error))
            }
        }
    }

    async fn set(&self, key: K, value: V) -> Result<(), Self::Error> {
        let did = key.as_ref().to_string();
        let auth_state = AuthState::new(did, value);
        auth_state
            .save_or_update(&self.db_pool)
            .await
            .map_err(SqliteStoreError::DatabaseError)?;
        Ok(())
    }

    async fn del(&self, _key: &K) -> Result<(), Self::Error> {
        let key = _key.as_ref().to_string();
        AuthState::delete_by_key(&self.db_pool, key)
            .await
            .map_err(SqliteStoreError::DatabaseError)?;
        Ok(())
    }

    async fn clear(&self) -> Result<(), Self::Error> {
        AuthState::delete_all(&self.db_pool)
            .await
            .map_err(SqliteStoreError::DatabaseError)?;
        Ok(())
    }
}
