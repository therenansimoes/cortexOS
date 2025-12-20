use std::path::PathBuf;

pub struct NodeConfig {
    pub name: String,
    pub port: u16,
    pub data_dir: PathBuf,
    pub skills: Vec<String>,
    pub enable_kademlia: bool,
    pub enable_orchestrator: bool,
    pub can_compute: bool,
}

impl NodeConfig {
    pub fn new(
        name: Option<String>,
        port: u16,
        data_dir: Option<String>,
        skills: Option<String>,
    ) -> Self {
        let name = name.unwrap_or_else(|| {
            std::env::var("HOSTNAME")
                .or_else(|_| std::env::var("COMPUTERNAME"))
                .unwrap_or_else(|_| format!("cortex-{}", &uuid::Uuid::new_v4().to_string()[..8]))
        });

        let data_dir = data_dir
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                directories::ProjectDirs::from("com", "cortexos", "cortexd")
                    .map(|d| d.data_dir().to_path_buf())
                    .unwrap_or_else(|| PathBuf::from(".cortexos"))
            });

        let skills = skills
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        Self {
            name,
            port,
            data_dir,
            skills,
            enable_kademlia: true,      // Default to enabled
            enable_orchestrator: true,   // Default to enabled
            can_compute: true,           // Default to enabled
        }
    }
}
