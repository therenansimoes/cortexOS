use thiserror::Error;

/// Errors that can occur in sensor operations and hardware abstraction.
///
/// These errors cover sensor lifecycle, hardware interactions, permissions,
/// and various sensor-specific operations like BLE scanning.
#[derive(Debug, Error)]
pub enum SensorError {
    /// Requested sensor does not exist or is not registered
    #[error("sensor not found: {0}")]
    NotFound(String),

    /// Attempted to start a sensor that is already running
    #[error("sensor already running")]
    AlreadyRunning,

    /// Attempted operation on a sensor that is not running
    #[error("sensor not running")]
    NotRunning,

    /// Hardware-level error occurred (e.g., device failure, I/O error)
    #[error("hardware error: {0}")]
    Hardware(String),

    /// Sensor configuration is invalid or incomplete
    #[error("configuration error: {0}")]
    Config(String),

    /// Required permission was denied (e.g., camera, microphone access)
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// Operation timed out waiting for sensor data
    #[error("timeout")]
    Timeout,

    /// Sensor data channel was closed
    #[error("channel closed")]
    ChannelClosed,

    /// Operation is not supported by this sensor type
    #[error("unsupported operation: {0}")]
    Unsupported(String),
}

/// Convenience Result type for sensor operations
pub type Result<T> = std::result::Result<T, SensorError>;

/// Errors specific to Bluetooth Low Energy (BLE) operations.
///
/// BLE operations include scanning, advertising, and connection management.
#[derive(Debug, Error)]
pub enum BleError {
    /// BLE adapter is not available on this device
    #[error("adapter not available")]
    AdapterNotAvailable,

    /// BLE scan is already in progress
    #[error("scan already in progress")]
    ScanInProgress,

    /// Attempted to stop scan when not scanning
    #[error("not scanning")]
    NotScanning,

    /// BLE advertisement operation failed
    #[error("advertisement failed: {0}")]
    AdvertisementFailed(String),

    /// Bluetooth permission was denied by the system
    #[error("permission denied")]
    PermissionDenied,

    /// BLE hardware error occurred
    #[error("hardware error: {0}")]
    Hardware(String),
}

/// Convenience Result type for BLE operations
pub type BleResult<T> = std::result::Result<T, BleError>;
