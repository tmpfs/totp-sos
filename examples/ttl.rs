use totp_sos::{Algorithm, TOTP};

fn main() {
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        "ThisIsAnExampleSecretWithEnoughBytes".as_bytes().to_vec(),
        "mock@example.com".to_string(),
        Some("Github".to_string()),
    )
    .unwrap();

    loop {
        println!(
            "code {}\t ttl {}\t valid until: {}",
            totp.generate_current().unwrap(),
            totp.ttl().unwrap(),
            totp.next_step_current().unwrap()
        );
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
