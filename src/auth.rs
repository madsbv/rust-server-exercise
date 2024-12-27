use std::random;

use color_eyre::Result;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use time::Duration;
use uuid::Uuid;

#[derive(Clone)]
pub struct JwtKey {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    exp: u64,
    iss: String,
    iat: u64,
    sub: String,
}

impl From<String> for JwtKey {
    fn from(secret: String) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&["Chirpy"]);
        validation.set_required_spec_claims(&["exp", "iss", "iat", "sub"]);
        JwtKey {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            validation,
        }
    }
}

impl JwtKey {
    pub fn encode_user(
        &self,
        user_id: &Uuid,
        expires_in: Duration,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let current_time = time::OffsetDateTime::now_utc();
        let exp = current_time + expires_in - time::OffsetDateTime::UNIX_EPOCH;
        let exp = exp.whole_seconds().try_into().expect("Time moves forward");
        let iat = (current_time - time::OffsetDateTime::UNIX_EPOCH)
            .whole_seconds()
            .try_into()
            .expect("Time moves forward");
        let claims = JwtClaims {
            exp,
            iss: "Chirpy".to_string(),
            iat,
            sub: user_id.to_string(),
        };
        encode(&Header::default(), &claims, &self.encoding_key)
    }

    pub fn decode(
        &self,
        token: &str,
    ) -> Result<jsonwebtoken::TokenData<JwtClaims>, jsonwebtoken::errors::Error> {
        decode::<JwtClaims>(token, &self.decoding_key, &self.validation)
    }

    pub fn decode_user(&self, token: &str) -> Result<Uuid> {
        let token_data = self.decode(token)?;
        Ok(Uuid::try_parse(&token_data.claims.sub)?)
    }
}

pub async fn make_refresh_token() -> String {
    let bits: (u128, u128) = (random::random(), random::random());
    format!("{:x}{:x}", bits.0, bits.1)
}
