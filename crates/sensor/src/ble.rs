use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::error::BleError;

pub type BleDeviceStream = BoxStream<'static, BleDevice>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BleDevice {
    pub address: [u8; 6],
    pub name: Option<String>,
    pub rssi: i8,
    pub advertisement_data: Vec<u8>,
}

impl BleDevice {
    pub fn address_string(&self) -> String {
        self.address
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(":")
    }
}

#[async_trait]
pub trait BleAdapter: Send + Sync {
    async fn start_scan(&mut self) -> Result<(), BleError>;
    async fn stop_scan(&mut self) -> Result<(), BleError>;
    async fn advertise(&mut self, data: &[u8]) -> Result<(), BleError>;
    async fn stop_advertise(&mut self) -> Result<(), BleError>;
    fn device_stream(&self) -> BleDeviceStream;
}

pub struct MockBleAdapter {
    scanning: bool,
    advertising: bool,
    tx: broadcast::Sender<BleDevice>,
}

impl MockBleAdapter {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(64);
        Self {
            scanning: false,
            advertising: false,
            tx,
        }
    }

    pub fn inject_device(&self, device: BleDevice) {
        let _ = self.tx.send(device);
    }
}

impl Default for MockBleAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BleAdapter for MockBleAdapter {
    async fn start_scan(&mut self) -> Result<(), BleError> {
        if self.scanning {
            return Err(BleError::ScanInProgress);
        }
        self.scanning = true;
        tracing::info!("MockBleAdapter: started scanning");
        Ok(())
    }

    async fn stop_scan(&mut self) -> Result<(), BleError> {
        if !self.scanning {
            return Err(BleError::NotScanning);
        }
        self.scanning = false;
        tracing::info!("MockBleAdapter: stopped scanning");
        Ok(())
    }

    async fn advertise(&mut self, data: &[u8]) -> Result<(), BleError> {
        self.advertising = true;
        tracing::info!("MockBleAdapter: advertising {} bytes", data.len());
        Ok(())
    }

    async fn stop_advertise(&mut self) -> Result<(), BleError> {
        self.advertising = false;
        tracing::info!("MockBleAdapter: stopped advertising");
        Ok(())
    }

    fn device_stream(&self) -> BleDeviceStream {
        let mut rx = self.tx.subscribe();
        Box::pin(async_stream::stream! {
            while let Ok(device) = rx.recv().await {
                yield device;
            }
        })
    }
}
