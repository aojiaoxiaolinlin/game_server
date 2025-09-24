use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// 玩家ID
    sub: u64,
    /// 过期时间
    exp: usize,
}

const JWT_SECRET: &[u8] = b"my_secret_key";

pub fn genenrate_token(user_id: u64) -> String {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(1))
        .expect("有效时间戳")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id,
        exp: expiration,
    };
    jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET),
    )
    .expect("生成token失败")
}

pub fn validate_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    jsonwebtoken::decode::<Claims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET),
        &jsonwebtoken::Validation::default(),
    )
    .map(|data| data.claims)
}
