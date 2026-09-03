//! # PKCE (RFC 7636)
//!
//! Proof Key for Code Exchange -- the mechanism that stops a native app's authorization
//! code from being redeemable by anyone but the Client that requested it, since a
//! public Client (no client secret) has no other way to prove its identity at the
//! token endpoint (ADR-0010, RFC 7636 §1).

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

/// Derive the `S256` `code_challenge` from a Client-generated `code_verifier`
/// (RFC 7636 §4.2): `BASE64URL-ENCODE(SHA256(ASCII(code_verifier)))`.
pub fn code_challenge_from_verifier(code_verifier: &str) -> String {
    let digest = Sha256::digest(code_verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

/// Check a presented `code_verifier` against the `code_challenge` recorded when the
/// authorization code was issued (RFC 7636 §4.6).
pub fn verify(code_verifier: &str, code_challenge: &str) -> bool {
    code_challenge_from_verifier(code_verifier) == code_challenge
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_accepts_the_matching_verifier() {
        let verifier = "a-sufficiently-long-random-code-verifier-string";
        let challenge = code_challenge_from_verifier(verifier);

        assert!(verify(verifier, &challenge));
    }

    #[test]
    fn verify_rejects_a_wrong_verifier() {
        let challenge = code_challenge_from_verifier("the-real-verifier");

        assert!(!verify("a-different-verifier", &challenge));
    }

    #[test]
    fn code_challenge_matches_the_rfc_7636_appendix_b_test_vector() {
        // RFC 7636 Appendix B's worked example.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let expected_challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

        assert_eq!(code_challenge_from_verifier(verifier), expected_challenge);
    }
}
