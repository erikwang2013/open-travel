#[test]
fn debug_issuer_with_authclaims() {
    let secret = "0123456789abcdef0123456789abcdef";
    let claims =
        serde_json::json!({"sub":"user-1","exp":4_102_444_800u64,"iss":"https://other.example"});
    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap();
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.set_issuer(&["https://issuer.example"]);
    let res = jsonwebtoken::decode::<ecat_auth::AuthClaims>(
        &token,
        &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    );
    assert!(
        res.is_err(),
        "wrong iss must be rejected with AuthClaims too, got: {res:?}"
    );
}
