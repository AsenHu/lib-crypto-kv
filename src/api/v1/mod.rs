macro_rules! info {
    ($key:expr, $status:expr) => {
        ::log::info!("{}{} {}", LOG_PREFIX, $key, $status);
    };
}

macro_rules! error {
    ($key:expr,$status:expr,$error:expr) => {
        ::log::error!("{}{} {}", $key, $status, $error)
    };
}

pub mod kvs;
pub mod spin;
