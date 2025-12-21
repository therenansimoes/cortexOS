use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SensorType {
    Microphone,
    Camera,
    Screen,
    Keyboard,
    Mouse,
    Clipboard,
    Location,
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    FileSystem {
        read: bool,
        write: bool,
        paths: Vec<PathBuf>,
    },
    Network {
        tcp: bool,
        udp: bool,
        hosts: Vec<String>,
    },
    Sensor(SensorType),
    Grid {
        relay: bool,
        task_accept: bool,
    },
    EventBus {
        publish: Vec<String>,
        subscribe: Vec<String>,
    },
}

impl Capability {
    pub fn fs_read(paths: Vec<PathBuf>) -> Self {
        Self::FileSystem {
            read: true,
            write: false,
            paths,
        }
    }

    pub fn fs_write(paths: Vec<PathBuf>) -> Self {
        Self::FileSystem {
            read: false,
            write: true,
            paths,
        }
    }

    pub fn fs_read_write(paths: Vec<PathBuf>) -> Self {
        Self::FileSystem {
            read: true,
            write: true,
            paths,
        }
    }

    pub fn network_tcp(hosts: Vec<String>) -> Self {
        Self::Network {
            tcp: true,
            udp: false,
            hosts,
        }
    }

    pub fn network_udp(hosts: Vec<String>) -> Self {
        Self::Network {
            tcp: false,
            udp: true,
            hosts,
        }
    }

    pub fn sensor(sensor_type: SensorType) -> Self {
        Self::Sensor(sensor_type)
    }

    pub fn grid_relay() -> Self {
        Self::Grid {
            relay: true,
            task_accept: false,
        }
    }

    pub fn grid_worker() -> Self {
        Self::Grid {
            relay: false,
            task_accept: true,
        }
    }

    pub fn grid_full() -> Self {
        Self::Grid {
            relay: true,
            task_accept: true,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CapabilitySet {
    capabilities: HashSet<Capability>,
}

impl CapabilitySet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capability(mut self, cap: Capability) -> Self {
        self.capabilities.insert(cap);
        self
    }

    pub fn add(&mut self, cap: Capability) {
        self.capabilities.insert(cap);
    }

    pub fn remove(&mut self, cap: &Capability) -> bool {
        self.capabilities.remove(cap)
    }

    pub fn has(&self, cap: &Capability) -> bool {
        self.capabilities.contains(cap)
    }

    pub fn check_fs_read(&self, path: &PathBuf) -> bool {
        self.capabilities.iter().any(|cap| match cap {
            Capability::FileSystem { read, paths, .. } => {
                *read && paths.iter().any(|p| path.starts_with(p))
            }
            _ => false,
        })
    }

    pub fn check_fs_write(&self, path: &PathBuf) -> bool {
        self.capabilities.iter().any(|cap| match cap {
            Capability::FileSystem { write, paths, .. } => {
                *write && paths.iter().any(|p| path.starts_with(p))
            }
            _ => false,
        })
    }

    pub fn check_network(&self, host: &str, is_tcp: bool) -> bool {
        self.capabilities.iter().any(|cap| match cap {
            Capability::Network { tcp, udp, hosts } => {
                let protocol_ok = if is_tcp { *tcp } else { *udp };
                protocol_ok && (hosts.is_empty() || hosts.iter().any(|h| host.contains(h)))
            }
            _ => false,
        })
    }

    pub fn check_sensor(&self, sensor_type: &SensorType) -> bool {
        self.capabilities
            .iter()
            .any(|cap| matches!(cap, Capability::Sensor(st) if st == sensor_type))
    }

    pub fn check_grid_relay(&self) -> bool {
        self.capabilities.iter().any(|cap| match cap {
            Capability::Grid { relay, .. } => *relay,
            _ => false,
        })
    }

    pub fn check_grid_task_accept(&self) -> bool {
        self.capabilities.iter().any(|cap| match cap {
            Capability::Grid { task_accept, .. } => *task_accept,
            _ => false,
        })
    }

    pub fn check_publish(&self, kind: &str) -> bool {
        self.capabilities.iter().any(|cap| match cap {
            Capability::EventBus { publish, .. } => {
                publish.iter().any(|p| pattern_matches(p, kind))
            }
            _ => false,
        })
    }

    pub fn check_subscribe(&self, pattern: &str) -> bool {
        self.capabilities.iter().any(|cap| match cap {
            Capability::EventBus { subscribe, .. } => {
                subscribe.iter().any(|s| pattern_matches(s, pattern))
            }
            _ => false,
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = &Capability> {
        self.capabilities.iter()
    }

    pub fn len(&self) -> usize {
        self.capabilities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }
}

fn pattern_matches(pattern: &str, target: &str) -> bool {
    if pattern == "*" || pattern == target {
        return true;
    }
    if pattern.ends_with(".*") {
        let prefix = &pattern[..pattern.len() - 2];
        return target.starts_with(prefix);
    }
    if pattern.ends_with("*") {
        let prefix = &pattern[..pattern.len() - 1];
        return target.starts_with(prefix);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_set() {
        let caps = CapabilitySet::new()
            .with_capability(Capability::fs_read(vec![PathBuf::from("/tmp")]))
            .with_capability(Capability::sensor(SensorType::Microphone));

        assert!(caps.check_fs_read(&PathBuf::from("/tmp/file.txt")));
        assert!(!caps.check_fs_write(&PathBuf::from("/tmp/file.txt")));
        assert!(caps.check_sensor(&SensorType::Microphone));
        assert!(!caps.check_sensor(&SensorType::Camera));
    }

    #[test]
    fn test_pattern_matching() {
        assert!(pattern_matches("sensor.*", "sensor.mic"));
        assert!(pattern_matches("*", "anything"));
        assert!(pattern_matches("exact", "exact"));
        assert!(!pattern_matches("sensor.*", "grid.msg"));
    }

    #[test]
    fn test_fs_write_capability() {
        let cap = Capability::fs_write(vec![PathBuf::from("/data")]);
        let mut caps = CapabilitySet::new();
        caps.add(cap);
        
        assert!(caps.check_fs_write(&PathBuf::from("/data/file.txt")));
        assert!(!caps.check_fs_read(&PathBuf::from("/data/file.txt")));
    }

    #[test]
    fn test_fs_read_write_capability() {
        let cap = Capability::fs_read_write(vec![PathBuf::from("/home")]);
        let caps = CapabilitySet::new().with_capability(cap);
        
        assert!(caps.check_fs_read(&PathBuf::from("/home/user/doc.txt")));
        assert!(caps.check_fs_write(&PathBuf::from("/home/user/doc.txt")));
    }

    #[test]
    fn test_network_tcp() {
        let cap = Capability::network_tcp(vec!["example.com".to_string()]);
        let caps = CapabilitySet::new().with_capability(cap);
        
        assert!(caps.check_network("example.com", true));
        assert!(!caps.check_network("example.com", false)); // UDP not allowed
    }

    #[test]
    fn test_network_udp() {
        let cap = Capability::network_udp(vec!["localhost".to_string()]);
        let caps = CapabilitySet::new().with_capability(cap);
        
        assert!(caps.check_network("localhost", false));
        assert!(!caps.check_network("localhost", true)); // TCP not allowed
    }

    #[test]
    fn test_grid_capabilities() {
        let relay_cap = Capability::grid_relay();
        let worker_cap = Capability::grid_worker();
        let full_cap = Capability::grid_full();
        
        let relay_caps = CapabilitySet::new().with_capability(relay_cap);
        assert!(relay_caps.check_grid_relay());
        assert!(!relay_caps.check_grid_task_accept());
        
        let worker_caps = CapabilitySet::new().with_capability(worker_cap);
        assert!(!worker_caps.check_grid_relay());
        assert!(worker_caps.check_grid_task_accept());
        
        let full_caps = CapabilitySet::new().with_capability(full_cap);
        assert!(full_caps.check_grid_relay());
        assert!(full_caps.check_grid_task_accept());
    }

    #[test]
    fn test_sensor_types() {
        let caps = CapabilitySet::new()
            .with_capability(Capability::sensor(SensorType::Camera))
            .with_capability(Capability::sensor(SensorType::Microphone))
            .with_capability(Capability::sensor(SensorType::Custom("GPS".to_string())));
        
        assert!(caps.check_sensor(&SensorType::Camera));
        assert!(caps.check_sensor(&SensorType::Microphone));
        assert!(caps.check_sensor(&SensorType::Custom("GPS".to_string())));
        assert!(!caps.check_sensor(&SensorType::Keyboard));
    }

    #[test]
    fn test_capability_set_operations() {
        let mut caps = CapabilitySet::new();
        assert!(caps.is_empty());
        assert_eq!(caps.len(), 0);
        
        let cap = Capability::sensor(SensorType::Screen);
        caps.add(cap.clone());
        assert!(!caps.is_empty());
        assert_eq!(caps.len(), 1);
        assert!(caps.has(&cap));
        
        assert!(caps.remove(&cap));
        assert!(caps.is_empty());
        assert!(!caps.has(&cap));
    }

    #[test]
    fn test_eventbus_capabilities() {
        let cap = Capability::EventBus {
            publish: vec!["sensor.*".to_string()],
            subscribe: vec!["grid.*".to_string(), "agent.intent".to_string()],
        };
        let caps = CapabilitySet::new().with_capability(cap);
        
        assert!(caps.check_publish("sensor.mic"));
        assert!(caps.check_publish("sensor.camera"));
        assert!(!caps.check_publish("grid.msg"));
        
        assert!(caps.check_subscribe("grid.*"));
        assert!(caps.check_subscribe("agent.intent"));
        assert!(!caps.check_subscribe("sensor.*"));
    }

    #[test]
    fn test_capability_iterator() {
        let caps = CapabilitySet::new()
            .with_capability(Capability::sensor(SensorType::Microphone))
            .with_capability(Capability::grid_relay());
        
        let count = caps.iter().count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_pattern_wildcard() {
        assert!(pattern_matches("test*", "test123"));
        assert!(pattern_matches("test*", "test"));
        assert!(!pattern_matches("test*", "tes"));
    }
}
