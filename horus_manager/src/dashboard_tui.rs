// Terminal UI Dashboard for HORUS
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use horus_core::core::{LogType, GLOBAL_LOG_BUFFER};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Tabs},
    Frame, Terminal,
};
use std::io::stdout;
use std::time::{Duration, Instant};

// Import the monitoring structs and functions
#[derive(Debug, Clone)]
pub struct NodeStatus {
    pub name: String,
    pub status: String,
    pub priority: u32,
    pub process_id: u32,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub publishers: Vec<String>,  // Topic names this node publishes to
    pub subscribers: Vec<String>, // Topic names this node subscribes from
}

#[derive(Clone)]
pub struct TuiDashboard {
    active_tab: Tab,
    selected_index: usize,
    scroll_offset: usize,

    // Data
    nodes: Vec<NodeStatus>,
    topics: Vec<TopicInfo>,
    params: std::sync::Arc<horus_core::RuntimeParams>,

    // State
    paused: bool,
    show_help: bool,
    last_update: Instant,

    // Log panel state
    show_log_panel: bool,
    panel_target: Option<LogPanelTarget>,
    panel_scroll_offset: usize,
}

#[derive(Debug, Clone)]
struct TopicInfo {
    name: String,
    msg_type: String,
    publishers: usize,
    subscribers: usize,
    rate: f32,
    publisher_nodes: Vec<String>,
    subscriber_nodes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
    Overview,
    Nodes,
    Topics,
    Packages,
    Parameters,
}

#[derive(Debug, Clone, PartialEq)]
enum LogPanelTarget {
    Node(String),
    Topic(String),
}

impl Tab {
    fn as_str(&self) -> &'static str {
        match self {
            Tab::Overview => "Overview",
            Tab::Nodes => "Nodes",
            Tab::Topics => "Topics",
            Tab::Packages => "Packages",
            Tab::Parameters => "Params",
        }
    }

    fn all() -> Vec<Tab> {
        vec![
            Tab::Overview,
            Tab::Nodes,
            Tab::Topics,
            Tab::Packages,
            Tab::Parameters,
        ]
    }
}

impl Default for TuiDashboard {
    fn default() -> Self {
        Self::new()
    }
}

impl TuiDashboard {
    pub fn new() -> Self {
        // Initialize real RuntimeParams
        let params = std::sync::Arc::new(
            horus_core::RuntimeParams::init()
                .unwrap_or_else(|_| horus_core::RuntimeParams::default()),
        );

        Self {
            active_tab: Tab::Overview,
            selected_index: 0,
            scroll_offset: 0,

            nodes: Vec::new(),
            topics: Vec::new(),
            params,

            paused: false,
            show_help: false,
            last_update: Instant::now(),

            show_log_panel: false,
            panel_target: None,
            panel_scroll_offset: 0,
        }
    }

    pub fn run() -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Create app and run
        let mut app = TuiDashboard::new();
        let res = app.run_app(&mut terminal);

        // Restore terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        if let Err(err) = res {
            eprintln!("Error: {:?}", err);
        }

        Ok(())
    }

    fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            // Update data if not paused
            if !self.paused && self.last_update.elapsed() > Duration::from_secs(1) {
                self.update_data()?;
                self.last_update = Instant::now();
            }

            // Draw UI
            terminal.draw(|f| self.draw_ui(f))?;

            // Handle input
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if self.show_help {
                        self.show_help = false;
                        continue;
                    }

                    // Check if Shift is pressed
                    let shift_pressed = key.modifiers.contains(KeyModifiers::SHIFT);

                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(()),

                        // ESC key: close log panel if open, otherwise do nothing
                        KeyCode::Esc => {
                            if self.show_log_panel {
                                self.show_log_panel = false;
                                self.panel_target = None;
                                self.panel_scroll_offset = 0;
                            }
                        }

                        // Enter key: open log panel for selected node/topic
                        KeyCode::Enter => {
                            if !self.show_log_panel {
                                self.open_log_panel();
                            }
                        }

                        KeyCode::Tab => self.next_tab(),
                        KeyCode::BackTab => self.prev_tab(),
                        KeyCode::Char('p') | KeyCode::Char('P') => self.paused = !self.paused,
                        KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::Char('H') => {
                            self.show_help = true
                        }

                        // Up/Down keys with different behavior based on Shift
                        KeyCode::Up => {
                            if shift_pressed && self.show_log_panel {
                                // Shift+Up: Navigate to previous node/topic and update log panel
                                self.select_prev();
                                self.update_log_panel_target();
                            } else if self.show_log_panel {
                                // Up: Scroll logs up
                                self.panel_scroll_offset =
                                    self.panel_scroll_offset.saturating_sub(1);
                            } else {
                                // Up: Navigate list
                                self.select_prev();
                            }
                        }
                        KeyCode::Down => {
                            if shift_pressed && self.show_log_panel {
                                // Shift+Down: Navigate to next node/topic and update log panel
                                self.select_next();
                                self.update_log_panel_target();
                            } else if self.show_log_panel {
                                // Down: Scroll logs down
                                self.panel_scroll_offset =
                                    self.panel_scroll_offset.saturating_add(1);
                            } else {
                                // Down: Navigate list
                                self.select_next();
                            }
                        }

                        KeyCode::PageUp => {
                            if self.show_log_panel {
                                self.panel_scroll_offset =
                                    self.panel_scroll_offset.saturating_sub(10);
                            } else {
                                self.scroll_up(10);
                            }
                        }
                        KeyCode::PageDown => {
                            if self.show_log_panel {
                                self.panel_scroll_offset =
                                    self.panel_scroll_offset.saturating_add(10);
                            } else {
                                self.scroll_down(10);
                            }
                        }

                        // Parameter operations (only in Parameters tab)
                        KeyCode::Char('r') | KeyCode::Char('R')
                            if self.active_tab == Tab::Parameters =>
                        {
                            // Refresh parameters from disk
                            self.params = std::sync::Arc::new(
                                horus_core::RuntimeParams::init()
                                    .unwrap_or_else(|_| horus_core::RuntimeParams::default()),
                            );
                        }
                        KeyCode::Char('s') | KeyCode::Char('S')
                            if self.active_tab == Tab::Parameters =>
                        {
                            // Save parameters to disk
                            let _ = self.params.save_to_disk();
                        }

                        _ => {}
                    }
                }
            }
        }
    }

    fn draw_ui(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Header (increased for status + tabs)
                Constraint::Min(0),    // Content
                Constraint::Length(2), // Footer
            ])
            .split(f.size());

        self.draw_header(f, chunks[0]);

        // Split content area horizontally if log panel is open
        let content_area = chunks[1];
        if self.show_log_panel {
            let horizontal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(25), // Narrow list showing only names
                    Constraint::Percentage(75), // Large log panel
                ])
                .split(content_area);

            // Draw simplified main content (only names)
            if self.show_help {
                self.draw_help(f, horizontal_chunks[0]);
            } else {
                match self.active_tab {
                    Tab::Overview => self.draw_overview(f, horizontal_chunks[0]),
                    Tab::Nodes => self.draw_nodes_simple(f, horizontal_chunks[0]),
                    Tab::Topics => self.draw_topics_simple(f, horizontal_chunks[0]),
                    Tab::Packages => self.draw_packages(f, horizontal_chunks[0]),
                    Tab::Parameters => self.draw_parameters(f, horizontal_chunks[0]),
                }
            }

            // Draw log panel
            self.draw_log_panel(f, horizontal_chunks[1]);
        } else {
            // Normal full-width content
            if self.show_help {
                self.draw_help(f, content_area);
            } else {
                match self.active_tab {
                    Tab::Overview => self.draw_overview(f, content_area),
                    Tab::Nodes => self.draw_nodes(f, content_area),
                    Tab::Topics => self.draw_topics(f, content_area),
                    Tab::Packages => self.draw_packages(f, content_area),
                    Tab::Parameters => self.draw_parameters(f, content_area),
                }
            }
        }

        self.draw_footer(f, chunks[2]);
    }

    fn draw_header(&self, f: &mut Frame, area: Rect) {
        // Create a block for the entire header area
        let header_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue));

        let inner_area = header_block.inner(area);
        f.render_widget(header_block, area);

        // Split the inner area into status line and tabs
        let header_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Status line
                Constraint::Length(1), // Tabs
            ])
            .split(inner_area);

        // Draw status line - exclude placeholder entries from count
        let node_count = if self.nodes.len() == 1 && self.nodes[0].name.contains("No HORUS nodes") {
            0
        } else {
            self.nodes.len()
        };
        let topic_count =
            if self.topics.len() == 1 && self.topics[0].name.contains("No active topics") {
                0
            } else {
                self.topics.len()
            };
        let status = if self.paused { "PAUSED" } else { "LIVE" };

        let status_text = vec![
            Span::styled(
                "HORUS TUI ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("v0.1.0 | "),
            Span::styled(
                status.to_string(),
                Style::default().fg(if self.paused {
                    Color::Yellow
                } else {
                    Color::Green
                }),
            ),
            Span::raw(" | Nodes: "),
            Span::styled(format!("{}", node_count), Style::default().fg(Color::Green)),
            Span::raw(" | Topics: "),
            Span::styled(format!("{}", topic_count), Style::default().fg(Color::Cyan)),
        ];

        let status_line = Paragraph::new(Line::from(status_text)).alignment(Alignment::Center);
        f.render_widget(status_line, header_chunks[0]);

        // Draw tabs
        let titles: Vec<Line> = Tab::all()
            .iter()
            .map(|t| Line::from(vec![Span::raw(t.as_str())]))
            .collect();

        let selected = Tab::all()
            .iter()
            .position(|&t| t == self.active_tab)
            .unwrap();

        let tabs = Tabs::new(titles)
            .select(selected)
            .style(Style::default().fg(Color::Gray))
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .divider(Span::raw(" | "));

        f.render_widget(tabs, header_chunks[1]);
    }

    fn draw_overview(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50), // Nodes summary
                Constraint::Percentage(50), // Topics summary
            ])
            .split(area);

        // Active Nodes Summary (top 10)
        self.draw_node_summary(f, chunks[0]);

        // Active Topics Summary (top 10)
        self.draw_topic_summary(f, chunks[1]);
    }

    fn draw_node_summary(&self, f: &mut Frame, area: Rect) {
        let rows = self.nodes.iter().take(10).map(|node| {
            let is_running = node.status == "active";
            let status_symbol = if is_running { "●" } else { "○" };
            let status_color = if is_running { Color::Green } else { Color::Red };

            Row::new(vec![
                Cell::from(status_symbol).style(Style::default().fg(status_color)),
                Cell::from(node.name.clone()),
                Cell::from(node.process_id.to_string()),
                Cell::from(format!("{} MB", node.memory_usage / 1024 / 1024)),
            ])
        });

        let table = Table::new(rows)
            .header(
                Row::new(vec!["", "Name", "PID", "Memory"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(
                Block::default()
                    .title(format!("Active Nodes ({})", self.nodes.len()))
                    .borders(Borders::ALL),
            )
            .widths(&[
                Constraint::Length(2),
                Constraint::Min(30),
                Constraint::Length(8),
                Constraint::Length(12),
            ]);

        f.render_widget(table, area);
    }

    fn draw_topic_summary(&self, f: &mut Frame, area: Rect) {
        let rows = self.topics.iter().take(10).map(|topic| {
            // Format node names compactly
            let pub_count = topic.publishers;
            let sub_count = topic.subscribers;
            let pub_label = if pub_count > 0 {
                format!(
                    "{}:{}",
                    pub_count,
                    topic.publisher_nodes.first().unwrap_or(&"-".to_string())
                )
            } else {
                "-".to_string()
            };
            let sub_label = if sub_count > 0 {
                format!(
                    "{}:{}",
                    sub_count,
                    topic.subscriber_nodes.first().unwrap_or(&"-".to_string())
                )
            } else {
                "-".to_string()
            };

            Row::new(vec![
                Cell::from(topic.name.clone()),
                Cell::from(topic.msg_type.clone()),
                Cell::from(pub_label).style(Style::default().fg(Color::Green)),
                Cell::from(sub_label).style(Style::default().fg(Color::Blue)),
                Cell::from(format!("{:.1} Hz", topic.rate)),
            ])
        });

        let table = Table::new(rows)
            .header(
                Row::new(vec![
                    "Topic",
                    "Type",
                    "Pub (N:Node)",
                    "Sub (N:Node)",
                    "Rate",
                ])
                .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(
                Block::default()
                    .title(format!("Active Topics ({})", self.topics.len()))
                    .borders(Borders::ALL),
            )
            .widths(&[
                Constraint::Percentage(30),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Length(10),
            ]);

        f.render_widget(table, area);
    }

    fn draw_topics_simple(&self, f: &mut Frame, area: Rect) {
        // Simplified view showing only topic names
        let rows: Vec<Row> = self
            .topics
            .iter()
            .map(|topic| {
                let has_activity = topic.publishers > 0 || topic.subscribers > 0;
                let status_symbol = if has_activity { "●" } else { "○" };
                let status_color = if has_activity {
                    Color::Cyan
                } else {
                    Color::DarkGray
                };

                Row::new(vec![
                    Cell::from(status_symbol).style(Style::default().fg(status_color)),
                    Cell::from(topic.name.clone()),
                ])
            })
            .collect();

        let table = Table::new(rows)
            .header(
                Row::new(vec!["", "Topic Name"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(Block::default().title("Topics").borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("► ")
            .widths(&[Constraint::Length(2), Constraint::Min(10)]);

        // Create table state with current selection
        let mut table_state = TableState::default();
        if !self.topics.is_empty() {
            let selected = self.selected_index.min(self.topics.len() - 1);
            table_state.select(Some(selected));
        }

        f.render_stateful_widget(table, area, &mut table_state);
    }

    fn draw_topics(&self, f: &mut Frame, area: Rect) {
        let rows: Vec<Row> = self
            .topics
            .iter()
            .map(|topic| {
                // Format publisher and subscriber node names
                let pub_nodes = if topic.publishers == 0 {
                    "-".to_string()
                } else {
                    topic.publisher_nodes.join(", ")
                };

                let sub_nodes = if topic.subscribers == 0 {
                    "-".to_string()
                } else {
                    topic.subscriber_nodes.join(", ")
                };

                Row::new(vec![
                    Cell::from(topic.name.clone()),
                    Cell::from(topic.msg_type.clone()),
                    Cell::from(format!("{:.1}", topic.rate)),
                    Cell::from(pub_nodes).style(Style::default().fg(Color::Green)),
                    Cell::from(sub_nodes).style(Style::default().fg(Color::Blue)),
                ])
            })
            .collect();

        let table = Table::new(rows)
            .header(
                Row::new(vec!["Topic", "Type", "Hz", "Publishers", "Subscribers"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(
                Block::default()
                    .title("Topics - Use ↑↓ to select, Enter to view logs")
                    .borders(Borders::ALL),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("► ")
            .widths(&[
                Constraint::Percentage(25),
                Constraint::Percentage(20),
                Constraint::Length(8),
                Constraint::Percentage(27),
                Constraint::Percentage(28),
            ]);

        // Create table state with current selection
        let mut table_state = TableState::default();
        if !self.topics.is_empty() {
            // Clamp selected_index to valid range
            let selected = self.selected_index.min(self.topics.len() - 1);
            table_state.select(Some(selected));
        }

        f.render_stateful_widget(table, area, &mut table_state);
    }

    fn draw_packages(&self, f: &mut Frame, area: Rect) {
        // Get installed packages from cache
        let packages = get_installed_packages();

        let rows = packages.iter().map(|(name, version, size)| {
            Row::new(vec![
                Cell::from(name.clone()),
                Cell::from(version.clone()),
                Cell::from(size.clone()),
            ])
        });

        let table = Table::new(rows)
            .header(
                Row::new(vec!["Package", "Version", "Size"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(
                Block::default()
                    .title("Installed Packages")
                    .borders(Borders::ALL),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .widths(&[
                Constraint::Min(30),
                Constraint::Length(15),
                Constraint::Length(12),
            ]);

        f.render_widget(table, area);
    }

    fn draw_parameters(&self, f: &mut Frame, area: Rect) {
        // Get REAL runtime parameters from RuntimeParams
        let params_map = self.params.get_all();

        let params: Vec<_> = params_map
            .iter()
            .map(|(key, value)| {
                // Determine type from value
                let type_str = match value {
                    serde_json::Value::Number(_) => "number",
                    serde_json::Value::String(_) => "string",
                    serde_json::Value::Bool(_) => "bool",
                    serde_json::Value::Array(_) => "array",
                    serde_json::Value::Object(_) => "object",
                    serde_json::Value::Null => "null",
                };

                // Format value for display
                let value_str = match value {
                    serde_json::Value::String(s) => s.clone(),
                    _ => value.to_string(),
                };

                (key.clone(), value_str, type_str.to_string())
            })
            .collect();

        let rows = params
            .iter()
            .enumerate()
            .map(|(idx, (name, value, type_))| {
                let is_selected = idx == self.selected_index && self.active_tab == Tab::Parameters;
                let style = if is_selected {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    Cell::from(name.clone()).style(Style::default().fg(Color::Cyan)),
                    Cell::from(value.clone()),
                    Cell::from(type_.clone()).style(Style::default().fg(Color::Yellow)),
                ])
                .style(style)
            });

        let help_text = if params.is_empty() {
            "No parameters set. Press 'a' to add"
        } else {
            "[a] Add | [e] Edit | [d] Delete | [s] Save | [r] Refresh"
        };

        let table = Table::new(rows)
            .header(
                Row::new(vec!["Parameter", "Value", "Type"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(
                Block::default()
                    .title(format!(
                        "Runtime Parameters ({}) - {}",
                        params.len(),
                        help_text
                    ))
                    .borders(Borders::ALL),
            )
            .widths(&[
                Constraint::Percentage(35),
                Constraint::Percentage(50),
                Constraint::Percentage(15),
            ]);

        f.render_widget(table, area);
    }

    fn draw_nodes_simple(&self, f: &mut Frame, area: Rect) {
        // Simplified view showing only node names
        let rows: Vec<Row> = self
            .nodes
            .iter()
            .map(|node| {
                let is_running = node.status == "active";
                let status_symbol = if is_running { "●" } else { "○" };
                let status_color = if is_running { Color::Green } else { Color::Red };

                Row::new(vec![
                    Cell::from(status_symbol).style(Style::default().fg(status_color)),
                    Cell::from(node.name.clone()),
                ])
            })
            .collect();

        let table = Table::new(rows)
            .header(
                Row::new(vec!["", "Node Name"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(Block::default().title("Nodes").borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("► ")
            .widths(&[Constraint::Length(2), Constraint::Min(10)]);

        // Create table state with current selection
        let mut table_state = TableState::default();
        if !self.nodes.is_empty() {
            let selected = self.selected_index.min(self.nodes.len() - 1);
            table_state.select(Some(selected));
        }

        f.render_stateful_widget(table, area, &mut table_state);
    }

    fn draw_nodes(&self, f: &mut Frame, area: Rect) {
        let rows: Vec<Row> = self
            .nodes
            .iter()
            .map(|node| {
                let is_running = node.status == "active";
                let status = if is_running { "Running" } else { "Stopped" };
                let status_color = if is_running { Color::Green } else { Color::Red };

                // Format publishers and subscribers compactly
                let pubs = if node.publishers.is_empty() {
                    "-".to_string()
                } else {
                    node.publishers.join(", ")
                };

                let subs = if node.subscribers.is_empty() {
                    "-".to_string()
                } else {
                    node.subscribers.join(", ")
                };

                Row::new(vec![
                    Cell::from(node.name.clone()),
                    Cell::from(node.process_id.to_string()),
                    Cell::from(format!("{:.1}%", node.cpu_usage)),
                    Cell::from(format!("{} MB", node.memory_usage / 1024 / 1024)),
                    Cell::from(status).style(Style::default().fg(status_color)),
                    Cell::from(pubs).style(Style::default().fg(Color::Green)),
                    Cell::from(subs).style(Style::default().fg(Color::Blue)),
                ])
            })
            .collect();

        let table = Table::new(rows)
            .header(
                Row::new(vec![
                    "Name",
                    "PID",
                    "CPU",
                    "Memory",
                    "Status",
                    "Publishes",
                    "Subscribes",
                ])
                .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(
                Block::default()
                    .title("Node Details - Use ↑↓ to select, Enter to view logs")
                    .borders(Borders::ALL),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("► ")
            .widths(&[
                Constraint::Percentage(15),
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Percentage(24),
                Constraint::Percentage(25),
            ]);

        // Create table state with current selection
        let mut table_state = TableState::default();
        if !self.nodes.is_empty() {
            // Clamp selected_index to valid range
            let selected = self.selected_index.min(self.nodes.len() - 1);
            table_state.select(Some(selected));
        }

        f.render_stateful_widget(table, area, &mut table_state);
    }

    fn draw_help(&self, f: &mut Frame, area: Rect) {
        let help_text = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "HORUS Terminal Dashboard - Help",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Navigation:",
                Style::default().fg(Color::Cyan),
            )]),
            Line::from("  Tab        - Next tab (Overview → Nodes → Topics)"),
            Line::from("  Shift+Tab  - Previous tab"),
            Line::from("  ↑/↓        - Navigate lists"),
            Line::from("  PgUp/PgDn  - Scroll quickly"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Actions:",
                Style::default().fg(Color::Cyan),
            )]),
            Line::from(
                "  Enter      - Open log panel for selected node/topic (in Nodes/Topics tabs)",
            ),
            Line::from("  ESC        - Close log panel"),
            Line::from("  Shift+↑↓   - Switch between nodes/topics while log panel is open"),
            Line::from("  p          - Pause/Resume updates"),
            Line::from("  q          - Quit dashboard"),
            Line::from("  ?/h        - Show this help"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Tabs:",
                Style::default().fg(Color::Cyan),
            )]),
            Line::from("  Overview   - Summary of nodes and topics"),
            Line::from("  Nodes      - Full list of detected HORUS nodes"),
            Line::from("  Topics     - Full list of shared memory topics"),
            Line::from("  Packages   - Installed HORUS packages"),
            Line::from("  Params     - Runtime configuration parameters"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Parameter Actions:",
                Style::default().fg(Color::Cyan),
            )]),
            Line::from("  r          - Refresh parameters from disk"),
            Line::from("  s          - Save parameters to disk"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Data Source: ", Style::default().fg(Color::Yellow)),
                Span::raw("Real-time from HORUS detect backend"),
            ]),
            Line::from("  • Nodes from /proc scan + registry"),
            Line::from("  • Topics from /dev/shm/horus/topics/"),
            Line::from("  • Params from ~/.horus/params.yaml (RuntimeParams)"),
            Line::from(""),
            Line::from("Press any key to close this help..."),
        ];

        let help = Paragraph::new(help_text)
            .block(
                Block::default()
                    .title("Help")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .alignment(Alignment::Left);

        f.render_widget(help, area);
    }

    fn draw_log_panel(&self, f: &mut Frame, area: Rect) {
        // Get logs based on panel target
        let (title, logs) = match &self.panel_target {
            Some(LogPanelTarget::Node(node_name)) => {
                let logs = GLOBAL_LOG_BUFFER.get_for_node(node_name);
                (format!("Logs: {}", node_name), logs)
            }
            Some(LogPanelTarget::Topic(topic_name)) => {
                let logs = GLOBAL_LOG_BUFFER.get_for_topic(topic_name);
                (format!("Logs: {}", topic_name), logs)
            }
            None => ("Logs".to_string(), Vec::new()),
        };

        // Format logs as lines
        let log_lines: Vec<Line> = if logs.is_empty() {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No logs available",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Logs will appear here when the node/topic",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    "starts publishing or subscribing",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        } else {
            logs.iter()
                .skip(self.panel_scroll_offset)
                .map(|entry| {
                    // Color based on log type
                    let (type_str, type_color) = match entry.log_type {
                        LogType::Publish => ("PUB", Color::Green),
                        LogType::Subscribe => ("SUB", Color::Blue),
                        LogType::Info => ("INFO", Color::Cyan),
                        LogType::Warning => ("WARN", Color::Yellow),
                        LogType::Error => ("ERR", Color::Red),
                        LogType::Debug => ("DBG", Color::Magenta),
                        LogType::TopicRead => ("READ", Color::Blue),
                        LogType::TopicWrite => ("WRITE", Color::Green),
                        LogType::TopicMap => ("MAP", Color::Cyan),
                        LogType::TopicUnmap => ("UNMAP", Color::DarkGray),
                        LogType::RemoteDeploy => ("DEPLOY", Color::Magenta),
                        LogType::RemoteCompile => ("COMPILE", Color::Magenta),
                        LogType::RemoteExecute => ("EXEC", Color::Magenta),
                    };

                    // Format: [TIME] TYPE topic: message
                    let time_str = if let Some(time_part) = entry.timestamp.split('T').nth(1) {
                        time_part.split('.').next().unwrap_or(&entry.timestamp)
                    } else {
                        &entry.timestamp
                    };

                    let mut spans = vec![
                        Span::styled(
                            format!("[{}] ", time_str),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            format!("{:<6} ", type_str),
                            Style::default().fg(type_color).add_modifier(Modifier::BOLD),
                        ),
                    ];

                    // Add topic if present
                    if let Some(topic) = &entry.topic {
                        spans.push(Span::styled(
                            format!("{}: ", topic),
                            Style::default().fg(Color::Cyan),
                        ));
                    }

                    // Add message
                    spans.push(Span::raw(&entry.message));

                    Line::from(spans)
                })
                .collect()
        };

        let help_text = format!("Showing {} logs | ↑↓ Scroll | ESC Close", logs.len());

        // Create block with title
        let block = Block::default()
            .title(Line::from(vec![Span::styled(
                if let Some(target) = &self.panel_target {
                    match target {
                        LogPanelTarget::Node(name) => format!("Node: {}", name),
                        LogPanelTarget::Topic(name) => format!("Topic: {}", name),
                    }
                } else {
                    "Logs".to_string()
                },
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let panel = Paragraph::new(log_lines)
            .block(block)
            .alignment(Alignment::Left);

        f.render_widget(panel, area);

        // Draw help text at bottom
        let help_area = Rect {
            x: area.x + 1,
            y: area.y + area.height - 2,
            width: area.width.saturating_sub(2),
            height: 1,
        };

        let help_paragraph = Paragraph::new(Line::from(vec![Span::styled(
            help_text,
            Style::default().fg(Color::DarkGray),
        )]));

        f.render_widget(help_paragraph, help_area);
    }

    fn draw_footer(&self, f: &mut Frame, area: Rect) {
        let footer_text = if self.show_help {
            "Press any key to close help"
        } else if self.show_log_panel {
            "[ESC] Close | [↑↓] Scroll Logs | [Shift+↑↓] Switch Node/Topic | [Q] Quit"
        } else if self.active_tab == Tab::Parameters {
            "[ENTER] View Logs | [TAB] Switch Tab | [R] Refresh | [S] Save | [P] Pause | [?] Help | [Q] Quit"
        } else if self.active_tab == Tab::Nodes || self.active_tab == Tab::Topics {
            "[ENTER] View Logs | [TAB] Switch Tab | [Q] Quit | [P] Pause | [?] Help"
        } else {
            "[TAB] Switch Tab | [Q] Quit | [P] Pause | [?] Help"
        };

        let footer = Paragraph::new(footer_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));

        f.render_widget(footer, area);
    }

    fn update_data(&mut self) -> Result<()> {
        // Update nodes from detect backend
        if let Ok(nodes) = get_active_nodes() {
            self.nodes = nodes;
        }

        // Update topics from detect backend
        if let Ok(topics) = get_active_topics() {
            self.topics = topics;
        }

        Ok(())
    }

    fn next_tab(&mut self) {
        let tabs = Tab::all();
        let current = tabs.iter().position(|&t| t == self.active_tab).unwrap();
        self.active_tab = tabs[(current + 1) % tabs.len()];
        self.selected_index = 0;
    }

    fn prev_tab(&mut self) {
        let tabs = Tab::all();
        let current = tabs.iter().position(|&t| t == self.active_tab).unwrap();
        self.active_tab = tabs[if current == 0 {
            tabs.len() - 1
        } else {
            current - 1
        }];
        self.selected_index = 0;
    }

    fn select_next(&mut self) {
        // Get max index based on current tab
        let max_index = match self.active_tab {
            Tab::Nodes => self.nodes.len().saturating_sub(1),
            Tab::Topics => self.topics.len().saturating_sub(1),
            Tab::Parameters => {
                let params_map = self.params.get_all();
                params_map.len().saturating_sub(1)
            }
            _ => 0,
        };

        if self.selected_index < max_index {
            self.selected_index += 1;
        }
    }

    fn select_prev(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
    }

    fn open_log_panel(&mut self) {
        match self.active_tab {
            Tab::Nodes => {
                // Open panel for selected node
                if self.selected_index < self.nodes.len() {
                    let node = &self.nodes[self.selected_index];
                    // Don't open panel for placeholder entries
                    if !node.name.contains("No HORUS nodes") {
                        self.panel_target = Some(LogPanelTarget::Node(node.name.clone()));
                        self.show_log_panel = true;
                        self.panel_scroll_offset = 0;
                    }
                }
            }
            Tab::Topics => {
                // Open panel for selected topic
                if self.selected_index < self.topics.len() {
                    let topic = &self.topics[self.selected_index];
                    // Don't open panel for placeholder entries
                    if !topic.name.contains("No active topics") {
                        self.panel_target = Some(LogPanelTarget::Topic(topic.name.clone()));
                        self.show_log_panel = true;
                        self.panel_scroll_offset = 0;
                    }
                }
            }
            _ => {
                // Log panel not supported for other tabs
            }
        }
    }

    fn update_log_panel_target(&mut self) {
        // Update the log panel to show logs for the currently selected node/topic
        // This is called when using Shift+Up/Down to navigate while panel is open
        match self.active_tab {
            Tab::Nodes => {
                if self.selected_index < self.nodes.len() {
                    let node = &self.nodes[self.selected_index];
                    // Don't update for placeholder entries
                    if !node.name.contains("No HORUS nodes") {
                        self.panel_target = Some(LogPanelTarget::Node(node.name.clone()));
                        self.panel_scroll_offset = 0; // Reset scroll when switching
                    }
                }
            }
            Tab::Topics => {
                if self.selected_index < self.topics.len() {
                    let topic = &self.topics[self.selected_index];
                    // Don't update for placeholder entries
                    if !topic.name.contains("No active topics") {
                        self.panel_target = Some(LogPanelTarget::Topic(topic.name.clone()));
                        self.panel_scroll_offset = 0; // Reset scroll when switching
                    }
                }
            }
            _ => {}
        }
    }
}

// Unified backend functions using monitor module

fn get_active_nodes() -> Result<Vec<NodeStatus>> {
    // Use unified backend from monitor module
    let discovered_nodes = crate::commands::monitor::discover_nodes().unwrap_or_default();

    if discovered_nodes.is_empty() {
        // Show demo data if no real nodes detected
        Ok(vec![NodeStatus {
            name: "No HORUS nodes detected".to_string(),
            status: "inactive".to_string(),
            cpu_usage: 0.0,
            memory_usage: 0,
            process_id: 0,
            priority: 0,
            publishers: Vec::new(),
            subscribers: Vec::new(),
        }])
    } else {
        Ok(discovered_nodes
            .into_iter()
            .map(|n| NodeStatus {
                name: n.name.clone(),
                status: if n.status == "Running" {
                    "active".to_string()
                } else {
                    "inactive".to_string()
                },
                cpu_usage: n.cpu_usage,
                memory_usage: n.memory_usage,
                process_id: n.process_id,
                priority: n.priority,
                publishers: n.publishers.iter().map(|p| p.topic.clone()).collect(),
                subscribers: n.subscribers.iter().map(|s| s.topic.clone()).collect(),
            })
            .collect())
    }
}

fn get_installed_packages() -> Vec<(String, String, String)> {
    let mut packages = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Check local .horus/cache first (project-specific)
    let local_cache = std::env::current_dir().ok().map(|d| d.join(".horus/cache"));

    if let Some(ref cache_dir) = local_cache {
        if cache_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(cache_dir) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if seen.insert(name.to_string()) {
                            let size = entry
                                .metadata()
                                .map(|m| {
                                    let kb = m.len() / 1024;
                                    if kb < 1024 {
                                        format!("{} KB", kb)
                                    } else {
                                        format!("{:.1} MB", kb as f64 / 1024.0)
                                    }
                                })
                                .unwrap_or_else(|_| "Unknown".to_string());

                            packages.push((
                                format!("{} (local)", name),
                                "latest".to_string(),
                                size,
                            ));
                        }
                    }
                }
            }
        }
    }

    // Check global ~/.horus/cache (system-wide)
    let global_cache = dirs::home_dir().map(|h| h.join(".horus/cache"));

    if let Some(ref cache_dir) = global_cache {
        if cache_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(cache_dir) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if seen.insert(name.to_string()) {
                            let size = entry
                                .metadata()
                                .map(|m| {
                                    let kb = m.len() / 1024;
                                    if kb < 1024 {
                                        format!("{} KB", kb)
                                    } else {
                                        format!("{:.1} MB", kb as f64 / 1024.0)
                                    }
                                })
                                .unwrap_or_else(|_| "Unknown".to_string());

                            packages.push((
                                format!("{} (global)", name),
                                "latest".to_string(),
                                size,
                            ));
                        }
                    }
                }
            }
        }
    }

    if packages.is_empty() {
        vec![(
            "No packages found".to_string(),
            "-".to_string(),
            "-".to_string(),
        )]
    } else {
        packages.sort_by(|a, b| a.0.cmp(&b.0));
        packages
    }
}

// Removed: get_runtime_parameters() - now using real RuntimeParams from horus_core

fn get_active_topics() -> Result<Vec<TopicInfo>> {
    // Use unified backend from monitor module
    let discovered_topics = crate::commands::monitor::discover_shared_memory().unwrap_or_default();

    if discovered_topics.is_empty() {
        // Show helpful message if no topics detected
        Ok(vec![TopicInfo {
            name: "No active topics".to_string(),
            msg_type: "N/A".to_string(),
            publishers: 0,
            subscribers: 0,
            rate: 0.0,
            publisher_nodes: Vec::new(),
            subscriber_nodes: Vec::new(),
        }])
    } else {
        Ok(discovered_topics
            .into_iter()
            .map(|t| {
                // Shorten type names for readability
                let short_type = t
                    .message_type
                    .as_ref()
                    .map(|ty| ty.split("::").last().unwrap_or(ty).to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                TopicInfo {
                    name: t.topic_name,
                    msg_type: short_type,
                    publishers: t.publishers.len(),
                    subscribers: t.subscribers.len(),
                    rate: t.message_rate_hz,
                    publisher_nodes: t.publishers,
                    subscriber_nodes: t.subscribers,
                }
            })
            .collect())
    }
}
