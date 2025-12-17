use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;

use crate::ble::BleDevice;
use crate::error::SensorError;
use crate::traits::{Sensor, SensorData, SensorReading, SensorStream, SensorType};

fn now_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub struct MockMicrophone {
    id: String,
    running: Arc<AtomicBool>,
    sample_rate: u32,
    tx: broadcast::Sender<SensorReading>,
}

impl MockMicrophone {
    pub fn new(id: impl Into<String>, sample_rate: u32) -> Self {
        let (tx, _) = broadcast::channel(64);
        Self {
            id: id.into(),
            running: Arc::new(AtomicBool::new(false)),
            sample_rate,
            tx,
        }
    }

    fn generate_samples(&self) -> Vec<f32> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..1024).map(|_| rng.gen_range(-1.0..1.0)).collect()
    }
}

#[async_trait]
impl Sensor for MockMicrophone {
    fn sensor_type(&self) -> SensorType {
        SensorType::Microphone
    }

    fn id(&self) -> &str {
        &self.id
    }

    async fn start(&mut self) -> Result<(), SensorError> {
        if self.running.load(Ordering::SeqCst) {
            return Err(SensorError::AlreadyRunning);
        }
        self.running.store(true, Ordering::SeqCst);

        let running = self.running.clone();
        let tx = self.tx.clone();
        let id = self.id.clone();
        let sample_rate = self.sample_rate;

        tokio::spawn(async move {
            let mut rng = rand::thread_rng();
            while running.load(Ordering::SeqCst) {
                use rand::Rng;
                let samples: Vec<f32> = (0..1024).map(|_| rng.gen_range(-1.0..1.0)).collect();
                let reading = SensorReading {
                    timestamp: now_timestamp(),
                    sensor_id: id.clone(),
                    sensor_type: SensorType::Microphone,
                    data: SensorData::Audio {
                        samples,
                        sample_rate,
                    },
                };
                let _ = tx.send(reading);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });

        tracing::info!("MockMicrophone {} started", self.id);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), SensorError> {
        if !self.running.load(Ordering::SeqCst) {
            return Err(SensorError::NotRunning);
        }
        self.running.store(false, Ordering::SeqCst);
        tracing::info!("MockMicrophone {} stopped", self.id);
        Ok(())
    }

    async fn read(&self) -> Result<SensorReading, SensorError> {
        Ok(SensorReading {
            timestamp: now_timestamp(),
            sensor_id: self.id.clone(),
            sensor_type: SensorType::Microphone,
            data: SensorData::Audio {
                samples: self.generate_samples(),
                sample_rate: self.sample_rate,
            },
        })
    }

    fn subscribe(&self) -> SensorStream {
        let mut rx = self.tx.subscribe();
        Box::pin(async_stream::stream! {
            while let Ok(reading) = rx.recv().await {
                yield reading;
            }
        })
    }
}

pub struct MockLight {
    id: String,
    running: Arc<AtomicBool>,
    tx: broadcast::Sender<SensorReading>,
}

impl MockLight {
    pub fn new(id: impl Into<String>) -> Self {
        let (tx, _) = broadcast::channel(64);
        Self {
            id: id.into(),
            running: Arc::new(AtomicBool::new(false)),
            tx,
        }
    }
}

#[async_trait]
impl Sensor for MockLight {
    fn sensor_type(&self) -> SensorType {
        SensorType::Light
    }

    fn id(&self) -> &str {
        &self.id
    }

    async fn start(&mut self) -> Result<(), SensorError> {
        if self.running.load(Ordering::SeqCst) {
            return Err(SensorError::AlreadyRunning);
        }
        self.running.store(true, Ordering::SeqCst);

        let running = self.running.clone();
        let tx = self.tx.clone();
        let id = self.id.clone();

        tokio::spawn(async move {
            let mut rng = rand::thread_rng();
            while running.load(Ordering::SeqCst) {
                use rand::Rng;
                let lux = rng.gen_range(0.0..10000.0);
                let reading = SensorReading {
                    timestamp: now_timestamp(),
                    sensor_id: id.clone(),
                    sensor_type: SensorType::Light,
                    data: SensorData::Light { lux },
                };
                let _ = tx.send(reading);
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        });

        tracing::info!("MockLight {} started", self.id);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), SensorError> {
        if !self.running.load(Ordering::SeqCst) {
            return Err(SensorError::NotRunning);
        }
        self.running.store(false, Ordering::SeqCst);
        tracing::info!("MockLight {} stopped", self.id);
        Ok(())
    }

    async fn read(&self) -> Result<SensorReading, SensorError> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        Ok(SensorReading {
            timestamp: now_timestamp(),
            sensor_id: self.id.clone(),
            sensor_type: SensorType::Light,
            data: SensorData::Light {
                lux: rng.gen_range(0.0..10000.0),
            },
        })
    }

    fn subscribe(&self) -> SensorStream {
        let mut rx = self.tx.subscribe();
        Box::pin(async_stream::stream! {
            while let Ok(reading) = rx.recv().await {
                yield reading;
            }
        })
    }
}

pub struct MockBle {
    id: String,
    running: Arc<AtomicBool>,
    tx: broadcast::Sender<SensorReading>,
}

impl MockBle {
    pub fn new(id: impl Into<String>) -> Self {
        let (tx, _) = broadcast::channel(64);
        Self {
            id: id.into(),
            running: Arc::new(AtomicBool::new(false)),
            tx,
        }
    }

    fn generate_mock_devices() -> Vec<BleDevice> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let count = rng.gen_range(0..5);

        (0..count)
            .map(|i| {
                let mut address = [0u8; 6];
                rng.fill(&mut address);
                BleDevice {
                    address,
                    name: Some(format!("MockDevice-{}", i)),
                    rssi: rng.gen_range(-100..-30),
                    advertisement_data: vec![0x02, 0x01, 0x06],
                }
            })
            .collect()
    }
}

#[async_trait]
impl Sensor for MockBle {
    fn sensor_type(&self) -> SensorType {
        SensorType::Ble
    }

    fn id(&self) -> &str {
        &self.id
    }

    async fn start(&mut self) -> Result<(), SensorError> {
        if self.running.load(Ordering::SeqCst) {
            return Err(SensorError::AlreadyRunning);
        }
        self.running.store(true, Ordering::SeqCst);

        let running = self.running.clone();
        let tx = self.tx.clone();
        let id = self.id.clone();

        tokio::spawn(async move {
            while running.load(Ordering::SeqCst) {
                let devices = Self::generate_mock_devices();
                let reading = SensorReading {
                    timestamp: now_timestamp(),
                    sensor_id: id.clone(),
                    sensor_type: SensorType::Ble,
                    data: SensorData::Ble { devices },
                };
                let _ = tx.send(reading);
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        });

        tracing::info!("MockBle {} started", self.id);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), SensorError> {
        if !self.running.load(Ordering::SeqCst) {
            return Err(SensorError::NotRunning);
        }
        self.running.store(false, Ordering::SeqCst);
        tracing::info!("MockBle {} stopped", self.id);
        Ok(())
    }

    async fn read(&self) -> Result<SensorReading, SensorError> {
        Ok(SensorReading {
            timestamp: now_timestamp(),
            sensor_id: self.id.clone(),
            sensor_type: SensorType::Ble,
            data: SensorData::Ble {
                devices: Self::generate_mock_devices(),
            },
        })
    }

    fn subscribe(&self) -> SensorStream {
        let mut rx = self.tx.subscribe();
        Box::pin(async_stream::stream! {
            while let Ok(reading) = rx.recv().await {
                yield reading;
            }
        })
    }
}
