use chrono::{Duration, Utc};
use num_bigint::BigUint;
use sha2::{Sha256, Digest};
use sqlx::PgPool;
use srp::groups::G_2048;
use srp::server::SrpServer;
use srp::client::SrpClient;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, Result};

fn compute_k() -> BigUint {
    let n = &G_2048.n;
    let g = &G_2048.g;
    let n_bytes = n.to_bytes_be();
    let g_bytes = g.to_bytes_be();
    
    let mut hasher = Sha256::new();
    hasher.update(&n_bytes);
    hasher.update(&g_bytes);
    BigUint::from_bytes_be(&hasher.finalize())
}

fn compute_u(a_pub: &[u8], b_pub: &[u8]) -> BigUint {
    let mut hasher = Sha256::new();
    hasher.update(a_pub);
    hasher.update(b_pub);
    BigUint::from_bytes_be(&hasher.finalize())
}

fn compute_m1(identity: &str, salt: &[u8], a_pub: &[u8], b_pub: &[u8], k: &[u8]) -> Vec<u8> {
    let n_bytes = G_2048.n.to_bytes_be();
    let g_bytes = G_2048.g.to_bytes_be();
    
    let h_n: Vec<u8> = Sha256::digest(&n_bytes).to_vec();
    let h_g: Vec<u8> = Sha256::digest(&g_bytes).to_vec();
    
    let h_n_xor_h_g: Vec<u8> = h_n.iter().zip(h_g.iter()).map(|(a, b)| a ^ b).collect();
    let h_i: Vec<u8> = Sha256::digest(identity.as_bytes()).to_vec();
    
    let mut hasher = Sha256::new();
    hasher.update(&h_n_xor_h_g);
    hasher.update(&h_i);
    hasher.update(salt);
    hasher.update(a_pub);
    hasher.update(b_pub);
    hasher.update(k);
    hasher.finalize().to_vec()
}

fn compute_m2(a_pub: &[u8], m1: &[u8], k: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(a_pub);
    hasher.update(m1);
    hasher.update(k);
    hasher.finalize().to_vec()
}

fn compute_b_pub(b: &[u8], verifier: &[u8]) -> Vec<u8> {
    let n = &G_2048.n;
    let g = &G_2048.g;
    let k = compute_k();
    let v = BigUint::from_bytes_be(verifier);
    let b_int = BigUint::from_bytes_be(b);
    let b_pub = (&k * &v + g.modpow(&b_int, n)) % n;
    b_pub.to_bytes_be()
}

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

        let mut b = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut b);
        
        let b_pub = compute_b_pub(&b, &verifier);

        let session_id = self
            .store_session(user_id, email, &salt, &b, &client_public, &verifier)
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
        email: &str,
        salt: &[u8],
        server_secret: &[u8],
        client_public: &[u8],
        verifier: &[u8],
    ) -> Result<Uuid> {
        let expires_at = Utc::now() + Duration::minutes(5);

        let (id,): (Uuid,) = sqlx::query_as(
            "INSERT INTO srp_sessions (user_id, email, salt, server_ephemeral_secret, client_ephemeral_public, verifier_cache, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id"
        )
        .bind(user_id)
        .bind(email)
        .bind(hex::encode(salt))
        .bind(hex::encode(server_secret))
        .bind(hex::encode(client_public))
        .bind(hex::encode(verifier))
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

        let client_proof = hex::decode(client_proof_hex)
            .map_err(|_| AppError::InvalidRequest("Invalid client proof".into()))?;

        let b_pub = compute_b_pub(&session.server_secret, &session.verifier);
        
        let u = compute_u(&session.client_public, &b_pub);
        let _k = compute_k();
        
        let a_pub = BigUint::from_bytes_be(&session.client_public);
        let v = BigUint::from_bytes_be(&session.verifier);
        let b = BigUint::from_bytes_be(&session.server_secret);
        let n = &G_2048.n;
        
        let s = (a_pub * v.modpow(&u, n)).modpow(&b, n);
        let session_key: Vec<u8> = Sha256::digest(&s.to_bytes_be()).to_vec();
        
        let expected_m1 = compute_m1(
            &session.email,
            &session.salt,
            &session.client_public,
            &b_pub,
            &session_key,
        );

        if client_proof != expected_m1 {
            return Err(AppError::InvalidCredentials);
        }

        let server_proof = compute_m2(&session.client_public, &expected_m1, &session_key);

        self.delete_session(session_id).await?;

        Ok((session.user_id, session.email, hex::encode(server_proof)))
    }

    async fn get_session(&self, session_id: Uuid) -> Result<SrpSession> {
        let row: Option<(Uuid, String, String, String, String, String)> = sqlx::query_as(
            "SELECT user_id, email, salt, server_ephemeral_secret, client_ephemeral_public, verifier_cache
             FROM srp_sessions WHERE id = $1 AND expires_at > NOW()"
        )
        .bind(session_id)
        .fetch_optional(self.db_pool.as_ref())
        .await?;

        match row {
            Some((user_id, email, salt_hex, server_secret_hex, client_public_hex, verifier_hex)) => {
                let salt = hex::decode(&salt_hex)
                    .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid session")))?;
                let server_secret = hex::decode(&server_secret_hex)
                    .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid session")))?;
                let client_public = hex::decode(&client_public_hex)
                    .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid session")))?;
                let verifier = hex::decode(&verifier_hex)
                    .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid session")))?;

                Ok(SrpSession {
                    user_id,
                    email,
                    salt,
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
    email: String,
    salt: Vec<u8>,
    server_secret: Vec<u8>,
    client_public: Vec<u8>,
    verifier: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_m1_matches_js() {
        let email = "test@example.com";
        let salt = hex::decode("c71831de4be151915261ae1a24127846ce0117c58d05c3b792424cabce69c052").unwrap();
        let a_pub = hex::decode("5029d310534ae41ca45b840f3e742879e999ce3aa34216063a1a30978d7ea4cbc8cd73287d065837a168b945754a7d9ef7f0b05abbe530b327e2d4e6006ead9fdfa71f91484272e53ef926422c19fb84dc1f8c2f484da029612f36f2ee8b296b9b86d46ca153d14c8ca46e515d365539f8d62a2fead86efb20e8cb0b12a68028968e90452ba3942f0d08f435741aa8a46a158663dc2e7719b614164c862511d9d15a51bafbd363f6dcf20083c16fddf40d3a6fffade10f566138ac63f8f8735d967ac7218a83c4fc1d5a696df8fe43a832cc95eed53d7d2e69583178a6d1df23830d1316d6281a8b5cb9f9cbc2a5e820e39525ffb4c6ebd227a53ce5f5abdc30").unwrap();
        let b_pub = hex::decode("132e13eba3b32d5eae53a78149ac22a7d20924e8800af68c95f1f1a104064f96047e8659ea6d25fd9217bd41331042ec080844b6af08d5c85c8cf67b1e2d5523368fab95b3cad74606a4938ad5d89ca5c179f92145ccebb27e3ed328e3d7fc2a8f7d996be59e77df8b06c27a0428d6854d657c0f0aa29c6352e56b31da669b03d43e53c187a84ca9ae52a2001121d7e5f925c731212bcbd97335242828d50e9007c4e91c87b6dfbe14a0006558230ef54379f3d6281f0676940e2359230de4e87f7a850459318990ada910dc1aa4821dde4dedc19b5fc408f233998b3d923463f90e9638f28e75c7e7e0258fc778a4446bff314c8e6cd1dba8351735c8ab81e6").unwrap();
        let session_key = hex::decode("ba22fca411d0b150fd7fe84b8981512c05251df092f97a468380eb1796c69f06").unwrap();
        
        let expected_m1 = "d8e05194652688047c0acd1785fb793d2c8eca81dbd7de7aede00a4f78741ae6";
        
        let m1 = compute_m1(email, &salt, &a_pub, &b_pub, &session_key);
        let m1_hex = hex::encode(&m1);
        
        println!("Expected M1: {}", expected_m1);
        println!("Computed M1: {}", m1_hex);
        
        assert_eq!(m1_hex, expected_m1);
    }

    #[test]
    fn test_k_matches_js() {
        let n = &G_2048.n;
        let g = &G_2048.g;
        let n_bytes = n.to_bytes_be();
        let g_bytes = g.to_bytes_be();
        
        println!("N bytes len: {}", n_bytes.len());
        println!("g bytes len: {}", g_bytes.len());
        println!("N hex: {}", hex::encode(&n_bytes));
        println!("g hex: {}", hex::encode(&g_bytes));
        
        let k = compute_k();
        let k_hex = hex::encode(&k.to_bytes_be());
        let expected_k = "4cba3fb2923e01fb263ddbbb185a01c131c638f2561942e437727e02ca3c266d";
        println!("Expected k: {}", expected_k);
        println!("Computed k: {}", k_hex);
        assert_eq!(k_hex, expected_k);
    }

    #[test]
    fn test_session_key_calculation() {
        let verifier = hex::decode("86f7b7624769fdc576a7cce7186c5ac17b0f69818c621af7fc8baaa8b7db0587c77b2e350d7c0a4dd1052058b822089bec4d8b32923ca01de881d2d2e25b49d2cef9e1a52cf313a6c361b90dc1a35360cb1ccf97bd77053ff2bfd4f4531bbfb58f06c8600fcfec3da6237350619de894666283faf5f449e5cf38b699e33726c9ce7eb6702cb06a8a08a0ba6c48b0e3cc627b5d2c2faf5e33d77024b4fc93b1001aa819ba4ff1c83aea110cb7a764b0cac25bd1a4a75c3ccf21df43048f076089682ce08ce8ec8918b34dd719098b7bf2ac5fdff4097c3cbbf91ba12d0dc189c4ccae0947b2656d9f74a72c3a3d486f9f6e8db3ff999be43bbac6c5f5a3cd4eda").unwrap();
        let a_pub = hex::decode("5029d310534ae41ca45b840f3e742879e999ce3aa34216063a1a30978d7ea4cbc8cd73287d065837a168b945754a7d9ef7f0b05abbe530b327e2d4e6006ead9fdfa71f91484272e53ef926422c19fb84dc1f8c2f484da029612f36f2ee8b296b9b86d46ca153d14c8ca46e515d365539f8d62a2fead86efb20e8cb0b12a68028968e90452ba3942f0d08f435741aa8a46a158663dc2e7719b614164c862511d9d15a51bafbd363f6dcf20083c16fddf40d3a6fffade10f566138ac63f8f8735d967ac7218a83c4fc1d5a696df8fe43a832cc95eed53d7d2e69583178a6d1df23830d1316d6281a8b5cb9f9cbc2a5e820e39525ffb4c6ebd227a53ce5f5abdc30").unwrap();
        let b_secret = hex::decode("b3b1e7b5e15258e7b0422bc7ebe1ac944ad36b1b0df49b4898e8145aaea64391").unwrap();
        let b_pub_expected = hex::decode("132e13eba3b32d5eae53a78149ac22a7d20924e8800af68c95f1f1a104064f96047e8659ea6d25fd9217bd41331042ec080844b6af08d5c85c8cf67b1e2d5523368fab95b3cad74606a4938ad5d89ca5c179f92145ccebb27e3ed328e3d7fc2a8f7d996be59e77df8b06c27a0428d6854d657c0f0aa29c6352e56b31da669b03d43e53c187a84ca9ae52a2001121d7e5f925c731212bcbd97335242828d50e9007c4e91c87b6dfbe14a0006558230ef54379f3d6281f0676940e2359230de4e87f7a850459318990ada910dc1aa4821dde4dedc19b5fc408f233998b3d923463f90e9638f28e75c7e7e0258fc778a4446bff314c8e6cd1dba8351735c8ab81e6").unwrap();
        let expected_session_key = "ba22fca411d0b150fd7fe84b8981512c05251df092f97a468380eb1796c69f06";

        let b_pub_calc = compute_b_pub(&b_secret, &verifier);
        println!("B expected: {}", hex::encode(&b_pub_expected));
        println!("B computed: {}", hex::encode(&b_pub_calc));
        assert_eq!(b_pub_calc, b_pub_expected, "B public mismatch");

        let n = &G_2048.n;
        let a = BigUint::from_bytes_be(&a_pub);
        let v = BigUint::from_bytes_be(&verifier);
        let b = BigUint::from_bytes_be(&b_secret);
        let u = compute_u(&a_pub, &b_pub_expected);
        
        let s = (&a * v.modpow(&u, n)).modpow(&b, n);
        let session_key: Vec<u8> = Sha256::digest(&s.to_bytes_be()).to_vec();
        
        println!("Expected K: {}", expected_session_key);
        println!("Computed K: {}", hex::encode(&session_key));
        
        assert_eq!(hex::encode(&session_key), expected_session_key);
    }
}