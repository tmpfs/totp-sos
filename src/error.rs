use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Secret '{0}' is not a valid non-padded base32 string")]
    Secret(String),

    #[error("An issuer '{0}' could be retrieved from the path, but a different issuer '{1}' was found in the issuer URL parameter")]
    IssuerMismatch(String, String),

    #[error("Issuer '{0}' must not contain a colon")]
    Issuer(String),
    
    #[error("Could not parse step '{0}' as a number")]
    Step(String),

    #[error("Could not parse digits '{0}' as a number")]
    Digits(String),

    #[error("Algorithm can only be SHA1, SHA256 or SHA512, not '{0}'")]
    Algorithm(String),

    #[error("Account name '{0}' must not contain a colon")]
    AccountName(String),

    #[error("Could not decode URL '{0}'")]
    IssuerDecoding(String),

    #[error("Host should be totp, not '{0}'")]
    Host(String),

    #[error("Scheme should be otpauth, not '{0}'")]
    Scheme(String),

    /// The length of the shared secret MUST be at least 128 bits
    #[error("The length of the shared secret MUST be at least 128 bits; {0} bits is not enough")]
    SecretTooSmall(usize),

    /// Implementations MUST extract a 6-digit code at a minimum and possibly 7 and 8-digit code
    #[error("Implementations MUST extract a 6-digit code at a minimum and possibly 7 and 8-digit code; {0} digits is not allowed")]
    InvalidDigits(usize),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error(transparent)]
    Time(#[from] std::time::SystemTimeError),
}
