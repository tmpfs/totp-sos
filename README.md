# totp-lite
![Build Status](https://github.com/constantoine/totp-rs/workflows/Rust/badge.svg) [![docs](https://docs.rs/totp-rs/badge.svg)](https://docs.rs/totp-rs) [![](https://img.shields.io/crates/v/totp-rs.svg)](https://crates.io/crates/totp-rs) [![codecov](https://codecov.io/gh/constantoine/totp-rs/branch/master/graph/badge.svg?token=Q50RAIFVWZ)](https://codecov.io/gh/constantoine/totp-rs) [![cargo-audit](https://github.com/constantoine/totp-rs/actions/workflows/security.yml/badge.svg)](https://github.com/constantoine/totp-rs/actions/workflows/security.yml)

This library permits the creation of 2FA authentification tokens per TOTP, the verification of said tokens, with configurable time skew, validity time of each token, algorithm and number of digits! Default features are kept as lightweight as possible to ensure small binaries and short compilation time.

It now supports parsing [otpauth URLs](https://github.com/google/google-authenticator/wiki/Key-Uri-Format) into a totp object, with sane default values.

Be aware that some authenticator apps will accept the `SHA256` and `SHA512` algorithms but silently fallback to `SHA1` which will make the `check()` function fail due to mismatched algorithms.

## Features
---
### serde
With optional feature "serde", library-defined types `TOTP` and `Algorithm` and will be Deserialize-able and Serialize-able.
### zeroize
Securely zero secret information when the TOTP struct is dropped.


# Examples

## Summary

0. [Understanding Secret](#understanding-secret)
1. [Generate a token](#generate-a-token)
3. [Enable serde support](#with-serde-support)
4. [Enable otpauth url support](#with-otpauth-url-support)
6. [With RFC-6238 compliant default](#with-rfc-6238-compliant-default)

### Understanding Secret
---
This new type was added as a disambiguation between Raw and already base32 encoded secrets.
```Rust
    Secret::Raw("TestSecretSuperSecret".as_bytes().to_vec())
```
Is equivalent to
```Rust
    Secret::Encoded("KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ".to_string())
```
### Generate a token
---
Add it to your `Cargo.toml`:
```toml
[dependencies]
totp-rs = "^3.0"
```
You can then do something like:
```Rust
use std::time::SystemTime;
use totp_rs::{Algorithm, TOTP, Secret};

fn main() {
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        Secret::Raw("TestSecretSuperSecret".as_bytes().to_vec()).to_bytes().unwrap(),
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
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        Secret::Encoded("KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ".to_string()).to_bytes().unwrap(),
    ).unwrap();
    let token = totp.generate_current().unwrap();
    println!("{}", token);   
}
```

### With serde support
---
Add it to your `Cargo.toml`:
```toml
[dependencies.totp-rs]
version = "^3.0"
features = ["serde"]
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
