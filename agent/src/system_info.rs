#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemInfo {
    pub os: String,
    pub architecture: String,
    pub hostname: Option<String>,
}

pub fn collect() -> SystemInfo {
    SystemInfo {
        os: std::env::consts::OS.to_owned(),
        architecture: std::env::consts::ARCH.to_owned(),
        hostname: std::fs::read_to_string("/etc/hostname")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty() && !value.chars().any(char::is_control)),
    }
}
