//! CortexOS Desktop Application
//! 
//! A unified distributed AI peer - like uTorrent for AI compute.
//! 
//! One app = One peer = Full functionality:
//! - AI Chat → queries go to distributed LLM swarm
//! - Queue → see who's processing your work & who you're helping
//! - Settings → port, NAT, discovery, compute limits
//! - Network → connected peers, bandwidth stats

mod state;

use iced::widget::{button, column, container, row, scrollable, text, text_input, toggler, Column, Space};
use iced::{executor, theme, Application, Command, Element, Length, Settings, Theme};
use state::AppState;
use std::sync::Arc;
use tokio::sync::RwLock;

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();
    
    CortexApp::run(Settings {
        window: iced::window::Settings {
            size: iced::Size::new(900.0, 700.0),
            min_size: Some(iced::Size::new(700.0, 500.0)),
            ..Default::default()
        },
        ..Default::default()
    })
}

#[derive(Debug, Clone)]
enum Tab {
    Chat,
    Queue,
    Network,
    Settings,
}

#[derive(Debug, Clone)]
enum Message {
    SwitchTab(Tab),
    RefreshData,
    DataRefreshed(serde_json::Value),
    
    // AI Chat
    ChatInputChanged(String),
    SendToAI,
    AIResponseReceived(String),
    
    // Settings
    ToggleContribute(bool),
    ToggleOpenToWorld(bool),
    PortChanged(String),
    MaxCpuChanged(String),
    DisplayNameChanged(String),
    SaveSettings,
}

struct CortexApp {
    state: Arc<RwLock<AppState>>,
    current_tab: Tab,
    
    // Chat
    chat_input: String,
    ai_processing: bool,
    
    // Settings (local copies for editing)
    settings_port: String,
    settings_max_cpu: String,
    settings_name: String,
    settings_contribute: bool,
    settings_open_to_world: bool,
    
    // Cached data
    cached_data: serde_json::Value,
}

impl Application for CortexApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let state = Arc::new(RwLock::new(AppState::new()));
        
        // Start the peer services
        let state_clone = state.clone();
        tokio::spawn(async move {
            let mut s = state_clone.write().await;
            if let Err(e) = s.start().await {
                tracing::error!("Failed to start peer: {}", e);
            }
        });

        (
            Self {
                state,
                current_tab: Tab::Chat,
                chat_input: String::new(),
                ai_processing: false,
                settings_port: "7654".to_string(),
                settings_max_cpu: "80".to_string(),
                settings_name: "Anonymous".to_string(),
                settings_contribute: true,
                settings_open_to_world: false,
                cached_data: serde_json::json!({}),
            },
            Command::perform(async {}, |_| Message::RefreshData),
        )
    }

    fn title(&self) -> String {
        String::from("CortexOS - Distributed AI")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SwitchTab(tab) => {
                self.current_tab = tab;
                Command::none()
            }
            
            Message::RefreshData => {
                let state = self.state.clone();
                Command::perform(
                    async move {
                        let mut s = state.write().await;
                        s.refresh().await;
                        s.to_json().await
                    },
                    Message::DataRefreshed,
                )
            }
            
            Message::DataRefreshed(data) => {
                self.cached_data = data;
                
                // Schedule next refresh
                Command::perform(
                    async { tokio::time::sleep(tokio::time::Duration::from_millis(500)).await },
                    |_| Message::RefreshData,
                )
            }
            
            // AI Chat
            Message::ChatInputChanged(input) => {
                self.chat_input = input;
                Command::none()
            }
            
            Message::SendToAI => {
                if self.chat_input.is_empty() || self.ai_processing {
                    return Command::none();
                }
                
                let query = self.chat_input.clone();
                self.chat_input.clear();
                self.ai_processing = true;
                
                let state = self.state.clone();
                Command::perform(
                    async move {
                        let mut s = state.write().await;
                        s.send_ai_query(&query).await
                    },
                    Message::AIResponseReceived,
                )
            }
            
            Message::AIResponseReceived(response) => {
                self.ai_processing = false;
                let state = self.state.clone();
                Command::perform(
                    async move {
                        let mut s = state.write().await;
                        s.add_ai_response(&response).await;
                    },
                    |_| Message::RefreshData,
                )
            }
            
            // Settings
            Message::ToggleContribute(enabled) => {
                self.settings_contribute = enabled;
                Command::none()
            }
            
            Message::ToggleOpenToWorld(enabled) => {
                self.settings_open_to_world = enabled;
                Command::none()
            }
            
            Message::PortChanged(port) => {
                self.settings_port = port;
                Command::none()
            }
            
            Message::MaxCpuChanged(cpu) => {
                self.settings_max_cpu = cpu;
                Command::none()
            }
            
            Message::DisplayNameChanged(name) => {
                self.settings_name = name;
                Command::none()
            }
            
            Message::SaveSettings => {
                let state = self.state.clone();
                let port = self.settings_port.parse().unwrap_or(7654);
                let max_cpu = self.settings_max_cpu.parse().unwrap_or(80);
                let name = self.settings_name.clone();
                let contribute = self.settings_contribute;
                let open = self.settings_open_to_world;
                
                Command::perform(
                    async move {
                        let mut s = state.write().await;
                        s.update_settings(port, max_cpu, &name, contribute, open).await;
                    },
                    |_| Message::RefreshData,
                )
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let sidebar = self.view_sidebar();
        let content = match self.current_tab {
            Tab::Chat => self.view_chat(),
            Tab::Queue => self.view_queue(),
            Tab::Network => self.view_network(),
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
        let peers = self.cached_data.get("network")
            .and_then(|n| n.get("peers_count"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        
        let queue_in = self.cached_data.get("queue")
            .and_then(|q| q.get("processing_for_me"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let queue_out = self.cached_data.get("queue")
            .and_then(|q| q.get("helping_others"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let nav = column![
            self.nav_button("🧠 AI Chat", Tab::Chat),
            self.nav_button(&format!("📊 Queue (↓{} ↑{})", queue_in, queue_out), Tab::Queue),
            self.nav_button(&format!("🌐 Network ({})", peers), Tab::Network),
            self.nav_button("⚙️ Settings", Tab::Settings),
        ]
        .spacing(5);

        let status = if self.settings_contribute {
            "🟢 Contributing"
        } else {
            "🔴 Not sharing"
        };

        container(
            column![
                text("🧠 CortexOS").size(22),
                text("Distributed AI").size(11),
                Space::with_height(20),
                nav,
                Space::with_height(Length::Fill),
                text(status).size(11),
            ]
            .spacing(10)
        )
        .width(Length::Fixed(180.0))
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

    fn view_chat(&self) -> Element<Message> {
        let messages = self.cached_data.get("chat")
            .and_then(|c| c.as_array())
            .map(|arr| arr.iter().map(|m| {
                let role = m.get("role").and_then(|v| v.as_str()).unwrap_or("system");
                let content = m.get("content").and_then(|v| v.as_str()).unwrap_or("");
                let is_user = role == "user";
                
                container(
                    column![
                        text(if is_user { "You" } else { "🧠 AI (Swarm)" }).size(11),
                        text(content).size(14),
                    ]
                    .spacing(4)
                )
                .padding(10)
                .width(Length::Fill)
                .into()
            }).collect::<Vec<Element<Message>>>())
            .unwrap_or_default();

        let chat_list = if messages.is_empty() {
            column![
                Space::with_height(50),
                text("Ask the distributed AI swarm anything!").size(16),
                text("Your query will be processed across multiple peers.").size(12),
            ]
        } else {
            Column::with_children(messages).spacing(10)
        };

        let input_row = row![
            text_input("Ask the AI swarm...", &self.chat_input)
                .on_input(Message::ChatInputChanged)
                .on_submit(Message::SendToAI)
                .width(Length::Fill)
                .padding(12),
            button(
                text(if self.ai_processing { "⏳" } else { "Send" }).size(14)
            )
            .padding(12)
            .on_press(Message::SendToAI),
        ]
        .spacing(10);

        let processing_hint = if self.ai_processing {
            text("🔄 Processing across the swarm...").size(11)
        } else {
            text("").size(11)
        };

        column![
            text("AI Chat").size(24),
            text("Queries processed by the distributed LLM swarm").size(12),
            Space::with_height(15),
            scrollable(chat_list).height(Length::Fill),
            processing_hint,
            input_row,
        ]
        .spacing(10)
        .into()
    }

    fn view_queue(&self) -> Element<Message> {
        let queue = self.cached_data.get("queue");
        
        let for_me = queue.and_then(|q| q.get("items_for_me"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);
        let helping = queue.and_then(|q| q.get("items_helping"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);
        let my_local = queue.and_then(|q| q.get("my_local"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);

        // Stats row (like uTorrent)
        let stats = row![
            self.queue_stat("↓ For Me", for_me, "chunks"),
            self.queue_stat("↑ Helping", helping, "chunks"),
            self.queue_stat("◉ Local", my_local, "chunks"),
        ]
        .spacing(20);

        // Queue items
        let items = queue.and_then(|q| q.get("all_items"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().map(|item| {
                let task_id = item.get("task_id").and_then(|v| v.as_str()).unwrap_or("-");
                let direction = item.get("direction").and_then(|v| v.as_str()).unwrap_or("unknown");
                let progress = item.get("progress").and_then(|v| v.as_u64()).unwrap_or(0);
                let peer = item.get("peer").and_then(|v| v.as_str()).unwrap_or("-");
                let layers = item.get("layers").and_then(|v| v.as_str()).unwrap_or("-");
                
                let icon = match direction {
                    "for_me" => "↓",
                    "helping" => "↑",
                    _ => "◉",
                };
                
                container(
                    row![
                        text(icon).size(16).width(Length::Fixed(25.0)),
                        column![
                            text(&task_id[..8.min(task_id.len())]).size(12),
                            text(format!("Peer: {} | Layers: {}", &peer[..8.min(peer.len())], layers)).size(10),
                        ],
                        Space::with_width(Length::Fill),
                        text(format!("{}%", progress)).size(14),
                    ]
                    .spacing(10)
                )
                .padding(8)
                .into()
            }).collect::<Vec<Element<Message>>>())
            .unwrap_or_default();

        let queue_list = if items.is_empty() {
            column![
                Space::with_height(30),
                text("Queue is empty").size(14),
                text("Send an AI query or wait for peers to request compute").size(11),
            ]
        } else {
            Column::with_children(items).spacing(5)
        };

        column![
            text("Queue").size(24),
            text("Work being processed - like uTorrent for AI").size(12),
            Space::with_height(15),
            stats,
            Space::with_height(15),
            scrollable(queue_list).height(Length::Fill),
        ]
        .spacing(10)
        .into()
    }

    fn queue_stat(&self, label: &str, value: usize, unit: &str) -> Element<Message> {
        container(
            column![
                text(label).size(11),
                text(format!("{}", value)).size(28),
                text(unit).size(10),
            ]
            .spacing(2)
        )
        .padding(15)
        .width(Length::FillPortion(1))
        .into()
    }

    fn view_network(&self) -> Element<Message> {
        let network = self.cached_data.get("network");
        
        let peers_count = network.and_then(|n| n.get("peers_count"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let uploaded = network.and_then(|n| n.get("bytes_sent"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let downloaded = network.and_then(|n| n.get("bytes_received"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let stats = row![
            self.network_stat("🌐 Peers", &format!("{}", peers_count)),
            self.network_stat("↑ Sent", &format_bytes(uploaded)),
            self.network_stat("↓ Received", &format_bytes(downloaded)),
        ]
        .spacing(20);

        let peer_list = network.and_then(|n| n.get("peers"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().map(|peer| {
                let node_id = peer.get("node_id").and_then(|v| v.as_str()).unwrap_or("-");
                let addr = peer.get("address").and_then(|v| v.as_str()).unwrap_or("-");
                let score = peer.get("score").and_then(|v| v.as_u64()).unwrap_or(0);
                
                container(
                    row![
                        text("●").size(10),
                        Space::with_width(8),
                        column![
                            text(&node_id[..12.min(node_id.len())]).size(12),
                            text(addr).size(10),
                        ],
                        Space::with_width(Length::Fill),
                        text(format!("Score: {}", score)).size(11),
                    ]
                )
                .padding(8)
                .into()
            }).collect::<Vec<Element<Message>>>())
            .unwrap_or_default();

        let peers_view = if peer_list.is_empty() {
            column![
                text("No peers connected").size(14),
                text("Discovering peers on the network...").size(11),
            ]
        } else {
            Column::with_children(peer_list).spacing(5)
        };

        let device = self.cached_data.get("device");
        let cpu = device.and_then(|d| d.get("cpu")).and_then(|v| v.as_str()).unwrap_or("-");
        let ram = device.and_then(|d| d.get("ram_mb")).and_then(|v| v.as_u64()).unwrap_or(0);
        let score = device.and_then(|d| d.get("score")).and_then(|v| v.as_u64()).unwrap_or(0);

        column![
            text("Network").size(24),
            Space::with_height(15),
            stats,
            Space::with_height(20),
            text("Connected Peers").size(16),
            scrollable(peers_view).height(Length::FillPortion(1)),
            Space::with_height(15),
            text("My Device").size(16),
            text(format!("CPU: {} | RAM: {} GB | Score: {}/100", cpu, ram / 1024, score)).size(12),
        ]
        .spacing(10)
        .into()
    }

    fn network_stat(&self, label: &str, value: &str) -> Element<Message> {
        container(
            column![
                text(label).size(11),
                text(value).size(20),
            ]
            .spacing(4)
        )
        .padding(15)
        .width(Length::FillPortion(1))
        .into()
    }

    fn view_settings(&self) -> Element<Message> {
        let node_id = self.cached_data.get("node_id")
            .and_then(|v| v.as_str())
            .unwrap_or("-");

        column![
            text("Settings").size(24),
            Space::with_height(20),
            
            // Compute contribution
            row![
                column![
                    text("Share Compute").size(14),
                    text("Help process AI tasks for others").size(11),
                ],
                Space::with_width(Length::Fill),
                toggler(String::new(), self.settings_contribute, Message::ToggleContribute),
            ],
            Space::with_height(15),
            
            // Open to world
            row![
                column![
                    text("Open to Internet").size(14),
                    text("Accept connections from outside your network").size(11),
                ],
                Space::with_width(Length::Fill),
                toggler(String::new(), self.settings_open_to_world, Message::ToggleOpenToWorld),
            ],
            Space::with_height(20),
            
            // Port
            row![
                text("Port:").size(13).width(Length::Fixed(100.0)),
                text_input("7654", &self.settings_port)
                    .on_input(Message::PortChanged)
                    .width(Length::Fixed(100.0))
                    .padding(8),
            ],
            Space::with_height(10),
            
            // Max CPU
            row![
                text("Max CPU %:").size(13).width(Length::Fixed(100.0)),
                text_input("80", &self.settings_max_cpu)
                    .on_input(Message::MaxCpuChanged)
                    .width(Length::Fixed(100.0))
                    .padding(8),
            ],
            Space::with_height(10),
            
            // Display name
            row![
                text("Name:").size(13).width(Length::Fixed(100.0)),
                text_input("Anonymous", &self.settings_name)
                    .on_input(Message::DisplayNameChanged)
                    .width(Length::Fixed(200.0))
                    .padding(8),
            ],
            Space::with_height(20),
            
            button(text("Save Settings").size(13))
                .padding(10)
                .on_press(Message::SaveSettings),
            
            Space::with_height(Length::Fill),
            text(format!("Node ID: {}", node_id)).size(11),
        ]
        .spacing(8)
        .into()
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
