use bcrypt::{hash, verify, BcryptError, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

static JWT_SECRET: OnceCell<String> = OnceCell::new();
const DEFAULT_EXPIRATION_HOURS: i64 = 24;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub exp: usize,
}

#[derive(Debug)]
pub enum SecurityError {
    JwtSecretAlreadyInitialized,
    JwtSecretMissing,
    PasswordHash(BcryptError),
    Jwt(jsonwebtoken::errors::Error),
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityError::JwtSecretAlreadyInitialized => {
                write!(f, "JWT secret has already been initialized")
            }
            SecurityError::JwtSecretMissing => write!(f, "JWT secret is not initialized"),
            SecurityError::PasswordHash(err) => write!(f, "Password hashing error: {}", err),
            SecurityError::Jwt(err) => write!(f, "JWT error: {}", err),
        }
    }
}

impl std::error::Error for SecurityError {}

impl From<BcryptError> for SecurityError {
    fn from(value: BcryptError) -> Self {
        SecurityError::PasswordHash(value)
    }
}

impl From<jsonwebtoken::errors::Error> for SecurityError {
    fn from(value: jsonwebtoken::errors::Error) -> Self {
        SecurityError::Jwt(value)
    }
}

pub fn init_jwt_secret(secret: String) -> Result<(), SecurityError> {
    JWT_SECRET
        .set(secret)
        .map_err(|_| SecurityError::JwtSecretAlreadyInitialized)
}

fn jwt_secret() -> Result<&'static str, SecurityError> {
    JWT_SECRET
        .get()
        .map(|value| value.as_str())
        .ok_or(SecurityError::JwtSecretMissing)
}

pub fn hash_password(password: &str) -> Result<String, SecurityError> {
    Ok(hash(password, DEFAULT_COST)?)
}

pub fn verify_password(password: &str, hashed: &str) -> Result<bool, SecurityError> {
    Ok(verify(password, hashed)?)
}

pub fn generate_jwt(user_id: i32, email: &str) -> Result<String, SecurityError> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(DEFAULT_EXPIRATION_HOURS))
        .expect("failed to calculate JWT expiration");

    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        exp: expiration.timestamp() as usize,
    };

    let secret = jwt_secret()?;
    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(token)
}

pub fn validate_jwt(token: &str) -> Result<Claims, SecurityError> {
    let secret = jwt_secret()?;
    let validation = Validation::new(Algorithm::HS256);
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;
    Ok(token_data.claims)
} 