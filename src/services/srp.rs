use chrono::{Duration, Utc};
use sha2::Sha256;
use sqlx::PgPool;
use srp::groups::G_2048;
use srp::server::SrpServer;
use srp::client::SrpClient;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, Result};

pub struct SrpService {
    db_pool: Arc<PgPool>,
}

pub struct SrpSessionData {
    pub session_id: Uuid,
    pub salt: String,
    pub server_public: String,
}

impl SrpService {
    pub fn new(db_pool: Arc<PgPool>) -> Self {
        Self { db_pool }
    }

    pub fn generate_salt() -> Vec<u8> {
        use rand::RngCore;
        let mut salt = vec![0u8; 16];
        rand::thread_rng().fill_bytes(&mut salt);
        salt
    }

    pub fn compute_verifier(identity: &str, password: &str, salt: &[u8]) -> Vec<u8> {
        let client = SrpClient::<Sha256>::new(&G_2048);
        client.compute_verifier(identity.as_bytes(), password.as_bytes(), salt)
    }

    pub async fn store_verifier(
        &self,
        user_id: Uuid,
        salt: &[u8],
        verifier: &[u8],
    ) -> Result<()> {
        sqlx::query("UPDATE users SET srp_salt = $1, srp_verifier = $2 WHERE id = $3")
            .bind(hex::encode(salt))
            .bind(hex::encode(verifier))
            .bind(user_id)
            .execute(self.db_pool.as_ref())
            .await?;
        Ok(())
    }

    pub async fn get_user_srp_data(&self, email: &str) -> Result<Option<(Uuid, Vec<u8>, Vec<u8>)>> {
        let row: Option<(Uuid, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT id, srp_salt, srp_verifier FROM users WHERE email = $1"
        )
        .bind(email)
        .fetch_optional(self.db_pool.as_ref())
        .await?;

        match row {
            Some((id, Some(salt), Some(verifier))) => {
                let salt = hex::decode(&salt)
                    .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid salt")))?;
                let verifier = hex::decode(&verifier)
                    .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid verifier")))?;
                Ok(Some((id, salt, verifier)))
            }
            _ => Ok(None),
        }
    }

    pub async fn init_login(
        &self,
        email: &str,
        client_public_hex: &str,
    ) -> Result<SrpSessionData> {
        let (user_id, salt, verifier) = self
            .get_user_srp_data(email)
            .await?
            .ok_or(AppError::InvalidCredentials)?;

        let client_public = hex::decode(client_public_hex)
            .map_err(|_| AppError::InvalidRequest("Invalid client public".into()))?;

        let server = SrpServer::<Sha256>::new(&G_2048);
        
        let mut b = [0u8; 64];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut b);
        
        let b_pub = server.compute_public_ephemeral(&b, &verifier);

        let session_id = self
            .store_session(user_id, &b, &client_public, &verifier)
            .await?;

        Ok(SrpSessionData {
            session_id,
            salt: hex::encode(&salt),
            server_public: hex::encode(&b_pub),
        })
    }

    async fn store_session(
        &self,
        user_id: Uuid,
        server_secret: &[u8],
        client_public: &[u8],
        verifier: &[u8],
    ) -> Result<Uuid> {
        let expires_at = Utc::now() + Duration::minutes(5);
        let data = format!(
            "{}:{}",
            hex::encode(client_public),
            hex::encode(verifier)
        );

        let (id,): (Uuid,) = sqlx::query_as(
            "INSERT INTO srp_sessions (user_id, server_ephemeral_secret, client_ephemeral_public, expires_at)
             VALUES ($1, $2, $3, $4) RETURNING id"
        )
        .bind(user_id)
        .bind(hex::encode(server_secret))
        .bind(data)
        .bind(expires_at)
        .fetch_one(self.db_pool.as_ref())
        .await?;

        Ok(id)
    }

    pub async fn verify_login(
        &self,
        session_id: Uuid,
        client_proof_hex: &str,
    ) -> Result<(Uuid, String, String)> {
        let session = self.get_session(session_id).await?;
        let email = self.get_email_by_user_id(session.user_id).await?;

        let client_proof = hex::decode(client_proof_hex)
            .map_err(|_| AppError::InvalidRequest("Invalid client proof".into()))?;

        let server = SrpServer::<Sha256>::new(&G_2048);
        
        let verifier = server
            .process_reply(&session.server_secret, &session.verifier, &session.client_public)
            .map_err(|_| AppError::InvalidCredentials)?;

        verifier
            .verify_client(&client_proof)
            .map_err(|_| AppError::InvalidCredentials)?;

        let server_proof = verifier.proof();

        self.delete_session(session_id).await?;

        Ok((session.user_id, email, hex::encode(server_proof)))
    }

    async fn get_session(&self, session_id: Uuid) -> Result<SrpSession> {
        let row: Option<(Uuid, String, String)> = sqlx::query_as(
            "SELECT user_id, server_ephemeral_secret, client_ephemeral_public 
             FROM srp_sessions WHERE id = $1 AND expires_at > NOW()"
        )
        .bind(session_id)
        .fetch_optional(self.db_pool.as_ref())
        .await?;

        match row {
            Some((user_id, server_secret_hex, data)) => {
                let server_secret = hex::decode(&server_secret_hex)
                    .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid session")))?;
                
                let parts: Vec<&str> = data.split(':').collect();
                if parts.len() != 2 {
                    return Err(AppError::Internal(anyhow::anyhow!("Invalid session data")));
                }
                
                let client_public = hex::decode(parts[0])
                    .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid session")))?;
                let verifier = hex::decode(parts[1])
                    .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid session")))?;

                Ok(SrpSession {
                    user_id,
                    server_secret,
                    client_public,
                    verifier,
                })
            }
            None => Err(AppError::InvalidToken),
        }
    }

    async fn get_email_by_user_id(&self, user_id: Uuid) -> Result<String> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT email FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_optional(self.db_pool.as_ref())
        .await?;

        row.map(|(e,)| e).ok_or(AppError::NotFound)
    }

    async fn delete_session(&self, session_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM srp_sessions WHERE id = $1")
            .bind(session_id)
            .execute(self.db_pool.as_ref())
            .await?;
        Ok(())
    }
}

struct SrpSession {
    user_id: Uuid,
    server_secret: Vec<u8>,
    client_public: Vec<u8>,
    verifier: Vec<u8>,
}