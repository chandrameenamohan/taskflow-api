use std::sync::Arc;

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand_core::OsRng;
use axum::extract::State;
use axum::Json;
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{AuthResponse, LoginRequest, RefreshRequest, RegisterRequest};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub jwt_secret: Arc<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    email: String,
    exp: usize,
    iat: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct RefreshClaims {
    sub: String,
    jti: String,
    exp: usize,
    iat: usize,
}

fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Password hashing failed: {e}")))
}

fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Invalid password hash: {e}")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

fn create_access_token(user_id: &str, email: &str, secret: &str) -> Result<String, AppError> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        iat: now.timestamp() as usize,
        exp: (now + Duration::minutes(15)).timestamp() as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Token creation failed: {e}")))
}

fn create_refresh_token(user_id: &str, jti: &str, secret: &str) -> Result<String, AppError> {
    let now = Utc::now();
    let claims = RefreshClaims {
        sub: user_id.to_string(),
        jti: jti.to_string(),
        iat: now.timestamp() as usize,
        exp: (now + Duration::days(7)).timestamp() as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Refresh token creation failed: {e}")))
}

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    if body.email.is_empty() {
        return Err(AppError::ValidationError("email".to_string()));
    }
    if body.password.is_empty() {
        return Err(AppError::ValidationError("password".to_string()));
    }

    // Check if user exists
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT id FROM users WHERE email = ?")
            .bind(&body.email)
            .fetch_optional(&state.pool)
            .await?;

    if existing.is_some() {
        return Err(AppError::UserAlreadyExists);
    }

    let user_id = Uuid::new_v4().to_string();
    let password_hash = hash_password(&body.password)?;

    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES (?, ?, ?)")
        .bind(&user_id)
        .bind(&body.email)
        .bind(&password_hash)
        .execute(&state.pool)
        .await?;

    let access_token = create_access_token(&user_id, &body.email, &state.jwt_secret)?;

    let refresh_id = Uuid::new_v4().to_string();
    let refresh_token = create_refresh_token(&user_id, &refresh_id, &state.jwt_secret)?;

    // Store refresh token hash
    let token_hash = format!("{:x}", md5_hash(&refresh_token));
    let expires_at = (Utc::now() + Duration::days(7)).to_rfc3339();

    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&refresh_id)
    .bind(&user_id)
    .bind(&token_hash)
    .bind(&expires_at)
    .execute(&state.pool)
    .await?;

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    if body.email.is_empty() {
        return Err(AppError::ValidationError("email".to_string()));
    }
    if body.password.is_empty() {
        return Err(AppError::ValidationError("password".to_string()));
    }

    let user: Option<(String, String, String)> =
        sqlx::query_as("SELECT id, email, password_hash FROM users WHERE email = ?")
            .bind(&body.email)
            .fetch_optional(&state.pool)
            .await?;

    let (user_id, email, password_hash) = user.ok_or(AppError::InvalidCredentials)?;

    if !verify_password(&body.password, &password_hash)? {
        return Err(AppError::InvalidCredentials);
    }

    let access_token = create_access_token(&user_id, &email, &state.jwt_secret)?;

    let refresh_id = Uuid::new_v4().to_string();
    let refresh_token = create_refresh_token(&user_id, &refresh_id, &state.jwt_secret)?;

    let token_hash = format!("{:x}", md5_hash(&refresh_token));
    let expires_at = (Utc::now() + Duration::days(7)).to_rfc3339();

    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&refresh_id)
    .bind(&user_id)
    .bind(&token_hash)
    .bind(&expires_at)
    .execute(&state.pool)
    .await?;

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
    }))
}

pub async fn refresh_token(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    if body.refresh_token.is_empty() {
        return Err(AppError::ValidationError("refresh_token".to_string()));
    }

    // Decode the refresh token
    let token_data = decode::<RefreshClaims>(
        &body.refresh_token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| AppError::InvalidToken)?;

    let claims = token_data.claims;

    // Verify token exists in DB and is not revoked
    let stored: Option<(String, String, i32)> = sqlx::query_as(
        "SELECT id, user_id, revoked FROM refresh_tokens WHERE id = ?",
    )
    .bind(&claims.jti)
    .fetch_optional(&state.pool)
    .await?;

    let (token_id, user_id, revoked) = stored.ok_or(AppError::InvalidToken)?;

    if revoked != 0 {
        return Err(AppError::InvalidToken);
    }

    // Revoke the old refresh token (rotation)
    sqlx::query("UPDATE refresh_tokens SET revoked = 1 WHERE id = ?")
        .bind(&token_id)
        .execute(&state.pool)
        .await?;

    // Get user email for new access token
    let (email,): (String,) = sqlx::query_as("SELECT email FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(&state.pool)
        .await?;

    let access_token = create_access_token(&user_id, &email, &state.jwt_secret)?;

    let new_refresh_id = Uuid::new_v4().to_string();
    let new_refresh_token =
        create_refresh_token(&user_id, &new_refresh_id, &state.jwt_secret)?;

    let token_hash = format!("{:x}", md5_hash(&new_refresh_token));
    let expires_at = (Utc::now() + Duration::days(7)).to_rfc3339();

    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&new_refresh_id)
    .bind(&user_id)
    .bind(&token_hash)
    .bind(&expires_at)
    .execute(&state.pool)
    .await?;

    Ok(Json(AuthResponse {
        access_token,
        refresh_token: new_refresh_token,
        token_type: "Bearer".to_string(),
    }))
}

/// Simple hash for storing refresh token references (not for security — the
/// token itself is a signed JWT; this is just for lookup/comparison).
fn md5_hash(input: &str) -> u128 {
    let mut hash: u128 = 0;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u128);
    }
    hash
}
