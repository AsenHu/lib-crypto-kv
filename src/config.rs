use std::{fs, net::SocketAddr, path::Path};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

impl From<LogLevel> for log::LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => log::LevelFilter::Trace,
            LogLevel::Debug => log::LevelFilter::Debug,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Warn => log::LevelFilter::Warn,
            LogLevel::Error => log::LevelFilter::Error,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct Log {
    pub level: LogLevel,
}

#[derive(Debug, Deserialize)]
pub struct ListenAddr {
    pub addr: SocketAddr,
}

#[derive(Debug, Deserialize)]
pub enum Mode {
    #[serde(rename = "high_throughput")]
    HighThroughput,
    #[serde(rename = "low_space")]
    LowSpace,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::LowSpace
    }
}

impl From<Mode> for sled::Mode {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::HighThroughput => sled::Mode::HighThroughput,
            Mode::LowSpace => sled::Mode::LowSpace,
        }
    }
}

impl From<sled::Mode> for Mode {
    fn from(mode: sled::Mode) -> Self {
        match mode {
            sled::Mode::HighThroughput => Mode::HighThroughput,
            sled::Mode::LowSpace => Mode::LowSpace,
        }
    }
}

fn default_compression_factor() -> i32 {
    5
}

fn default_cache_capacity() -> u64 {
    1024 * 1024 * 1024
}

#[derive(Debug, Deserialize)]
pub struct Database {
    pub path: Box<Path>,
    #[serde(default)]
    pub mode: Mode,
    #[serde(default = "default_compression_factor")]
    pub compression_factor: i32,
    #[serde(default = "default_cache_capacity")]
    pub cache_capacity: u64,
}

#[derive(Debug, Deserialize)]
pub struct Lock {
    pub expire_minute: i64,
}

impl Default for Lock {
    fn default() -> Self {
        Lock { expire_minute: 5 }
    }
}

#[derive(Debug, Deserialize)]
pub struct AutoDelete {
    pub max_days: u16,
    pub lazy: bool,
}

impl Default for AutoDelete {
    fn default() -> Self {
        AutoDelete {
            max_days: 365,
            lazy: false,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct GarbageCollection {
    #[serde(default)]
    pub lock: Lock,
    #[serde(default)]
    pub auto_delete: AutoDelete,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub log: Log,
    pub listen: ListenAddr,
    pub database: Database,
    #[serde(default)]
    pub garbage_collection: GarbageCollection,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self, Error> {
        let content = fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}
