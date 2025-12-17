use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

use crate::ble::BleDevice;
use crate::error::SensorError;

pub type Timestamp = u64;
pub type SensorStream = BoxStream<'static, SensorReading>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SensorType {
    Microphone,
    Camera,
    Light,
    Temperature,
    Accelerometer,
    Gyroscope,
    Gps,
    Ble,
    Wifi,
    Gpio,
    Custom(String),
}

impl std::fmt::Display for SensorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SensorType::Microphone => write!(f, "microphone"),
            SensorType::Camera => write!(f, "camera"),
            SensorType::Light => write!(f, "light"),
            SensorType::Temperature => write!(f, "temperature"),
            SensorType::Accelerometer => write!(f, "accelerometer"),
            SensorType::Gyroscope => write!(f, "gyroscope"),
            SensorType::Gps => write!(f, "gps"),
            SensorType::Ble => write!(f, "ble"),
            SensorType::Wifi => write!(f, "wifi"),
            SensorType::Gpio => write!(f, "gpio"),
            SensorType::Custom(name) => write!(f, "custom.{}", name),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageFormat {
    Rgb8,
    Rgba8,
    Yuv420,
    Jpeg,
    Png,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SensorData {
    Audio {
        samples: Vec<f32>,
        sample_rate: u32,
    },
    Image {
        width: u32,
        height: u32,
        data: Vec<u8>,
        format: ImageFormat,
    },
    Light {
        lux: f32,
    },
    Temperature {
        celsius: f32,
    },
    Motion {
        x: f32,
        y: f32,
        z: f32,
    },
    Location {
        lat: f64,
        lon: f64,
        altitude: Option<f64>,
    },
    Ble {
        devices: Vec<BleDevice>,
    },
    Gpio {
        pin: u8,
        value: bool,
    },
    Raw {
        bytes: Vec<u8>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    pub timestamp: Timestamp,
    pub sensor_id: String,
    pub sensor_type: SensorType,
    pub data: SensorData,
}

#[async_trait]
pub trait Sensor: Send + Sync {
    fn sensor_type(&self) -> SensorType;
    fn id(&self) -> &str;

    async fn start(&mut self) -> Result<(), SensorError>;
    async fn stop(&mut self) -> Result<(), SensorError>;
    async fn read(&self) -> Result<SensorReading, SensorError>;

    fn subscribe(&self) -> SensorStream;
}
