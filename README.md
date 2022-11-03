# totp-sos

This library permits the creation of 2FA authentification tokens per TOTP, the verification of said tokens, with configurable time skew, validity time of each token, algorithm and number of digits! Default features are kept as lightweight as possible to ensure small binaries and short compilation time.

Supports parsing [otpauth URLs](https://github.com/google/google-authenticator/wiki/Key-Uri-Format) into a totp object, with sane default values.

Be aware that some authenticator apps will accept the `SHA256` and `SHA512` algorithms but silently fallback to `SHA1` which will make the `check()` function fail due to mismatched algorithms.

## Features
---
### zeroize
Securely zero secret information when the TOTP struct is dropped; this feature is enabled by default.
### serde
Optional feature `serde`, enables serde support for the `TOTP` and `Algorithm` types.


# Examples

## Summary

* [Generate a token](#generate-a-token)
* [Enable serde support](#with-serde-support)
* [Enable otpauth url support](#with-otpauth-url-support)
* [With RFC-6238 compliant default](#with-rfc-6238-compliant-default)

### Generate a token
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
Which is equivalent to:
```Rust
use std::time::SystemTime;
use totp_rs::{Algorithm, TOTP, Secret};

fn main() {
    let totp = TOTP::from_secret_base32(""KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ"").unwrap();
    let token = totp.generate_current().unwrap();
    println!("{}", token);   
}
```

### With RFC-6238 compliant default
---
You can do something like this
```Rust
use totp_rs::{Algorithm, TOTP, Secret, Rfc6238};

fn main () {
    let mut rfc = Rfc6238::with_defaults(
            Secret::Encoded("KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ".to_string()).to_bytes().unwrap(),
        )
        .unwrap();

    // optional, set digits
    rfc.digits(8).unwrap();

    // create a TOTP from rfc
    let totp = TOTP::from_rfc6238(rfc).unwrap();
    let code = totp.generate_current().unwrap();
    println!("code: {}", code);
}
```
