//! CortexOS Desktop Application
//! 
//! Control panel for managing distributed AI peers.
//! Start, stop, configure, and monitor your peers.

mod peer_manager;
mod state;

use iced::widget::{button, column, container, row, scrollable, text, text_input, Column, Space};
use iced::{executor, theme, Application, Command, Element, Length, Settings, Theme};
use peer_manager::{PeerConfig, PeerManager, PeerStatus};
use state::AppState;
use std::sync::Arc;
use tokio::sync::RwLock;

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();
    
    CortexApp::run(Settings {
        window: iced::window::Settings {
            size: iced::Size::new(1100.0, 750.0),
            min_size: Some(iced::Size::new(900.0, 650.0)),
            ..Default::default()
        },
        ..Default::default()
    })
}

#[derive(Debug, Clone)]
enum Tab {
    Dashboard,
    Peers,
    Chat,
    Settings,
}

#[derive(Debug, Clone)]
enum Message {
    SwitchTab(Tab),
    RefreshData,
    DataRefreshed(serde_json::Value, Vec<serde_json::Value>, Vec<serde_json::Value>, Vec<serde_json::Value>),
    
    // Peer management
    AddPeer,
    StartPeer(String),
    StopPeer(String),
    RemovePeer(String),
    StartAllPeers,
    StopAllPeers,
    
    // Chat
    ChatInputChanged(String),
    SendChat,
    
    // Settings
    NameInputChanged(String),
    SaveName,
}

struct CortexApp {
    state: Arc<RwLock<AppState>>,
    peer_manager: Arc<RwLock<PeerManager>>,
    current_tab: Tab,
    chat_input: String,
    name_input: String,
    
    // Cached data
    cached_status: serde_json::Value,
    cached_peers: Vec<serde_json::Value>,
    cached_managed_peers: Vec<serde_json::Value>,
    cached_chat: Vec<serde_json::Value>,
}

impl Application for CortexApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let state = Arc::new(RwLock::new(AppState::new()));
        let peer_manager = Arc::new(RwLock::new(PeerManager::new()));
        
        // Add a default peer
        {
            let mut pm = peer_manager.try_write().unwrap();
            pm.add_peer(PeerConfig::default());
        }
        
        // Start the local peer services
        let state_clone = state.clone();
        tokio::spawn(async move {
            let mut s = state_clone.write().await;
            if let Err(e) = s.start().await {
                tracing::error!("Failed to start: {}", e);
            }
        });

        (
            Self {
                state,
                peer_manager,
                current_tab: Tab::Dashboard,
                chat_input: String::new(),
                name_input: String::new(),
                cached_status: serde_json::json!({}),
                cached_peers: vec![],
                cached_managed_peers: vec![],
                cached_chat: vec![],
            },
            Command::perform(async {}, |_| Message::RefreshData),
        )
    }

    fn title(&self) -> String {
        String::from("CortexOS - Peer Control Panel")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SwitchTab(tab) => {
                self.current_tab = tab;
                Command::none()
            }
            
            Message::RefreshData => {
                let state = self.state.clone();
                let pm = self.peer_manager.clone();
                Command::perform(
                    async move {
                        // Poll chat messages
                        {
                            let mut s = state.write().await;
                            s.poll_chat_messages();
                        }
                        
                        // Refresh peer statuses
                        {
                            let mut manager = pm.write().await;
                            manager.refresh_statuses();
                        }
                        
                        let s = state.read().await;
                        let status = s.get_status().await;
                        let peers = s.get_peers().await;
                        let chat = s.get_chat_messages().await;
                        
                        let manager = pm.read().await;
                        let managed = manager.get_peers();
                        
                        (status, peers, managed, chat)
                    },
                    |(status, peers, managed, chat)| Message::DataRefreshed(status, peers, managed, chat),
                )
            }
            
            Message::DataRefreshed(status, peers, managed, chat) => {
                self.cached_status = status;
                self.cached_peers = peers;
                self.cached_managed_peers = managed;
                self.cached_chat = chat;
                
                // Schedule next refresh
                Command::perform(
                    async { tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await },
                    |_| Message::RefreshData,
                )
            }
            
            // Peer Management
            Message::AddPeer => {
                let pm = self.peer_manager.clone();
                let offset = (self.cached_managed_peers.len() * 10) as u16;
                Command::perform(
                    async move {
                        let mut manager = pm.write().await;
                        let config = PeerConfig::default().with_port_offset(offset);
                        manager.add_peer(config);
                    },
                    |_| Message::RefreshData,
                )
            }
            
            Message::StartPeer(id) => {
                let pm = self.peer_manager.clone();
                Command::perform(
                    async move {
                        let mut manager = pm.write().await;
                        if let Err(e) = manager.start_peer(&id) {
                            tracing::error!("Failed to start peer: {}", e);
                        }
                    },
                    |_| Message::RefreshData,
                )
            }
            
            Message::StopPeer(id) => {
                let pm = self.peer_manager.clone();
                Command::perform(
                    async move {
                        let mut manager = pm.write().await;
                        if let Err(e) = manager.stop_peer(&id) {
                            tracing::error!("Failed to stop peer: {}", e);
                        }
                    },
                    |_| Message::RefreshData,
                )
            }
            
            Message::RemovePeer(id) => {
                let pm = self.peer_manager.clone();
                Command::perform(
                    async move {
                        let mut manager = pm.write().await;
                        if let Err(e) = manager.remove_peer(&id) {
                            tracing::error!("Failed to remove peer: {}", e);
                        }
                    },
                    |_| Message::RefreshData,
                )
            }
            
            Message::StartAllPeers => {
                let pm = self.peer_manager.clone();
                Command::perform(
                    async move {
                        let mut manager = pm.write().await;
                        manager.start_all();
                    },
                    |_| Message::RefreshData,
                )
            }
            
            Message::StopAllPeers => {
                let pm = self.peer_manager.clone();
                Command::perform(
                    async move {
                        let mut manager = pm.write().await;
                        manager.stop_all();
                    },
                    |_| Message::RefreshData,
                )
            }
            
            // Chat
            Message::ChatInputChanged(input) => {
                self.chat_input = input;
                Command::none()
            }
            
            Message::SendChat => {
                if self.chat_input.is_empty() {
                    return Command::none();
                }
                
                let msg = self.chat_input.clone();
                self.chat_input.clear();
                
                let state = self.state.clone();
                Command::perform(
                    async move {
                        let mut s = state.write().await;
                        s.send_chat_message(&msg).await;
                    },
                    |_| Message::RefreshData,
                )
            }
            
            // Settings
            Message::NameInputChanged(name) => {
                self.name_input = name;
                Command::none()
            }
            
            Message::SaveName => {
                let name = self.name_input.clone();
                let state = self.state.clone();
                Command::perform(
                    async move {
                        let mut s = state.write().await;
                        s.set_display_name(&name);
                    },
                    |_| Message::RefreshData,
                )
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let sidebar = self.view_sidebar();
        let content = match self.current_tab {
            Tab::Dashboard => self.view_dashboard(),
            Tab::Peers => self.view_peers(),
            Tab::Chat => self.view_chat(),
            Tab::Settings => self.view_settings(),
        };

        row![
            sidebar,
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(25)
        ]
        .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

impl CortexApp {
    fn view_sidebar(&self) -> Element<Message> {
        let running = self.cached_managed_peers.iter()
            .filter(|p| p.get("status").and_then(|s| s.as_str()) == Some("Running"))
            .count();
        
        let nav = column![
            self.nav_button("📊 Dashboard", Tab::Dashboard),
            self.nav_button(&format!("🖥️ My Peers ({})", self.cached_managed_peers.len()), Tab::Peers),
            self.nav_button("💬 Chat", Tab::Chat),
            self.nav_button("⚙️ Settings", Tab::Settings),
        ]
        .spacing(5);

        let status_text = if running > 0 {
            format!("🟢 {} peer{} running", running, if running > 1 { "s" } else { "" })
        } else {
            "🔴 No peers running".to_string()
        };

        container(
            column![
                text("🧠 CortexOS").size(22),
                text("Peer Control Panel").size(11),
                Space::with_height(15),
                nav,
                Space::with_height(Length::Fill),
                text(status_text).size(11),
            ]
            .spacing(10)
        )
        .width(Length::Fixed(200.0))
        .height(Length::Fill)
        .padding(15)
        .into()
    }

    fn nav_button(&self, label: &str, tab: Tab) -> Element<Message> {
        let is_selected = std::mem::discriminant(&self.current_tab) == std::mem::discriminant(&tab);
        
        button(text(label).size(13))
            .width(Length::Fill)
            .padding(10)
            .style(if is_selected { theme::Button::Primary } else { theme::Button::Secondary })
            .on_press(Message::SwitchTab(tab))
            .into()
    }

    fn view_dashboard(&self) -> Element<Message> {
        let device = self.cached_status.get("device");
        let stats = self.cached_status.get("stats");
        let ports = self.cached_status.get("ports");
        
        let score = device.and_then(|d| d.get("capacity_score")).and_then(|v| v.as_u64()).unwrap_or(0);
        let cpu = device.and_then(|d| d.get("cpu_model")).and_then(|v| v.as_str()).unwrap_or("-");
        let cores = device.and_then(|d| d.get("cpu_cores")).and_then(|v| v.as_u64()).unwrap_or(0);
        let ram = device.and_then(|d| d.get("ram_total_mb")).and_then(|v| v.as_u64()).unwrap_or(0);
        let gpu = device.and_then(|d| d.get("gpu")).and_then(|v| v.as_str()).unwrap_or("None");
        let layers = device.and_then(|d| d.get("max_layers")).and_then(|v| v.as_u64()).unwrap_or(0);
        
        let local_ip = self.cached_status.get("local_ip").and_then(|v| v.as_str()).unwrap_or("-");
        let node_id = self.cached_status.get("short_id").and_then(|v| v.as_str()).unwrap_or("-");
        
        let running_peers = self.cached_managed_peers.iter()
            .filter(|p| p.get("status").and_then(|s| s.as_str()) == Some("Running"))
            .count();
        let discovered_peers = self.cached_peers.len();

        // Stats cards
        let stats_row = row![
            self.stat_card("💪 Capacity", &format!("{}/100", score), "score"),
            self.stat_card("🖥️ My Peers", &format!("{} running", running_peers), "active"),
            self.stat_card("🌐 Network", &format!("{} discovered", discovered_peers), "peers"),
            self.stat_card("📊 Max Layers", &format!("{}", layers), "AI capacity"),
        ]
        .spacing(15);

        // Device info
        let device_info = column![
            text("💻 This Machine").size(16),
            text(format!("Node ID: {}", node_id)).size(12),
            text(format!("Local IP: {}", local_ip)).size(12),
            text(format!("CPU: {} ({} cores)", cpu, cores)).size(12),
            text(format!("RAM: {:.1} GB", ram as f64 / 1024.0)).size(12),
            text(format!("GPU: {}", gpu)).size(12),
        ]
        .spacing(6);

        // Quick actions
        let actions = row![
            button(text("▶ Start All Peers").size(12))
                .style(theme::Button::Positive)
                .padding(10)
                .on_press(Message::StartAllPeers),
            button(text("⏹ Stop All Peers").size(12))
                .style(theme::Button::Destructive)
                .padding(10)
                .on_press(Message::StopAllPeers),
            button(text("➕ Add Peer").size(12))
                .style(theme::Button::Primary)
                .padding(10)
                .on_press(Message::AddPeer),
        ]
        .spacing(10);

        column![
            text("Dashboard").size(24),
            Space::with_height(15),
            stats_row,
            Space::with_height(20),
            device_info,
            Space::with_height(20),
            text("Quick Actions").size(16),
            actions,
        ]
        .spacing(10)
        .into()
    }

    fn stat_card(&self, title: &str, value: &str, sub: &str) -> Element<Message> {
        container(
            column![
                text(title).size(11),
                text(value).size(24),
                text(sub).size(10),
            ]
            .spacing(4)
        )
        .padding(15)
        .width(Length::FillPortion(1))
        .into()
    }

    fn view_peers(&self) -> Element<Message> {
        let header = row![
            text("My Peers").size(24),
            Space::with_width(Length::Fill),
            button(text("➕ Add Peer").size(12))
                .style(theme::Button::Primary)
                .padding(8)
                .on_press(Message::AddPeer),
        ];

        let peer_cards: Vec<Element<Message>> = self.cached_managed_peers.iter().map(|peer| {
            let id = peer.get("id").and_then(|v| v.as_str()).unwrap_or("-").to_string();
            let name = peer.get("name").and_then(|v| v.as_str()).unwrap_or("Peer");
            let status = peer.get("status").and_then(|v| v.as_str()).unwrap_or("Stopped");
            let p2p_port = peer.get("p2p_port").and_then(|v| v.as_u64()).unwrap_or(0);
            let task_port = peer.get("task_port").and_then(|v| v.as_u64()).unwrap_or(0);
            let uptime = peer.get("uptime_seconds").and_then(|v| v.as_u64()).unwrap_or(0);
            let pid = peer.get("pid").and_then(|v| v.as_u64());
            let last_error = peer.get("last_error").and_then(|v| v.as_str());
            let logs = peer.get("logs").and_then(|v| v.as_array());
            
            let is_running = status == "Running";
            let is_error = status.contains("Error");
            let status_icon = if is_running { "🟢" } else if is_error { "🔴" } else { "⚪" };
            
            let uptime_str = if uptime > 0 {
                let h = uptime / 3600;
                let m = (uptime % 3600) / 60;
                let s = uptime % 60;
                format!("{:02}:{:02}:{:02}", h, m, s)
            } else {
                "-".to_string()
            };

            let id_for_start = id.clone();
            let id_for_stop = id.clone();
            let id_for_remove = id.clone();

            // Build logs display
            let logs_text: String = logs
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_else(|| "No logs".to_string());

            let mut peer_content = column![
                row![
                    text(format!("{} {}", status_icon, name)).size(16),
                    Space::with_width(Length::Fill),
                    text(status).size(12),
                ],
                text(format!("Ports: {} (P2P) / {} (Tasks)", p2p_port, task_port)).size(11),
                text(format!("Uptime: {} | PID: {}", uptime_str, pid.map(|p| p.to_string()).unwrap_or("-".to_string()))).size(11),
            ]
            .spacing(4);

            // Show error if any
            if let Some(err) = last_error {
                peer_content = peer_content.push(
                    text(format!("❌ Error: {}", err)).size(11)
                );
            }

            // Show recent logs
            if !logs_text.is_empty() && logs_text != "No logs" {
                peer_content = peer_content.push(Space::with_height(5));
                peer_content = peer_content.push(
                    text("📋 Logs:").size(10)
                );
                peer_content = peer_content.push(
                    container(
                        scrollable(text(logs_text).size(9))
                            .height(Length::Fixed(60.0))
                    )
                    .padding(5)
                );
            }

            peer_content = peer_content.push(Space::with_height(8));
            peer_content = peer_content.push(
                row![
                    if is_running {
                        button(text("⏹ Stop").size(11))
                            .style(theme::Button::Destructive)
                            .padding(6)
                            .on_press(Message::StopPeer(id_for_stop))
                    } else {
                        button(text("▶ Start").size(11))
                            .style(theme::Button::Positive)
                            .padding(6)
                            .on_press(Message::StartPeer(id_for_start))
                    },
                    Space::with_width(8),
                    if !is_running {
                        button(text("🗑 Remove").size(11))
                            .style(theme::Button::Secondary)
                            .padding(6)
                            .on_press(Message::RemovePeer(id_for_remove))
                    } else {
                        button(text("🗑 Remove").size(11))
                            .style(theme::Button::Secondary)
                            .padding(6)
                    },
                ]
            );

            container(peer_content)
                .padding(15)
                .width(Length::Fill)
                .into()
        }).collect();

        let peer_list = if peer_cards.is_empty() {
            column![
                Space::with_height(40),
                text("No peers configured").size(14),
                text("Click 'Add Peer' to create one").size(12),
            ]
            .width(Length::Fill)
        } else {
            Column::with_children(peer_cards).spacing(10)
        };

        column![
            header,
            Space::with_height(15),
            scrollable(peer_list).height(Length::Fill),
        ]
        .spacing(10)
        .into()
    }

    fn view_chat(&self) -> Element<Message> {
        let messages: Vec<Element<Message>> = self.cached_chat.iter().map(|m| {
            let content = m.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let from = m.get("from_name").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let is_mine = m.get("is_mine").and_then(|v| v.as_bool()).unwrap_or(false);
            let is_system = m.get("is_system").and_then(|v| v.as_bool()).unwrap_or(false);
            
            let msg_text = if is_system {
                text(content).size(12)
            } else if is_mine {
                text(format!("You: {}", content)).size(13)
            } else {
                text(format!("{}: {}", from, content)).size(13)
            };
            
            container(msg_text).padding(8).into()
        }).collect();

        let input_row = row![
            text_input("Type a message...", &self.chat_input)
                .on_input(Message::ChatInputChanged)
                .on_submit(Message::SendChat)
                .width(Length::Fill)
                .padding(10),
            button(text("Send").size(13))
                .padding(10)
                .on_press(Message::SendChat),
        ]
        .spacing(10);

        column![
            text("Chat").size(24),
            Space::with_height(10),
            scrollable(Column::with_children(messages).spacing(5)).height(Length::Fill),
            input_row,
        ]
        .spacing(10)
        .into()
    }

    fn view_settings(&self) -> Element<Message> {
        let node_id = self.cached_status.get("node_id").and_then(|v| v.as_str()).unwrap_or("-");
        let uptime = self.cached_status.get("stats")
            .and_then(|s| s.get("uptime_seconds"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        
        let h = uptime / 3600;
        let m = (uptime % 3600) / 60;
        let s = uptime % 60;

        let name_row = row![
            text("Display Name:").size(13),
            Space::with_width(10),
            text_input("Anonymous", &self.name_input)
                .on_input(Message::NameInputChanged)
                .width(Length::Fixed(200.0))
                .padding(8),
            Space::with_width(10),
            button(text("Save").size(12))
                .padding(8)
                .on_press(Message::SaveName),
        ];

        column![
            text("Settings").size(24),
            Space::with_height(20),
            name_row,
            Space::with_height(15),
            text(format!("App Node ID: {}...", &node_id[..20.min(node_id.len())])).size(12),
            text(format!("App Uptime: {:02}:{:02}:{:02}", h, m, s)).size(12),
            Space::with_height(20),
            text("About CortexOS").size(16),
            text("A free, community-powered, decentralized AI network.").size(12),
            text("Run peers on any device to contribute compute power.").size(12),
        ]
        .spacing(8)
        .into()
    }
}
