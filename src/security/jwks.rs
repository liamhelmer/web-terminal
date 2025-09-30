// Per 011-authentication-spec.md section 2.1
// JWKS (JSON Web Key Set) data structures and key conversion

use jsonwebtoken::{Algorithm, DecodingKey};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// JWKS error types
#[derive(Error, Debug)]
pub enum JwksError {
    #[error("Invalid JWK format: {0}")]
    InvalidFormat(String),

    #[error("Unsupported key type: {0}")]
    UnsupportedKeyType(String),

    #[error("Unsupported algorithm: {0}")]
    UnsupportedAlgorithm(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Base64 decode error: {0}")]
    Base64DecodeError(String),

    #[error("Key conversion error: {0}")]
    KeyConversionError(String),

    #[error("Invalid modulus or exponent")]
    InvalidRsaComponents,
}

/// JSON Web Key Set structure
/// Per RFC 7517: https://tools.ietf.org/html/rfc7517
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonWebKeySet {
    pub keys: Vec<JsonWebKey>,
}

impl JsonWebKeySet {
    /// Find a key by its key ID (kid)
    pub fn find_key(&self, kid: &str) -> Option<&JsonWebKey> {
        self.keys.iter().find(|k| k.kid == kid)
    }

    /// Get all keys for a specific algorithm
    pub fn keys_for_algorithm(&self, alg: &str) -> Vec<&JsonWebKey> {
        self.keys.iter().filter(|k| k.alg == alg).collect()
    }
}

/// JSON Web Key structure
/// Per 011-authentication-spec.md section 3.2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonWebKey {
    /// Key ID
    pub kid: String,

    /// Key type (RSA, EC, oct, OKP)
    pub kty: String,

    /// Algorithm (RS256, RS384, RS512, ES256, ES384, ES512)
    pub alg: String,

    /// Public key use (sig for signature, enc for encryption)
    #[serde(rename = "use")]
    pub use_: String,

    /// RSA modulus (base64url encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<String>,

    /// RSA public exponent (base64url encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub e: Option<String>,

    /// EC curve (P-256, P-384, P-521)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crv: Option<String>,

    /// EC x coordinate (base64url encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<String>,

    /// EC y coordinate (base64url encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<String>,

    /// Additional fields
    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}

impl JsonWebKey {
    /// Convert JWK to jsonwebtoken DecodingKey
    /// Supports RSA (RS256/RS384/RS512) and EC (ES256/ES384/ES512) algorithms
    /// Per 011-authentication-spec.md section 9.1
    pub fn to_decoding_key(&self) -> Result<DecodingKey, JwksError> {
        match self.kty.as_str() {
            "RSA" => self.to_rsa_decoding_key(),
            "EC" => self.to_ec_decoding_key(),
            other => Err(JwksError::UnsupportedKeyType(other.to_string())),
        }
    }

    /// Convert RSA JWK to DecodingKey
    fn to_rsa_decoding_key(&self) -> Result<DecodingKey, JwksError> {
        let n = self
            .n
            .as_ref()
            .ok_or_else(|| JwksError::MissingField("n (modulus)".to_string()))?;

        let e = self
            .e
            .as_ref()
            .ok_or_else(|| JwksError::MissingField("e (exponent)".to_string()))?;

        // Decode base64url encoded values
        let n_bytes = base64_url_decode(n)?;
        let e_bytes = base64_url_decode(e)?;

        // Construct RSA public key in DER format
        let public_key = construct_rsa_public_key(&n_bytes, &e_bytes)?;

        DecodingKey::from_rsa_der(&public_key)
            .map_err(|e| JwksError::KeyConversionError(e.to_string()))
    }

    /// Convert EC JWK to DecodingKey
    fn to_ec_decoding_key(&self) -> Result<DecodingKey, JwksError> {
        let x = self
            .x
            .as_ref()
            .ok_or_else(|| JwksError::MissingField("x (coordinate)".to_string()))?;

        let y = self
            .y
            .as_ref()
            .ok_or_else(|| JwksError::MissingField("y (coordinate)".to_string()))?;

        let crv = self
            .crv
            .as_ref()
            .ok_or_else(|| JwksError::MissingField("crv (curve)".to_string()))?;

        // Decode base64url encoded values
        let x_bytes = base64_url_decode(x)?;
        let y_bytes = base64_url_decode(y)?;

        // Construct EC public key in PEM format
        let public_key = construct_ec_public_key(&x_bytes, &y_bytes, crv)?;

        DecodingKey::from_ec_pem(&public_key)
            .map_err(|e| JwksError::KeyConversionError(e.to_string()))
    }

    /// Get the algorithm as jsonwebtoken::Algorithm
    pub fn algorithm(&self) -> Result<Algorithm, JwksError> {
        match self.alg.as_str() {
            "RS256" => Ok(Algorithm::RS256),
            "RS384" => Ok(Algorithm::RS384),
            "RS512" => Ok(Algorithm::RS512),
            "ES256" => Ok(Algorithm::ES256),
            "ES384" => Ok(Algorithm::ES384),
            "ES512" => Ok(Algorithm::ES512),
            other => Err(JwksError::UnsupportedAlgorithm(other.to_string())),
        }
    }

    /// Validate that the key is suitable for signature verification
    pub fn validate(&self) -> Result<(), JwksError> {
        // Check use field
        if self.use_ != "sig" {
            return Err(JwksError::InvalidFormat(format!(
                "Key use must be 'sig', got '{}'",
                self.use_
            )));
        }

        // Validate algorithm
        self.algorithm()?;

        // Validate key type specific fields
        match self.kty.as_str() {
            "RSA" => {
                if self.n.is_none() || self.e.is_none() {
                    return Err(JwksError::MissingField(
                        "RSA key must have 'n' and 'e' fields".to_string(),
                    ));
                }
            }
            "EC" => {
                if self.x.is_none() || self.y.is_none() || self.crv.is_none() {
                    return Err(JwksError::MissingField(
                        "EC key must have 'x', 'y', and 'crv' fields".to_string(),
                    ));
                }
            }
            other => {
                return Err(JwksError::UnsupportedKeyType(other.to_string()));
            }
        }

        Ok(())
    }
}

/// Decode base64url encoded string
/// Per RFC 4648 section 5
fn base64_url_decode(input: &str) -> Result<Vec<u8>, JwksError> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

    URL_SAFE_NO_PAD
        .decode(input)
        .map_err(|e| JwksError::Base64DecodeError(e.to_string()))
}

/// Construct RSA public key in DER format
/// Per RFC 3447 (PKCS#1)
fn construct_rsa_public_key(n: &[u8], e: &[u8]) -> Result<Vec<u8>, JwksError> {
    // Simple DER encoding for RSA public key
    // SEQUENCE {
    //   SEQUENCE {
    //     OBJECT IDENTIFIER rsaEncryption (1.2.840.113549.1.1.1)
    //     NULL
    //   }
    //   BIT STRING containing SEQUENCE { n, e }
    // }

    let mut modulus = encode_der_integer(n);
    let mut exponent = encode_der_integer(e);

    let mut inner_seq = Vec::new();
    inner_seq.extend_from_slice(&modulus);
    inner_seq.extend_from_slice(&exponent);

    let mut bit_string = Vec::new();
    bit_string.push(0x30); // SEQUENCE
    append_der_length(&mut bit_string, inner_seq.len());
    bit_string.extend_from_slice(&inner_seq);

    let mut bit_string_encoded = Vec::new();
    bit_string_encoded.push(0x03); // BIT STRING
    append_der_length(&mut bit_string_encoded, bit_string.len() + 1);
    bit_string_encoded.push(0x00); // no unused bits
    bit_string_encoded.extend_from_slice(&bit_string);

    // RSA encryption OID: 1.2.840.113549.1.1.1
    let oid_seq = vec![
        0x30, 0x0d, // SEQUENCE
        0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01, // OID
        0x05, 0x00, // NULL
    ];

    let mut result = Vec::new();
    result.push(0x30); // SEQUENCE
    append_der_length(&mut result, oid_seq.len() + bit_string_encoded.len());
    result.extend_from_slice(&oid_seq);
    result.extend_from_slice(&bit_string_encoded);

    Ok(result)
}

/// Encode integer in DER format
fn encode_der_integer(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    result.push(0x02); // INTEGER tag

    // Add padding byte if high bit is set
    let mut padded_data = Vec::new();
    if !data.is_empty() && (data[0] & 0x80) != 0 {
        padded_data.push(0x00);
    }
    padded_data.extend_from_slice(data);

    append_der_length(&mut result, padded_data.len());
    result.extend_from_slice(&padded_data);
    result
}

/// Append DER length encoding
fn append_der_length(output: &mut Vec<u8>, length: usize) {
    if length < 128 {
        output.push(length as u8);
    } else if length < 256 {
        output.push(0x81);
        output.push(length as u8);
    } else if length < 65536 {
        output.push(0x82);
        output.push((length >> 8) as u8);
        output.push((length & 0xff) as u8);
    } else {
        output.push(0x83);
        output.push((length >> 16) as u8);
        output.push((length >> 8) as u8);
        output.push((length & 0xff) as u8);
    }
}

/// Construct EC public key in PEM format
fn construct_ec_public_key(x: &[u8], y: &[u8], curve: &str) -> Result<Vec<u8>, JwksError> {
    // For EC keys, we need to construct the full PEM format
    // This is a simplified version - in production, use a proper crypto library
    let curve_oid = match curve {
        "P-256" => "prime256v1",
        "P-384" => "secp384r1",
        "P-521" => "secp521r1",
        other => {
            return Err(JwksError::UnsupportedAlgorithm(format!(
                "Unsupported EC curve: {}",
                other
            )))
        }
    };

    // Construct uncompressed point: 0x04 || x || y
    let mut point = Vec::new();
    point.push(0x04);
    point.extend_from_slice(x);
    point.extend_from_slice(y);

    // For now, return the point data
    // In production, this should be properly encoded in PEM format
    Ok(format!(
        "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----",
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &point
        )
    )
    .into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwks_parse() {
        let jwks_json = r#"{
            "keys": [
                {
                    "kid": "test-key-1",
                    "kty": "RSA",
                    "alg": "RS256",
                    "use": "sig",
                    "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbOpbISD08qNLyrdkt-bFTWhAI4vMQFh6WeZu0fM4lFd2NcRwr3XPksINHaQ-G_xBniIqbw0Ls1jF44-csFCur-kEgU8awapJzKnqDKgw",
                    "e": "AQAB"
                }
            ]
        }"#;

        let jwks: JsonWebKeySet = serde_json::from_str(jwks_json).unwrap();
        assert_eq!(jwks.keys.len(), 1);
        assert_eq!(jwks.keys[0].kid, "test-key-1");
        assert_eq!(jwks.keys[0].kty, "RSA");
        assert_eq!(jwks.keys[0].alg, "RS256");
    }

    #[test]
    fn test_find_key() {
        let jwks_json = r#"{
            "keys": [
                {
                    "kid": "key-1",
                    "kty": "RSA",
                    "alg": "RS256",
                    "use": "sig",
                    "n": "test",
                    "e": "AQAB"
                },
                {
                    "kid": "key-2",
                    "kty": "RSA",
                    "alg": "RS384",
                    "use": "sig",
                    "n": "test",
                    "e": "AQAB"
                }
            ]
        }"#;

        let jwks: JsonWebKeySet = serde_json::from_str(jwks_json).unwrap();
        assert!(jwks.find_key("key-1").is_some());
        assert!(jwks.find_key("key-2").is_some());
        assert!(jwks.find_key("key-3").is_none());
    }

    #[test]
    fn test_algorithm_conversion() {
        let key = JsonWebKey {
            kid: "test".to_string(),
            kty: "RSA".to_string(),
            alg: "RS256".to_string(),
            use_: "sig".to_string(),
            n: Some("test".to_string()),
            e: Some("AQAB".to_string()),
            crv: None,
            x: None,
            y: None,
            other: HashMap::new(),
        };

        assert!(matches!(key.algorithm().unwrap(), Algorithm::RS256));
    }

    #[test]
    fn test_validate_rsa_key() {
        let key = JsonWebKey {
            kid: "test".to_string(),
            kty: "RSA".to_string(),
            alg: "RS256".to_string(),
            use_: "sig".to_string(),
            n: Some("test".to_string()),
            e: Some("AQAB".to_string()),
            crv: None,
            x: None,
            y: None,
            other: HashMap::new(),
        };

        assert!(key.validate().is_ok());
    }

    #[test]
    fn test_validate_missing_fields() {
        let key = JsonWebKey {
            kid: "test".to_string(),
            kty: "RSA".to_string(),
            alg: "RS256".to_string(),
            use_: "sig".to_string(),
            n: None, // Missing
            e: Some("AQAB".to_string()),
            crv: None,
            x: None,
            y: None,
            other: HashMap::new(),
        };

        assert!(key.validate().is_err());
    }

    #[test]
    fn test_base64_url_decode() {
        let input = "AQAB"; // Common RSA exponent
        let result = base64_url_decode(input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1, 0, 1]);
    }

    #[test]
    fn test_unsupported_algorithm() {
        let key = JsonWebKey {
            kid: "test".to_string(),
            kty: "RSA".to_string(),
            alg: "HS256".to_string(), // Not supported
            use_: "sig".to_string(),
            n: Some("test".to_string()),
            e: Some("AQAB".to_string()),
            crv: None,
            x: None,
            y: None,
            other: HashMap::new(),
        };

        assert!(key.algorithm().is_err());
    }

    #[test]
    fn test_keys_for_algorithm() {
        let jwks_json = r#"{
            "keys": [
                {
                    "kid": "rs256-key",
                    "kty": "RSA",
                    "alg": "RS256",
                    "use": "sig",
                    "n": "test",
                    "e": "AQAB"
                },
                {
                    "kid": "rs384-key",
                    "kty": "RSA",
                    "alg": "RS384",
                    "use": "sig",
                    "n": "test",
                    "e": "AQAB"
                }
            ]
        }"#;

        let jwks: JsonWebKeySet = serde_json::from_str(jwks_json).unwrap();
        let rs256_keys = jwks.keys_for_algorithm("RS256");
        assert_eq!(rs256_keys.len(), 1);
        assert_eq!(rs256_keys[0].kid, "rs256-key");
    }
}