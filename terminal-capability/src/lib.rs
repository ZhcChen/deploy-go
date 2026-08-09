use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, Signer as _, SigningKey, Verifier as _, VerifyingKey};
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u8 = 1;
pub const DEFAULT_TTL_SECONDS: i64 = 15;
pub const MAX_TTL_SECONDS: i64 = 30;
pub const CLOCK_SKEW_SECONDS: i64 = 5;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Claims {
    pub schema_version: u8,
    pub capability_id: String,
    pub node_id: String,
    pub agent_id: String,
    pub session_id: String,
    pub connection_generation: i64,
    pub issued_at: i64,
    pub expires_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpectedBinding<'a> {
    pub node_id: &'a str,
    pub agent_id: &'a str,
    pub session_id: &'a str,
    pub connection_generation: i64,
}

#[derive(Clone)]
pub struct CapabilitySigner(SigningKey);

#[derive(Clone)]
pub struct CapabilityVerifier(VerifyingKey);

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CapabilityError {
    #[error("invalid terminal capability key")]
    InvalidKey,
    #[error("invalid terminal capability format")]
    InvalidFormat,
    #[error("invalid terminal capability signature")]
    InvalidSignature,
    #[error("invalid terminal capability claims")]
    InvalidClaims,
    #[error("terminal capability binding mismatch")]
    BindingMismatch,
    #[error("terminal capability is not currently valid")]
    InvalidTime,
}

impl CapabilitySigner {
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self(SigningKey::from_bytes(&seed))
    }

    pub fn sign(&self, claims: &Claims) -> Result<String, CapabilityError> {
        validate_claim_shape(claims)?;
        let payload = serde_json::to_vec(claims).map_err(|_| CapabilityError::InvalidClaims)?;
        let signature = self.0.sign(&payload);
        Ok(format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(payload),
            URL_SAFE_NO_PAD.encode(signature.to_bytes())
        ))
    }

    pub fn public_key_base64(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.0.verifying_key().to_bytes())
    }

    pub fn verifier(&self) -> CapabilityVerifier {
        CapabilityVerifier(self.0.verifying_key())
    }
}

impl CapabilityVerifier {
    pub fn from_base64(value: &str) -> Result<Self, CapabilityError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(value)
            .map_err(|_| CapabilityError::InvalidKey)?;
        let bytes: [u8; 32] = bytes.try_into().map_err(|_| CapabilityError::InvalidKey)?;
        let key = VerifyingKey::from_bytes(&bytes).map_err(|_| CapabilityError::InvalidKey)?;
        Ok(Self(key))
    }

    pub fn verify(
        &self,
        token: &str,
        expected: &ExpectedBinding<'_>,
        now: i64,
    ) -> Result<Claims, CapabilityError> {
        let (payload, signature) = token
            .split_once('.')
            .filter(|(_, signature)| !signature.contains('.'))
            .ok_or(CapabilityError::InvalidFormat)?;
        let payload = URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|_| CapabilityError::InvalidFormat)?;
        let signature = URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(|_| CapabilityError::InvalidFormat)?;
        let signature =
            Signature::from_slice(&signature).map_err(|_| CapabilityError::InvalidFormat)?;
        self.0
            .verify(&payload, &signature)
            .map_err(|_| CapabilityError::InvalidSignature)?;
        let claims: Claims =
            serde_json::from_slice(&payload).map_err(|_| CapabilityError::InvalidClaims)?;
        validate_claim_shape(&claims)?;
        if claims.node_id != expected.node_id
            || claims.agent_id != expected.agent_id
            || claims.session_id != expected.session_id
            || claims.connection_generation != expected.connection_generation
        {
            return Err(CapabilityError::BindingMismatch);
        }
        if claims.issued_at > now.saturating_add(CLOCK_SKEW_SECONDS)
            || claims.expires_at <= now
            || claims.expires_at.saturating_sub(claims.issued_at) > MAX_TTL_SECONDS
        {
            return Err(CapabilityError::InvalidTime);
        }
        Ok(claims)
    }
}

fn validate_claim_shape(claims: &Claims) -> Result<(), CapabilityError> {
    if claims.schema_version != SCHEMA_VERSION
        || !valid_id(&claims.capability_id, "cap_")
        || !valid_id(&claims.node_id, "node_")
        || !valid_id(&claims.agent_id, "agent_")
        || !valid_id(&claims.session_id, "term_")
        || claims.connection_generation <= 0
        || claims.expires_at <= claims.issued_at
    {
        return Err(CapabilityError::InvalidClaims);
    }
    Ok(())
}

fn valid_id(value: &str, prefix: &str) -> bool {
    value.starts_with(prefix)
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims() -> Claims {
        Claims {
            schema_version: SCHEMA_VERSION,
            capability_id: "cap_01TEST".into(),
            node_id: "node_01TEST".into(),
            agent_id: "agent_01TEST".into(),
            session_id: "term_01TEST".into(),
            connection_generation: 7,
            issued_at: 100,
            expires_at: 115,
        }
    }

    fn binding() -> ExpectedBinding<'static> {
        ExpectedBinding {
            node_id: "node_01TEST",
            agent_id: "agent_01TEST",
            session_id: "term_01TEST",
            connection_generation: 7,
        }
    }

    #[test]
    fn signs_and_verifies_bound_short_lived_capability() {
        let signer = CapabilitySigner::from_seed([7; 32]);
        let verifier = CapabilityVerifier::from_base64(&signer.public_key_base64()).unwrap();
        let token = signer.sign(&claims()).unwrap();
        assert_eq!(verifier.verify(&token, &binding(), 110).unwrap(), claims());
    }

    #[test]
    fn rejects_tampering_wrong_binding_and_invalid_time() {
        let signer = CapabilitySigner::from_seed([7; 32]);
        let verifier = signer.verifier();
        let token = signer.sign(&claims()).unwrap();
        let mut tampered = token.into_bytes();
        tampered[5] = if tampered[5] == b'A' { b'B' } else { b'A' };
        assert!(matches!(
            verifier.verify(std::str::from_utf8(&tampered).unwrap(), &binding(), 110),
            Err(CapabilityError::InvalidSignature | CapabilityError::InvalidClaims)
        ));

        let token = signer.sign(&claims()).unwrap();
        let wrong = ExpectedBinding {
            node_id: "node_OTHER",
            ..binding()
        };
        assert_eq!(
            verifier.verify(&token, &wrong, 110),
            Err(CapabilityError::BindingMismatch)
        );
        assert_eq!(
            verifier.verify(&token, &binding(), 116),
            Err(CapabilityError::InvalidTime)
        );
        assert_eq!(
            verifier.verify(&token, &binding(), 115),
            Err(CapabilityError::InvalidTime)
        );
        assert_eq!(
            verifier.verify(&token, &binding(), 94),
            Err(CapabilityError::InvalidTime)
        );
    }
}
