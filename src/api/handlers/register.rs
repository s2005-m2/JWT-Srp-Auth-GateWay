use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::{AppError, Result};
use crate::api::AppState;

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
}

#[derive(Serialize)]
pub struct RegisterResponse {
    pub message: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>> {
    info!(email = %req.email, "Registration attempt");

    if !is_valid_email(&req.email) {
        warn!(email = %req.email, "Registration failed: invalid email format");
        return Err(AppError::InvalidEmail);
    }

    if state.user_service.find_by_email(&req.email).await?.is_some() {
        warn!(email = %req.email, "Registration failed: email already exists");
        return Err(AppError::EmailExists);
    }

    let code = generate_code();
    save_verification_code(&state, &req.email, &code, "register").await?;
    state.email_service.send_verification_code(&req.email, &code).await?;

    info!(email = %req.email, "Registration verification code sent");

    Ok(Json(RegisterResponse {
        message: "Verification code sent".to_string(),
    }))
}

fn is_valid_email(email: &str) -> bool {
    const MAX_EMAIL_LENGTH: usize = 254;
    const MAX_LOCAL_LENGTH: usize = 64;
    
    if email.len() > MAX_EMAIL_LENGTH || email.len() < 5 {
        return false;
    }
    
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    
    let (local, domain) = (parts[0], parts[1]);
    
    if local.is_empty() || local.len() > MAX_LOCAL_LENGTH {
        return false;
    }
    
    if !is_valid_local_part(local) {
        return false;
    }
    
    if !is_valid_domain(domain) {
        return false;
    }
    
    let domain_lower = domain.to_lowercase();
    const ALLOWED_DOMAINS: &[&str] = &[
        "qq.com",
        "163.com",
        "126.com",
        "yeah.net",
        "sina.com",
        "gmail.com",
        "outlook.com",
        "hotmail.com",
        "yahoo.com",
        "icloud.com",
    ];
    
    ALLOWED_DOMAINS.contains(&domain_lower.as_str())
}

fn is_valid_local_part(local: &str) -> bool {
    if local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return false;
    }
    
    local.chars().all(|c| {
        c.is_ascii_alphanumeric() || "!#$%&'*+/=?^_`{|}~.-".contains(c)
    })
}

fn is_valid_domain(domain: &str) -> bool {
    if domain.is_empty() || domain.len() > 253 {
        return false;
    }
    
    if domain.starts_with('.') || domain.ends_with('.') || domain.starts_with('-') {
        return false;
    }
    
    let labels: Vec<&str> = domain.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    
    for label in &labels {
        if label.is_empty() || label.len() > 63 {
            return false;
        }
        if label.starts_with('-') || label.ends_with('-') {
            return false;
        }
        if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return false;
        }
    }
    
    let tld = match labels.last() {
        Some(t) => t,
        None => {
            tracing::warn!(domain = %domain, "Domain validation: labels unexpectedly empty");
            return false;
        }
    };
    if tld.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    
    true
}

fn generate_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!("{:06}", rng.gen_range(0..1000000))
}

async fn save_verification_code(
    state: &AppState,
    email: &str,
    code: &str,
    code_type: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO verification_codes (email, code, code_type, expires_at) 
         VALUES ($1, $2, $3, NOW() + INTERVAL '10 minutes')"
    )
    .bind(email)
    .bind(code)
    .bind(code_type)
    .execute(state.db_pool.as_ref())
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_email_allowed_domains() {
        assert!(is_valid_email("user@gmail.com"));
        assert!(is_valid_email("test.user@outlook.com"));
        assert!(is_valid_email("user123@qq.com"));
    }

    #[test]
    fn test_invalid_email_disallowed_domains() {
        assert!(!is_valid_email("user@unknown.com"));
        assert!(!is_valid_email("user@example.org"));
    }

    #[test]
    fn test_invalid_email_no_at_symbol() {
        assert!(!is_valid_email("usergmail.com"));
    }

    #[test]
    fn test_invalid_email_multiple_at_symbols() {
        assert!(!is_valid_email("user@@gmail.com"));
        assert!(!is_valid_email("user@test@gmail.com"));
    }

    #[test]
    fn test_invalid_email_empty_local_part() {
        assert!(!is_valid_email("@gmail.com"));
    }

    #[test]
    fn test_invalid_email_dots_in_local() {
        assert!(!is_valid_email(".user@gmail.com"));
        assert!(!is_valid_email("user.@gmail.com"));
        assert!(!is_valid_email("us..er@gmail.com"));
    }

    #[test]
    fn test_invalid_email_too_short() {
        assert!(!is_valid_email("a@b"));
        assert!(!is_valid_email("ab"));
    }

    #[test]
    fn test_invalid_email_too_long() {
        let long_local = "a".repeat(65);
        let email = format!("{}@gmail.com", long_local);
        assert!(!is_valid_email(&email));
    }

    #[test]
    fn test_valid_local_part() {
        assert!(is_valid_local_part("user"));
        assert!(is_valid_local_part("user.name"));
        assert!(is_valid_local_part("user+tag"));
    }

    #[test]
    fn test_invalid_local_part_special_chars() {
        assert!(!is_valid_local_part("user name"));
        assert!(!is_valid_local_part("user<>name"));
    }

    #[test]
    fn test_valid_domain() {
        assert!(is_valid_domain("gmail.com"));
        assert!(is_valid_domain("sub.domain.com"));
    }

    #[test]
    fn test_invalid_domain_no_tld() {
        assert!(!is_valid_domain("localhost"));
    }

    #[test]
    fn test_invalid_domain_numeric_tld() {
        assert!(!is_valid_domain("test.123"));
    }

    #[test]
    fn test_invalid_domain_starts_with_hyphen() {
        assert!(!is_valid_domain("-invalid.com"));
    }
}
