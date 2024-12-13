use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub data: Arc<Mutex<AppStateData>>,
    pub config: AppConfig,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(AppStateData::new())),
            config: AppConfig::new(),
        }
    }
}

pub struct AppStateData {
    pub fileserver_hits: u64,
}

impl AppStateData {
    fn new() -> Self {
        Self { fileserver_hits: 0 }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Platform {
    Dev,
    Prod,
}

impl Platform {
    fn new() -> Self {
        // Use safest default option
        Platform::Prod
    }
}

impl From<&str> for Platform {
    fn from(value: &str) -> Self {
        match value {
            "dev" => Platform::Dev,
            "prod" => Platform::Prod,
            _ => Platform::new(),
        }
    }
}

#[derive(Clone)]
pub struct AppConfig {
    pub platform: Platform,
}

impl AppConfig {
    fn new() -> Self {
        // Use safest options as default
        AppConfig {
            platform: Platform::new(),
        }
    }
}
