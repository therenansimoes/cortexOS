//! Device capability detection
//! 
//! Auto-detects hardware capabilities to select appropriate model size:
//! - Weak device (< 2GB RAM): TinyLlama 1.1B or smaller
//! - Medium device (2-8GB RAM): Qwen 0.5B-1.5B
//! - Strong device (> 8GB RAM): Qwen 7B or larger

use tracing::info;

/// Device tier based on available resources
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceTier {
    /// Weak device: phones, old computers, Raspberry Pi
    /// < 2GB RAM, uses smallest models
    Weak,
    /// Medium device: typical laptops, desktops
    /// 2-8GB RAM, uses mid-size models
    Medium,
    /// Strong device: gaming PCs, workstations
    /// > 8GB RAM, can run larger models
    Strong,
}

impl DeviceTier {
    /// Recommended model for this tier
    pub fn recommended_model(&self) -> &'static str {
        match self {
            DeviceTier::Weak => "qwen2.5:0.5b",      // 379MB
            DeviceTier::Medium => "qwen2.5:1.5b",    // ~1GB
            DeviceTier::Strong => "qwen2.5:7b",      // ~4GB
        }
    }
    
    /// Model context size for this tier
    pub fn context_size(&self) -> u32 {
        match self {
            DeviceTier::Weak => 512,
            DeviceTier::Medium => 2048,
            DeviceTier::Strong => 4096,
        }
    }
    
    /// Max tokens to generate for this tier
    pub fn max_tokens(&self) -> u32 {
        match self {
            DeviceTier::Weak => 64,
            DeviceTier::Medium => 128,
            DeviceTier::Strong => 256,
        }
    }
}

/// Detect device capabilities
pub fn detect_device_tier() -> DeviceTier {
    let total_memory_mb = get_total_memory_mb();
    let cpu_cores = get_cpu_cores();
    
    info!("📊 Detected: {}MB RAM, {} CPU cores", total_memory_mb, cpu_cores);
    
    let tier = if total_memory_mb < 2048 {
        DeviceTier::Weak
    } else if total_memory_mb < 8192 {
        DeviceTier::Medium
    } else {
        DeviceTier::Strong
    };
    
    info!("🎯 Device tier: {:?} → Model: {}", tier, tier.recommended_model());
    
    tier
}

/// Get total system memory in MB
fn get_total_memory_mb() -> u64 {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        
        if let Ok(output) = Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
        {
            if let Ok(s) = String::from_utf8(output.stdout) {
                if let Ok(bytes) = s.trim().parse::<u64>() {
                    return bytes / (1024 * 1024);
                }
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(kb) = parts.get(1) {
                        if let Ok(kb_val) = kb.parse::<u64>() {
                            return kb_val / 1024;
                        }
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        // Windows: use GlobalMemoryStatusEx
        // For now, return a default
    }
    
    // Default fallback
    4096 // Assume 4GB
}

/// Get number of CPU cores
fn get_cpu_cores() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(2)
}

/// Estimate if device can handle LLM inference
pub fn can_run_llm() -> bool {
    let tier = detect_device_tier();
    // Even weak devices can run the smallest models
    true
}

/// Get model download size in MB
pub fn model_download_size(tier: DeviceTier) -> u32 {
    match tier {
        DeviceTier::Weak => 379,    // Qwen 0.5B
        DeviceTier::Medium => 1100,  // Qwen 1.5B
        DeviceTier::Strong => 4000,  // Qwen 7B
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detection() {
        let tier = detect_device_tier();
        assert!(matches!(tier, DeviceTier::Weak | DeviceTier::Medium | DeviceTier::Strong));
    }
    
    #[test]
    fn test_model_recommendations() {
        assert!(DeviceTier::Weak.recommended_model().contains("0.5b"));
        assert!(DeviceTier::Strong.recommended_model().contains("7b"));
    }
}

