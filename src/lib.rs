#![deny(missing_docs)]
#![forbid(unsafe_code)]

//! This library permits the creation of 2FA authentification tokens per TOTP, the verification of said tokens, with configurable time skew, validity time of each token, algorithm and number of digits! Default features are kept as low-dependency as possible to ensure small binaries and short compilation time
//!
//! Be aware that some authenticator apps will accept the `SHA256`
//! and `SHA512` algorithms but silently fallback to `SHA1` which will
//! make the `check()` function fail due to mismatched algorithms.
//!
//! Use the `SHA1` algorithm to avoid this problem.
//!
//! # Examples
//!
//! ```rust
//! use totp_sos::{Algorithm, TOTP};
//!
//! let totp = TOTP::new(
//!     Algorithm::SHA1,
//!     6,
//!     1,
//!     30,
//!     "TestSecretSuperSecret".as_bytes().to_vec(),
//!     "mock@example.com".to_string(),
//!     Some("Github".to_string()),
//! ).unwrap();
//! let token = totp.generate_current().unwrap();
//! println!("{}", token);
//! ```

mod error;

pub use error::Error;

/// Result type for the TOTP library.
pub type Result<T> = std::result::Result<T, Error>;

use constant_time_eq::constant_time_eq;
use hmac::Mac;
use std::{
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};
use url::{Host, Url};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

type HmacSha1 = hmac::Hmac<sha1::Sha1>;
type HmacSha256 = hmac::Hmac<sha2::Sha256>;
type HmacSha512 = hmac::Hmac<sha2::Sha512>;

/// Algorithm enum holds the three standards algorithms for TOTP as per the [reference implementation](https://tools.ietf.org/html/rfc6238#appendix-A)
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Algorithm {
    /// The SHA1 algorithm.
    SHA1,
    /// The SHA256 algorithm.
    SHA256,
    /// The SHA512 algorithm.
    SHA512,
}

impl std::default::Default for Algorithm {
    fn default() -> Self {
        Algorithm::SHA1
    }
}

impl fmt::Display for Algorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Algorithm::SHA1 => f.write_str("SHA1"),
            Algorithm::SHA256 => f.write_str("SHA256"),
            Algorithm::SHA512 => f.write_str("SHA512"),
        }
    }
}

impl Algorithm {
    fn hash<D>(mut digest: D, data: &[u8]) -> Vec<u8>
    where
        D: Mac,
    {
        digest.update(data);
        digest.finalize().into_bytes().to_vec()
    }

    fn sign(&self, key: &[u8], data: &[u8]) -> Vec<u8> {
        match self {
            Algorithm::SHA1 => {
                Algorithm::hash(HmacSha1::new_from_slice(key).unwrap(), data)
            }
            Algorithm::SHA256 => Algorithm::hash(
                HmacSha256::new_from_slice(key).unwrap(),
                data,
            ),
            Algorithm::SHA512 => Algorithm::hash(
                HmacSha512::new_from_slice(key).unwrap(),
                data,
            ),
        }
    }
}

fn system_time() -> Result<u64> {
    let t = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    Ok(t)
}

/// TOTP holds informations as to how to generate an auth code and validate it. Its [secret](struct.TOTP.html#structfield.secret) field is sensitive data, treat it accordingly
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "zeroize",
    derive(zeroize::Zeroize, zeroize::ZeroizeOnDrop)
)]
pub struct TOTP {
    /// SHA-1 is the most widespread algorithm used, 
    /// and for totp pursposes, SHA-1 hash collisions 
    /// are [not a problem](https://tools.ietf.org/html/rfc4226#appendix-B.2) 
    /// as HMAC-SHA-1 is not impacted. It's also the main 
    /// one cited in [rfc-6238](https://tools.ietf.org/html/rfc6238#section-3) 
    /// even though the [reference implementation](https://tools.ietf.org/html/rfc6238#appendix-A) 
    /// permits the use of SHA-1, SHA-256 and SHA-512.
    ///
    /// Not all clients support other algorithms then SHA-1
    #[cfg_attr(feature = "zeroize", zeroize(skip))]
    pub algorithm: Algorithm,

    /// The number of digits for the auth code.
    ///
    /// Per [rfc-4226](https://tools.ietf.org/html/rfc4226#section-5.3), 
    /// this can be in the range between 6 and 8 digits
    pub digits: usize,

    /// Number of steps allowed as network delay.
    ///
    /// One would mean one step before current step and 
    /// one step after are valid.
    ///
    /// The recommended value per [rfc-6238](https://tools.ietf.org/html/rfc6238#section-5.2) is 1. Anything more is sketchy and should not be used.
    pub skew: u8,

    /// Duration in seconds of a step.
    ///
    /// The recommended value per [rfc-6238](https://tools.ietf.org/html/rfc6238#section-5.2) is 30 seconds
    pub step: u64,

    /// As per [rfc-4226](https://tools.ietf.org/html/rfc4226#section-4) 
    /// the secret should come from a strong source, most likely a CSPRNG.
    ///
    /// It should be at least 128 bits, but 160 are recommended.
    pub secret: Vec<u8>,

    /// The account name, typically either an email address or username.
    ///
    /// The "mock@example.com" part of "Github:mock@example.com".
    ///
    /// Must not contain a colon `:`.
    pub account_name: String,

    /// The name of your service/website.
    ///
    /// The "Github" part of "Github:mock@example.com".
    ///
    /// Must not contain a colon `:`.
    pub issuer: Option<String>,
}

impl PartialEq for TOTP {
    fn eq(&self, other: &Self) -> bool {
        constant_time_eq(self.secret.as_ref(), other.secret.as_ref())
    }
}

impl TOTP {
    /// Create a new instance of TOTP with given parameters.
    ///
    /// See [the doc](struct.TOTP.html#fields) for reference as to how to choose those values.
    ///
    /// * `digits`: MUST be between 6 & 8
    /// * `secret`: Must have bitsize of at least 128
    /// * `account_name`: Must not contain `:`
    /// * `issuer`: Must not contain `:`
    ///
    pub fn new(
        algorithm: Algorithm,
        digits: usize,
        skew: u8,
        step: u64,
        secret: Vec<u8>,
        account_name: String,
        issuer: Option<String>,
    ) -> Result<TOTP> {
        if !(6..=8).contains(&digits) {
            return Err(Error::InvalidDigits(digits));
        }

        if secret.len() < 16 {
            return Err(Error::SecretTooSmall(secret.len() * 8));
        }

        if account_name.contains(':') {
            return Err(Error::AccountName(account_name));
        }

        if let Some(issuer) = &issuer {
            if issuer.contains(':') {
                return Err(Error::Issuer(issuer.to_string()));
            }
        }

        Ok(TOTP {
            algorithm,
            digits,
            skew,
            step,
            secret,
            account_name,
            issuer,
        })
    }

    /// Sign the given timestamp
    pub fn sign(&self, time: u64) -> Vec<u8> {
        self.algorithm.sign(
            self.secret.as_ref(),
            (time / self.step).to_be_bytes().as_ref(),
        )
    }

    /// Generate a token given the provided timestamp in seconds
    pub fn generate(&self, time: u64) -> String {
        let result: &[u8] = &self.sign(time);
        let offset = (result.last().unwrap() & 15) as usize;
        let result = u32::from_be_bytes(
            result[offset..offset + 4].try_into().unwrap(),
        ) & 0x7fff_ffff;
        format!(
            "{1:00$}",
            self.digits,
            result % 10_u32.pow(self.digits as u32)
        )
    }

    /// Returns the timestamp of the first second for the next step
    /// given the provided timestamp in seconds
    pub fn next_step(&self, time: u64) -> u64 {
        let step = time / self.step;
        (step + 1) * self.step
    }

    /// Returns the timestamp of the first second of the next step
    /// According to system time
    pub fn next_step_current(&self) -> Result<u64> {
        let t = system_time()?;
        Ok(self.next_step(t))
    }

    /// Give the ttl (in seconds) of the current token
    pub fn ttl(&self) -> Result<u64> {
        let t = system_time()?;
        Ok(self.step - (t % self.step))
    }

    /// Generate a token from the current system time
    pub fn generate_current(&self) -> Result<String> {
        let t = system_time()?;
        Ok(self.generate(t))
    }

    /// Check if token is valid given the provided timestamp 
    /// in seconds, accounting [skew](struct.TOTP.html#structfield.skew)
    pub fn check(&self, token: &str, time: u64) -> bool {
        let basestep = time / self.step - (self.skew as u64);
        for i in 0..self.skew * 2 + 1 {
            let step_time = (basestep + (i as u64)) * (self.step as u64);

            if constant_time_eq(
                self.generate(step_time).as_bytes(),
                token.as_bytes(),
            ) {
                return true;
            }
        }
        false
    }

    /// Check if token is valid by current system time, 
    /// accounting [skew](struct.TOTP.html#structfield.skew).
    pub fn check_current(&self, token: &str) -> Result<bool> {
        let t = system_time()?;
        Ok(self.check(token, t))
    }

    /// Return the base32 representation of the secret, which 
    /// might be useful when users want to manually add the 
    /// secret to their authenticator.
    pub fn to_secret_base32(&self) -> String {
        base32::encode(
            base32::Alphabet::RFC4648 { padding: false },
            self.secret.as_ref(),
        )
    }

    /// Convert a base32 secret into a TOTP.
    ///
    /// The account name is the empty string and the issuer is None; 
    /// so you should set them explicitly after decoding the secret bytes.
    pub fn from_secret_base32<S: AsRef<str>>(secret: S) -> Result<TOTP> {
        let buffer = base32::decode(
            base32::Alphabet::RFC4648 { padding: false },
            secret.as_ref(),
        )
        .ok_or(Error::Secret(secret.as_ref().to_string()))?;

        TOTP::new(Algorithm::SHA1, 6, 1, 30, buffer, String::new(), None)
    }

    /// Generate a TOTP from the standard otpauth URL
    pub fn from_url<S: AsRef<str>>(url: S) -> Result<TOTP> {
        let url = Url::parse(url.as_ref())?;

        if url.scheme() != "otpauth" {
            return Err(Error::Scheme(url.scheme().to_string()));
        }
        if url.host() != Some(Host::Domain("totp")) {
            return Err(Error::Host(url.host().unwrap().to_string()));
        }

        let mut algorithm = Algorithm::SHA1;
        let mut digits = 6;
        let mut step = 30;
        let mut secret = Vec::new();
        let mut account_name: String;
        let mut issuer: Option<String> = None;

        let path = url.path().trim_start_matches('/');
        if path.contains(':') {
            let parts = path.split_once(':').unwrap();
            issuer = Some(
                urlencoding::decode(parts.0.to_owned().as_str())
                    .map_err(|_| Error::IssuerDecoding(parts.0.to_owned()))?
                    .to_string(),
            );
            account_name = parts.1.trim_start_matches(':').to_owned();
        } else {
            account_name = path.to_owned();
        }

        account_name = urlencoding::decode(account_name.as_str())
            .map_err(|_| Error::AccountName(account_name.to_string()))?
            .to_string();

        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "algorithm" => {
                    algorithm = match value.as_ref() {
                        "SHA1" => Algorithm::SHA1,
                        "SHA256" => Algorithm::SHA256,
                        "SHA512" => Algorithm::SHA512,
                        _ => return Err(Error::Algorithm(value.to_string())),
                    }
                }
                "digits" => {
                    digits = value
                        .parse::<usize>()
                        .map_err(|_| Error::Digits(value.to_string()))?;
                }
                "period" => {
                    step = value
                        .parse::<u64>()
                        .map_err(|_| Error::Step(value.to_string()))?;
                }
                "secret" => {
                    secret = base32::decode(
                        base32::Alphabet::RFC4648 { padding: false },
                        value.as_ref(),
                    )
                    .ok_or_else(|| Error::Secret(value.to_string()))?;
                }
                "issuer" => {
                    let param_issuer = value
                        .parse::<String>()
                        .map_err(|_| Error::Issuer(value.to_string()))?;
                    if issuer.is_some()
                        && param_issuer.as_str() != issuer.as_ref().unwrap()
                    {
                        return Err(Error::IssuerMismatch(
                            issuer.as_ref().unwrap().to_string(),
                            param_issuer,
                        ));
                    }
                    issuer = Some(param_issuer);
                }
                _ => {}
            }
        }

        if secret.is_empty() {
            return Err(Error::Secret("".to_string()));
        }

        TOTP::new(algorithm, digits, 1, step, secret, account_name, issuer)
    }

    /// Generate a standard URL used to automatically add TOTP auths.
    ///
    /// Usually used with a QR code.
    ///
    /// Label and issuer will be URL-encoded; the secret will be 
    /// converted to base32 without padding, as per the RFC.
    pub fn get_url(&self) -> String {
        let account_name: String =
            urlencoding::encode(self.account_name.as_str()).to_string();
        let mut label: String = format!("{}?", account_name);
        if self.issuer.is_some() {
            let issuer: String =
                urlencoding::encode(self.issuer.as_ref().unwrap().as_str())
                    .to_string();
            label = format!("{0}:{1}?issuer={0}&", issuer, account_name);
        }

        format!(
            "otpauth://totp/{}secret={}&digits={}&algorithm={}",
            label,
            self.to_secret_base32(),
            self.digits,
            self.algorithm,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_wrong_issuer() {
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            1,
            "TestSecretSuperSecret".as_bytes().to_vec(),
            "mock@example.com".to_string(),
            Some("Github:".to_string()),
        );
        assert!(totp.is_err());
        assert!(matches!(totp.unwrap_err(), Error::Issuer(_)));
    }

    #[test]
    fn new_wrong_account_name() {
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            1,
            "TestSecretSuperSecret".as_bytes().to_vec(),
            "mock:example.com".to_string(),
            Some("Github".to_string()),
        );
        assert!(totp.is_err());
        assert!(matches!(totp.unwrap_err(), Error::AccountName(_)));
    }

    #[test]
    fn new_wrong_account_name_no_issuer() {
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            1,
            "TestSecretSuperSecret".as_bytes().to_vec(),
            "mock:example.com".to_string(),
            None,
        );
        assert!(totp.is_err());
        assert!(matches!(totp.unwrap_err(), Error::AccountName(_)));
    }

    #[test]
    fn comparison_ok() {
        let reference = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            1,
            "TestSecretSuperSecret".as_bytes().to_vec(),
            "mock@example.com".to_string(),
            Some("Github".to_string()),
        )
        .unwrap();
        let test = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            1,
            "TestSecretSuperSecret".as_bytes().to_vec(),
            "mock@example.com".to_string(),
            Some("Github".to_string()),
        )
        .unwrap();
        assert_eq!(reference, test);
    }

    #[test]
    fn url_for_secret_matches_sha1_without_issuer() {
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            1,
            "TestSecretSuperSecret".as_bytes().to_vec(),
            "mock@example.com".to_string(),
            None,
        )
        .unwrap();
        let url = totp.get_url();
        assert_eq!(url.as_str(), "otpauth://totp/mock%40example.com?secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ&digits=6&algorithm=SHA1");
    }

    #[test]
    fn url_for_secret_matches_sha1() {
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            1,
            "TestSecretSuperSecret".as_bytes().to_vec(),
            "mock@example.com".to_string(),
            Some("Github".to_string()),
        )
        .unwrap();
        let url = totp.get_url();
        assert_eq!(url.as_str(), "otpauth://totp/Github:mock%40example.com?issuer=Github&secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ&digits=6&algorithm=SHA1");
    }

    #[test]
    fn url_for_secret_matches_sha256() {
        let totp = TOTP::new(
            Algorithm::SHA256,
            6,
            1,
            1,
            "TestSecretSuperSecret".as_bytes().to_vec(),
            "mock@example.com".to_string(),
            Some("Github".to_string()),
        )
        .unwrap();
        let url = totp.get_url();
        assert_eq!(url.as_str(), "otpauth://totp/Github:mock%40example.com?issuer=Github&secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ&digits=6&algorithm=SHA256");
    }

    #[test]
    fn url_for_secret_matches_sha512() {
        let totp = TOTP::new(
            Algorithm::SHA512,
            6,
            1,
            1,
            "TestSecretSuperSecret".as_bytes().to_vec(),
            "mock@example.com".to_string(),
            Some("Github".to_string()),
        )
        .unwrap();
        let url = totp.get_url();
        assert_eq!(url.as_str(), "otpauth://totp/Github:mock%40example.com?issuer=Github&secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ&digits=6&algorithm=SHA512");
    }

    #[test]
    fn ttl_ok() {
        let totp = TOTP::new(
            Algorithm::SHA512,
            6,
            1,
            1,
            "TestSecretSuperSecret".as_bytes().to_vec(),
            "mock@example.com".to_string(),
            Some("Github".to_string()),
        )
        .unwrap();
        assert!(totp.ttl().is_ok());
    }

    #[test]
    fn returns_base32() {
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            1,
            "TestSecretSuperSecret".as_bytes().to_vec(),
            "mock@example.com".to_string(),
            None,
        )
        .unwrap();
        assert_eq!(
            totp.to_secret_base32().as_str(),
            "KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ"
        );
    }

    #[test]
    fn generate_token() {
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            1,
            "TestSecretSuperSecret".as_bytes().to_vec(),
            "mock@example.com".to_string(),
            None,
        )
        .unwrap();
        assert_eq!(totp.generate(1000).as_str(), "659761");
    }

    #[test]
    fn generate_token_current() {
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            1,
            "TestSecretSuperSecret".as_bytes().to_vec(),
            "mock@example.com".to_string(),
            None,
        )
        .unwrap();
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(
            totp.generate(time).as_str(),
            totp.generate_current().unwrap()
        );
    }

    #[test]
    fn generates_token_sha256() {
        let totp = TOTP::new(
            Algorithm::SHA256,
            6,
            1,
            1,
            "TestSecretSuperSecret".as_bytes().to_vec(),
            "mock@example.com".to_string(),
            None,
        )
        .unwrap();
        assert_eq!(totp.generate(1000).as_str(), "076417");
    }

    #[test]
    fn generates_token_sha512() {
        let totp = TOTP::new(
            Algorithm::SHA512,
            6,
            1,
            1,
            "TestSecretSuperSecret".as_bytes().to_vec(),
            "mock@example.com".to_string(),
            None,
        )
        .unwrap();
        assert_eq!(totp.generate(1000).as_str(), "473536");
    }

    #[test]
    fn checks_token() {
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            0,
            1,
            "TestSecretSuperSecret".as_bytes().to_vec(),
            "mock@example.com".to_string(),
            None,
        )
        .unwrap();
        assert!(totp.check("659761", 1000));
    }

    #[test]
    fn checks_token_current() {
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            0,
            1,
            "TestSecretSuperSecret".as_bytes().to_vec(),
            "mock@example.com".to_string(),
            None,
        )
        .unwrap();
        assert!(totp
            .check_current(&totp.generate_current().unwrap())
            .unwrap());
        assert!(!totp.check_current("bogus").unwrap());
    }

    #[test]
    fn checks_token_with_skew() {
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            1,
            "TestSecretSuperSecret".as_bytes().to_vec(),
            "mock@example.com".to_string(),
            None,
        )
        .unwrap();
        assert!(
            totp.check("174269", 1000)
                && totp.check("659761", 1000)
                && totp.check("260393", 1000)
        );
    }

    #[test]
    fn next_step() {
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            "TestSecretSuperSecret".as_bytes().to_vec(),
            "mock@example.com".to_string(),
            Some("Mock Service".to_string()),
        )
        .unwrap();
        assert!(totp.next_step(0) == 30);
        assert!(totp.next_step(29) == 30);
        assert!(totp.next_step(30) == 60);
    }

    #[test]
    fn from_url_err() {
        assert!(TOTP::from_url("otpauth://hotp/123").is_err());
        assert!(TOTP::from_url("otpauth://totp/GitHub:test").is_err());
        assert!(TOTP::from_url(
            "otpauth://totp/GitHub:test:?secret=ABC&digits=8&period=60&algorithm=SHA256"
        )
        .is_err());
        assert!(TOTP::from_url("otpauth://totp/Github:mock%40example.com?issuer=GitHub&secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ&digits=6&algorithm=SHA1").is_err())
    }

    #[test]
    fn from_url_default() {
        let totp = TOTP::from_url(
            "otpauth://totp/GitHub:test?secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ",
        )
        .unwrap();
        assert_eq!(
            totp.secret,
            base32::decode(
                base32::Alphabet::RFC4648 { padding: false },
                "KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ"
            )
            .unwrap()
        );
        assert_eq!(totp.algorithm, Algorithm::SHA1);
        assert_eq!(totp.digits, 6);
        assert_eq!(totp.skew, 1);
        assert_eq!(totp.step, 30);
    }

    #[test]
    fn from_url_query() {
        let totp = TOTP::from_url("otpauth://totp/GitHub:test?secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ&digits=8&period=60&algorithm=SHA256").unwrap();
        assert_eq!(
            totp.secret,
            base32::decode(
                base32::Alphabet::RFC4648 { padding: false },
                "KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ"
            )
            .unwrap()
        );
        assert_eq!(totp.algorithm, Algorithm::SHA256);
        assert_eq!(totp.digits, 8);
        assert_eq!(totp.skew, 1);
        assert_eq!(totp.step, 60);
    }

    #[test]
    fn from_url_query_sha512() {
        let totp = TOTP::from_url("otpauth://totp/GitHub:test?secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ&digits=8&period=60&algorithm=SHA512").unwrap();
        assert_eq!(
            totp.secret,
            base32::decode(
                base32::Alphabet::RFC4648 { padding: false },
                "KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ"
            )
            .unwrap()
        );
        assert_eq!(totp.algorithm, Algorithm::SHA512);
        assert_eq!(totp.digits, 8);
        assert_eq!(totp.skew, 1);
        assert_eq!(totp.step, 60);
    }

    #[test]
    fn from_url_to_url() {
        let totp = TOTP::from_url("otpauth://totp/Github:mock%40example.com?issuer=Github&secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ&digits=6&algorithm=SHA1").unwrap();
        let totp_bis = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            1,
            "TestSecretSuperSecret".as_bytes().to_vec(),
            "mock@example.com".to_string(),
            Some("Github".to_string()),
        )
        .unwrap();
        assert_eq!(totp.get_url(), totp_bis.get_url());
    }

    #[test]
    fn from_url_unknown_param() {
        let totp = TOTP::from_url("otpauth://totp/GitHub:test?secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ&digits=8&period=60&algorithm=SHA256&foo=bar").unwrap();
        assert_eq!(
            totp.secret,
            base32::decode(
                base32::Alphabet::RFC4648 { padding: false },
                "KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ"
            )
            .unwrap()
        );
        assert_eq!(totp.algorithm, Algorithm::SHA256);
        assert_eq!(totp.digits, 8);
        assert_eq!(totp.skew, 1);
        assert_eq!(totp.step, 60);
    }

    #[test]
    fn from_url_issuer_special() {
        let totp = TOTP::from_url("otpauth://totp/Github%40:mock%40example.com?issuer=Github%40&secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ&digits=6&algorithm=SHA1").unwrap();
        let totp_bis = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            1,
            "TestSecretSuperSecret".as_bytes().to_vec(),
            "mock@example.com".to_string(),
            Some("Github@".to_string()),
        )
        .unwrap();
        assert_eq!(totp.get_url(), totp_bis.get_url());
        assert_eq!(totp.issuer.as_ref().unwrap(), "Github@");
    }

    #[test]
    fn from_url_query_issuer() {
        let totp = TOTP::from_url("otpauth://totp/GitHub:test?issuer=GitHub&secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ&digits=8&period=60&algorithm=SHA256").unwrap();
        assert_eq!(
            totp.secret,
            base32::decode(
                base32::Alphabet::RFC4648 { padding: false },
                "KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ"
            )
            .unwrap()
        );
        assert_eq!(totp.algorithm, Algorithm::SHA256);
        assert_eq!(totp.digits, 8);
        assert_eq!(totp.skew, 1);
        assert_eq!(totp.step, 60);
        assert_eq!(totp.issuer.as_ref().unwrap(), "GitHub");
    }

    #[test]
    fn from_url_wrong_scheme() {
        let totp = TOTP::from_url("http://totp/GitHub:test?issuer=GitHub&secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ&digits=8&period=60&algorithm=SHA256");
        assert!(totp.is_err());
        let err = totp.unwrap_err();
        assert!(matches!(err, Error::Scheme(_)));
    }

    #[test]
    fn from_url_wrong_algo() {
        let totp = TOTP::from_url("otpauth://totp/GitHub:test?issuer=GitHub&secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ&digits=8&period=60&algorithm=MD5");
        assert!(totp.is_err());
        let err = totp.unwrap_err();
        assert!(matches!(err, Error::Algorithm(_)));
    }

    #[test]
    fn from_url_query_different_issuers() {
        let totp = TOTP::from_url("otpauth://totp/GitHub:test?issuer=Gitlab&secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ&digits=8&period=60&algorithm=SHA256");
        assert!(totp.is_err());
        assert!(matches!(totp.unwrap_err(), Error::IssuerMismatch(_, _)));
    }
}
