# totp-sos

This library supports the creation of 2FA authentification tokens per TOTP, the verification of said tokens, with configurable time skew, validity time of each token, algorithm and number of digits! Default features are kept as lightweight as possible to ensure small binaries and short compilation time.

Supports parsing [otpauth URLs](https://github.com/google/google-authenticator/wiki/Key-Uri-Format) into a totp object, with sane default values.

Be aware that some authenticator apps will accept the `SHA256` and `SHA512` algorithms but silently fallback to `SHA1` which will make the `check()` function fail due to mismatched algorithms.

Derived from the [totp-rs](https://docs.rs/totp-rs/latest/totp_rs/) crate with a simpler API and fewer dependencies.

## Features
---
### zeroize
Securely zero secret information when the TOTP struct is dropped; this feature is enabled by default.
### serde
Optional feature `serde`, enables serde support for the `TOTP` and `Algorithm` types.

## Generate a token
---

```Rust
use std::time::SystemTime;
use totp_rs::{Algorithm, TOTP, Secret};

fn main() {
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        "TestSecretSuperSecret".as_bytes().to_vec(),
        "mock@example.com".to_string(),
        Some("Mock issuer".to_string()),
    ).unwrap();
    let token = totp.generate_current().unwrap();
    println!("{}", token);   
}
```

* [RFC6238](https://tools.ietf.org/html/rfc6238)
* [RFC4226](https://tools.ietf.org/html/rfc4226)

