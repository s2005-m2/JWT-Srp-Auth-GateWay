use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::Result;
use crate::models::User;

pub struct UserService {
    pool: Arc<PgPool>,
}

impl UserService {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(self.pool.as_ref())
            .await?;
        Ok(user)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool.as_ref())
            .await?;
        Ok(user)
    }
}
