use cortex_core::event::{Event, EventKind};
use cortex_core::id::NodeId;

use crate::traits::{SensorReading, SensorType};

pub fn sensor_event_kind(sensor_type: &SensorType) -> EventKind {
    match sensor_type {
        SensorType::Microphone => EventKind::new("sensor.mic.v1"),
        SensorType::Camera => EventKind::new("sensor.camera.v1"),
        SensorType::Light => EventKind::new("sensor.light.v1"),
        SensorType::Temperature => EventKind::new("sensor.temp.v1"),
        SensorType::Accelerometer => EventKind::new("sensor.accel.v1"),
        SensorType::Gyroscope => EventKind::new("sensor.gyro.v1"),
        SensorType::Gps => EventKind::new("sensor.gps.v1"),
        SensorType::Ble => EventKind::new("sensor.ble.v1"),
        SensorType::Wifi => EventKind::new("sensor.wifi.v1"),
        SensorType::Gpio => EventKind::new("sensor.gpio.v1"),
        SensorType::Custom(name) => EventKind::new(format!("sensor.custom.{}.v1", name)),
    }
}

pub fn reading_to_event(reading: SensorReading, origin: NodeId) -> Event {
    let kind = sensor_event_kind(&reading.sensor_type);
    let payload = serde_json::to_vec(&reading).unwrap_or_default();

    Event::new(kind, origin, payload)
}

pub fn event_to_reading(event: &Event) -> Option<SensorReading> {
    serde_json::from_slice(&event.payload).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{SensorData, SensorType};

    #[test]
    fn test_event_kind_mapping() {
        assert_eq!(
            sensor_event_kind(&SensorType::Microphone).as_str(),
            "sensor.mic.v1"
        );
        assert_eq!(
            sensor_event_kind(&SensorType::Ble).as_str(),
            "sensor.ble.v1"
        );
        assert_eq!(
            sensor_event_kind(&SensorType::Custom("pressure".into())).as_str(),
            "sensor.custom.pressure.v1"
        );
    }

    #[test]
    fn test_reading_roundtrip() {
        let reading = SensorReading {
            timestamp: 12345,
            sensor_id: "light-1".into(),
            sensor_type: SensorType::Light,
            data: SensorData::Light { lux: 500.0 },
        };

        let origin = NodeId::generate();
        let event = reading_to_event(reading.clone(), origin);

        let recovered = event_to_reading(&event).unwrap();
        assert_eq!(recovered.sensor_id, "light-1");
    }
}
