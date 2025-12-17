use thiserror::Error;

#[derive(Debug, Error)]
pub enum SensorError {
    #[error("sensor not found: {0}")]
    NotFound(String),

    #[error("sensor already running")]
    AlreadyRunning,

    #[error("sensor not running")]
    NotRunning,

    #[error("hardware error: {0}")]
    Hardware(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("timeout")]
    Timeout,

    #[error("channel closed")]
    ChannelClosed,

    #[error("unsupported operation: {0}")]
    Unsupported(String),
}

#[derive(Debug, Error)]
pub enum BleError {
    #[error("adapter not available")]
    AdapterNotAvailable,

    #[error("scan already in progress")]
    ScanInProgress,

    #[error("not scanning")]
    NotScanning,

    #[error("advertisement failed: {0}")]
    AdvertisementFailed(String),

    #[error("permission denied")]
    PermissionDenied,

    #[error("hardware error: {0}")]
    Hardware(String),
}
