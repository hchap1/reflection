use rand::{Rng, distr::Alphanumeric, rng};
use sha2::Sha256;
use sha2::Digest;
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};

// Ensure the CSRF meets formatting requirements by limiting scope.
const CSRF_CHARSET: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
      abcdefghijklmnopqrstuvwxyz\
      0123456789";

/// Generate a random string that is echoed back throughout OAUTH2 processes to validate authenticity of foreign servers. If this code is different, terminate.
pub fn generate_csrf() -> String {
    let mut rng = rng();
    (0..32)
        .map(|_| {
            let id = rng.random_range(0..CSRF_CHARSET.len());
            CSRF_CHARSET[id] as char
        }).collect()
}

/// Create dual PKCE codes.
/// The verifier is stored in memory and the challenge is sent in the URL. Later, the client can be verified by checking the hash of verifier against the challenge.
pub fn generate_pkce() -> (String, String) {
    let verifier: String = rng()
        .sample_iter(&Alphanumeric)
        // 64 is the standard
        .take(64)
        .map(char::from)
        .collect();

    // Perform a SHA256 hash on the verifier.
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hashed = hasher.finalize();

    // Base64 encode the SHA256 hash as expected by the API, ending with a 43-byte string with URL protocol.
    let challenge = BASE64_URL_SAFE_NO_PAD.encode(hashed);

    // Return both the original PKCE and the hashed challenge.
    // Even if the original hash is intercepted, the verifier cannot be derived and thus an attacker cannot complete the authentication.
    (verifier, challenge)
}
