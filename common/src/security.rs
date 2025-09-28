use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header};
use serde::{Deserialize, Serialize};

/// JWT 声明
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// 玩家ID
    sub: u64,
    /// 过期时间
    exp: usize,
}

const JWT_SECRET: &[u8] = b"my_secret_key";

/// 生成JWT token
///
/// # 参数
///
/// * `user_id` - 玩家ID
///
/// # 返回值
///
/// 返回生成的JWT token字符串
///
/// # 示例
///
/// ```
/// use common::security::genenrate_token;
///
/// let token = genenrate_token(123);
/// ```
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

/// 验证JWT token
///
/// # 参数
///
/// * `token` - 待验证的JWT token字符串
///
/// # 返回值
///
/// 返回验证通过的 `Claims` 结构体
///
/// # 示例
///
/// ```
/// use common::security::genenrate_token;
/// use common::security::validate_token;
///
/// let token = genenrate_token(123);
/// let claims = validate_token(&token).unwrap();
/// ```
pub fn validate_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    jsonwebtoken::decode::<Claims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET),
        &jsonwebtoken::Validation::default(),
    )
    .map(|data| data.claims)
}
