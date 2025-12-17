use std::collections::HashMap;

use crate::traits::{Sensor, SensorType};

pub struct SensorRegistry {
    sensors: HashMap<String, Box<dyn Sensor>>,
}

impl SensorRegistry {
    pub fn new() -> Self {
        Self {
            sensors: HashMap::new(),
        }
    }

    pub fn register(&mut self, sensor: Box<dyn Sensor>) {
        let id = sensor.id().to_string();
        tracing::info!("Registering sensor: {} ({:?})", id, sensor.sensor_type());
        self.sensors.insert(id, sensor);
    }

    pub fn unregister(&mut self, id: &str) -> Option<Box<dyn Sensor>> {
        self.sensors.remove(id)
    }

    pub fn get(&self, id: &str) -> Option<&dyn Sensor> {
        self.sensors.get(id).map(|s| s.as_ref())
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Box<dyn Sensor>> {
        self.sensors.get_mut(id)
    }

    pub fn by_type(&self, sensor_type: SensorType) -> Vec<&dyn Sensor> {
        self.sensors
            .values()
            .filter(|s| s.sensor_type() == sensor_type)
            .map(|s| s.as_ref())
            .collect()
    }

    pub fn all(&self) -> Vec<&dyn Sensor> {
        self.sensors.values().map(|s| s.as_ref()).collect()
    }

    pub fn ids(&self) -> Vec<&str> {
        self.sensors.keys().map(|s| s.as_str()).collect()
    }

    pub fn count(&self) -> usize {
        self.sensors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sensors.is_empty()
    }
}

impl Default for SensorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockLight;

    #[test]
    fn test_register_and_get() {
        let mut registry = SensorRegistry::new();
        let sensor = MockLight::new("light-1");

        registry.register(Box::new(sensor));

        assert_eq!(registry.count(), 1);
        assert!(registry.get("light-1").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_by_type() {
        let mut registry = SensorRegistry::new();
        registry.register(Box::new(MockLight::new("light-1")));
        registry.register(Box::new(MockLight::new("light-2")));

        let lights = registry.by_type(SensorType::Light);
        assert_eq!(lights.len(), 2);

        let mics = registry.by_type(SensorType::Microphone);
        assert!(mics.is_empty());
    }
}
