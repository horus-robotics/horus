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

    // Parameter editing state
    param_edit_mode: ParamEditMode,
    param_input_key: String,
    param_input_value: String,
    param_input_focus: ParamInputFocus,

    // Package navigation state
    package_view_mode: PackageViewMode,
    package_panel_focus: PackagePanelFocus,
    selected_workspace: Option<WorkspaceData>,

    // Workspace caching (to avoid repeated filesystem operations)
    workspace_cache: Vec<WorkspaceData>,
    workspace_cache_time: Instant,
    current_workspace_path: Option<std::path::PathBuf>,

    // Graph view state
    graph_nodes: Vec<TuiGraphNode>,
    graph_edges: Vec<TuiGraphEdge>,
    graph_layout: GraphLayout,
    graph_zoom: f32,
    graph_offset_x: i32,
    graph_offset_y: i32,
}

#[derive(Debug, Clone, PartialEq)]
enum ParamEditMode {
    None,
    Add,
    Edit(String),   // Stores the original key being edited
    Delete(String), // Stores the key to delete
}

#[derive(Debug, Clone, PartialEq)]
enum ParamInputFocus {
    Key,
    Value,
}

#[derive(Debug, Clone, PartialEq)]
enum PackageViewMode {
    List,             // Viewing all workspaces
    WorkspaceDetails, // Viewing packages inside a workspace
}

#[derive(Debug, Clone, PartialEq)]
enum PackagePanelFocus {
    LocalWorkspaces, // Focused on local workspaces panel
    GlobalPackages,  // Focused on global packages panel
}

#[derive(Debug, Clone)]
struct WorkspaceData {
    name: String,
    path: String,
    packages: Vec<PackageData>,
    is_current: bool, // True if this is the current workspace (detected via find_workspace_root)
}

#[derive(Debug, Clone)]
struct PackageData {
    name: String,
    version: String,
    installed_packages: Vec<(String, String)>, // (name, version) pairs
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

// Graph data structures for TUI
#[derive(Debug, Clone)]
struct TuiGraphNode {
    id: String,
    label: String,
    node_type: TuiNodeType,
    x: i32,  // TUI coordinates (character cells)
    y: i32,
    pid: Option<u32>,
    active: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum TuiNodeType {
    Process,
    Topic,
}

#[derive(Debug, Clone)]
struct TuiGraphEdge {
    from: String,
    to: String,
    edge_type: TuiEdgeType,
    active: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum TuiEdgeType {
    Publish,   // Process publishes to topic
    Subscribe, // Process subscribes from topic
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum GraphLayout {
    Hierarchical,  // Processes on left, topics on right
    Vertical,      // Processes on top, topics on bottom
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
    Overview,
    Nodes,
    Topics,
    Graph,
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
            Tab::Graph => "Graph",
            Tab::Packages => "Packages",
            Tab::Parameters => "Params",
        }
    }

    fn all() -> Vec<Tab> {
        vec![
            Tab::Overview,
            Tab::Nodes,
            Tab::Topics,
            Tab::Graph,
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

        // Detect current workspace on startup
        let current_workspace_path = crate::workspace::find_workspace_root();

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

            param_edit_mode: ParamEditMode::None,
            param_input_key: String::new(),
            param_input_value: String::new(),
            param_input_focus: ParamInputFocus::Key,

            package_view_mode: PackageViewMode::List,
            package_panel_focus: PackagePanelFocus::LocalWorkspaces,
            selected_workspace: None,

            // Initialize workspace cache as empty (will load on first access)
            workspace_cache: Vec::new(),
            workspace_cache_time: Instant::now() - Duration::from_secs(10), // Force initial load
            current_workspace_path,

            graph_nodes: Vec::new(),
            graph_edges: Vec::new(),
            graph_layout: GraphLayout::Hierarchical,
            graph_zoom: 1.0,
            graph_offset_x: 0,
            graph_offset_y: 0,
        }
    }

    /// Refresh workspace cache if stale (5 second TTL)
    fn refresh_workspace_cache_if_needed(&mut self) {
        const CACHE_TTL: Duration = Duration::from_secs(5);

        if self.workspace_cache_time.elapsed() > CACHE_TTL {
            self.workspace_cache = get_local_workspaces(&self.current_workspace_path);
            self.workspace_cache_time = Instant::now();
        }
    }

    /// Force refresh of workspace cache (e.g., on manual refresh)
    fn force_refresh_workspace_cache(&mut self) {
        self.workspace_cache = get_local_workspaces(&self.current_workspace_path);
        self.workspace_cache_time = Instant::now();
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
            // Update data if not paused (250ms refresh for real-time feel)
            if !self.paused && self.last_update.elapsed() > Duration::from_millis(250) {
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

                        // ESC key: navigate back in packages, close log panel, or cancel edit mode
                        KeyCode::Esc => {
                            if self.param_edit_mode != ParamEditMode::None {
                                // Cancel parameter editing
                                self.param_edit_mode = ParamEditMode::None;
                                self.param_input_key.clear();
                                self.param_input_value.clear();
                            } else if self.active_tab == Tab::Packages
                                && self.package_view_mode == PackageViewMode::WorkspaceDetails
                            {
                                // Navigate back to workspace list
                                self.package_view_mode = PackageViewMode::List;
                                self.package_panel_focus = PackagePanelFocus::LocalWorkspaces;
                                self.selected_workspace = None;
                                self.selected_index = 0;
                            } else if self.show_log_panel {
                                self.show_log_panel = false;
                                self.panel_target = None;
                                self.panel_scroll_offset = 0;
                            }
                        }

                        // Enter key: navigate packages or open log panel
                        KeyCode::Enter if self.param_edit_mode == ParamEditMode::None => {
                            if self.active_tab == Tab::Packages {
                                self.handle_package_enter();
                            } else if !self.show_log_panel {
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
                            if self.active_tab == Tab::Graph && !self.show_log_panel {
                                // Pan up in graph view
                                self.graph_offset_y += 2;
                            } else if shift_pressed && self.show_log_panel {
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
                            if self.active_tab == Tab::Graph && !self.show_log_panel {
                                // Pan down in graph view
                                self.graph_offset_y -= 2;
                            } else if shift_pressed && self.show_log_panel {
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
                            if self.active_tab == Tab::Parameters
                                && self.param_edit_mode == ParamEditMode::None =>
                        {
                            // Refresh parameters from disk
                            self.params = std::sync::Arc::new(
                                horus_core::RuntimeParams::init()
                                    .unwrap_or_else(|_| horus_core::RuntimeParams::default()),
                            );
                        }
                        KeyCode::Char('s') | KeyCode::Char('S')
                            if self.active_tab == Tab::Parameters
                                && self.param_edit_mode == ParamEditMode::None =>
                        {
                            // Save parameters to disk
                            let _ = self.params.save_to_disk();
                        }
                        KeyCode::Char('a') | KeyCode::Char('A')
                            if self.active_tab == Tab::Parameters
                                && self.param_edit_mode == ParamEditMode::None =>
                        {
                            // Start adding a new parameter
                            self.param_edit_mode = ParamEditMode::Add;
                            self.param_input_key.clear();
                            self.param_input_value.clear();
                            self.param_input_focus = ParamInputFocus::Key;
                        }
                        KeyCode::Char('e') | KeyCode::Char('E')
                            if self.active_tab == Tab::Parameters
                                && self.param_edit_mode == ParamEditMode::None =>
                        {
                            // Start editing selected parameter
                            self.start_edit_parameter();
                        }
                        KeyCode::Char('d') | KeyCode::Char('D')
                            if self.active_tab == Tab::Parameters
                                && self.param_edit_mode == ParamEditMode::None =>
                        {
                            // Delete selected parameter (with confirmation)
                            self.start_delete_parameter();
                        }

                        // Graph operations (only in Graph tab)
                        KeyCode::Char('l') | KeyCode::Char('L')
                            if self.active_tab == Tab::Graph =>
                        {
                            // Toggle layout
                            self.graph_layout = match self.graph_layout {
                                GraphLayout::Hierarchical => GraphLayout::Vertical,
                                GraphLayout::Vertical => GraphLayout::Hierarchical,
                            };
                            self.apply_graph_layout();
                        }
                        KeyCode::Char('+') | KeyCode::Char('=')
                            if self.active_tab == Tab::Graph =>
                        {
                            // Zoom in
                            self.graph_zoom = (self.graph_zoom * 1.2).min(5.0);
                        }
                        KeyCode::Char('-') | KeyCode::Char('_')
                            if self.active_tab == Tab::Graph =>
                        {
                            // Zoom out
                            self.graph_zoom = (self.graph_zoom / 1.2).max(0.2);
                        }
                        KeyCode::Left
                            if self.active_tab == Tab::Graph && !self.show_log_panel =>
                        {
                            // Pan left
                            self.graph_offset_x += 5;
                        }
                        KeyCode::Right
                            if self.active_tab == Tab::Graph && !self.show_log_panel =>
                        {
                            // Pan right
                            self.graph_offset_x -= 5;
                        }

                        // Switch between Local/Global panels in Packages tab
                        KeyCode::Left
                            if self.active_tab == Tab::Packages
                                && self.package_view_mode == PackageViewMode::List
                                && !self.show_log_panel =>
                        {
                            self.package_panel_focus = PackagePanelFocus::LocalWorkspaces;
                            self.selected_index = 0;
                        }
                        KeyCode::Right
                            if self.active_tab == Tab::Packages
                                && self.package_view_mode == PackageViewMode::List
                                && !self.show_log_panel =>
                        {
                            self.package_panel_focus = PackagePanelFocus::GlobalPackages;
                            self.selected_index = 0;
                        }

                        // Handle input when in parameter edit mode
                        KeyCode::Char(c) if self.param_edit_mode != ParamEditMode::None => {
                            match self.param_edit_mode {
                                ParamEditMode::Add | ParamEditMode::Edit(_) => {
                                    match self.param_input_focus {
                                        ParamInputFocus::Key => self.param_input_key.push(c),
                                        ParamInputFocus::Value => self.param_input_value.push(c),
                                    }
                                }
                                ParamEditMode::Delete(_) => {
                                    // In delete confirmation, 'y' confirms, 'n' or ESC cancels
                                    if c == 'y' || c == 'Y' {
                                        self.confirm_delete_parameter();
                                    } else if c == 'n' || c == 'N' {
                                        self.param_edit_mode = ParamEditMode::None;
                                    }
                                }
                                _ => {}
                            }
                        }
                        KeyCode::Backspace if self.param_edit_mode != ParamEditMode::None => {
                            match self.param_edit_mode {
                                ParamEditMode::Add | ParamEditMode::Edit(_) => {
                                    match self.param_input_focus {
                                        ParamInputFocus::Key => {
                                            self.param_input_key.pop();
                                        }
                                        ParamInputFocus::Value => {
                                            self.param_input_value.pop();
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        KeyCode::Enter if self.param_edit_mode != ParamEditMode::None => {
                            match &self.param_edit_mode {
                                ParamEditMode::Add => {
                                    if self.param_input_focus == ParamInputFocus::Key {
                                        // Move to value input
                                        self.param_input_focus = ParamInputFocus::Value;
                                    } else {
                                        // Confirm add
                                        self.confirm_add_parameter();
                                    }
                                }
                                ParamEditMode::Edit(_) => {
                                    if self.param_input_focus == ParamInputFocus::Key {
                                        // Move to value input
                                        self.param_input_focus = ParamInputFocus::Value;
                                    } else {
                                        // Confirm edit
                                        self.confirm_edit_parameter();
                                    }
                                }
                                ParamEditMode::Delete(_) => {
                                    // Enter confirms delete
                                    self.confirm_delete_parameter();
                                }
                                _ => {}
                            }
                        }

                        _ => {}
                    }
                }
            }
        }
    }

    fn draw_ui(&mut self, f: &mut Frame) {
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
                    Tab::Graph => self.draw_graph(f, horizontal_chunks[0]),
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
                    Tab::Graph => self.draw_graph(f, content_area),
                    Tab::Packages => self.draw_packages(f, content_area),
                    Tab::Parameters => self.draw_parameters(f, content_area),
                }
            }
        }

        self.draw_footer(f, chunks[2]);

        // Draw parameter edit dialog overlay if in edit mode
        if self.param_edit_mode != ParamEditMode::None {
            self.draw_param_edit_dialog(f);
        }
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
        let node_count = self.get_active_node_count();
        let topic_count = self.get_active_topic_count();
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
                    .title(format!("Active Nodes ({})", self.get_active_node_count()))
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
                    .title(format!("Active Topics ({})", self.get_active_topic_count()))
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
            .highlight_symbol(" ")
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
                    .title("Topics - Use  to select, Enter to view logs")
                    .borders(Borders::ALL),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(" ")
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

    fn draw_graph(&self, f: &mut Frame, area: Rect) {
        use std::collections::HashMap;

        // Create a block for the graph
        let block = Block::default()
            .title(format!(
                "Graph - {} nodes, {} edges | ● Process → [Topic] → ● Subscriber | Layout: {:?} | [L]ayout [+/-] Zoom [←↑↓→] Pan",
                self.graph_nodes.len(),
                self.graph_edges.len(),
                self.graph_layout
            ))
            .borders(Borders::ALL);

        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.graph_nodes.is_empty() {
            // Show message if no graph data
            let text = Paragraph::new("No nodes or topics detected. Start some HORUS nodes to see the graph.")
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center);
            f.render_widget(text, inner);
            return;
        }

        // Create a buffer to draw on
        let width = inner.width as usize;
        let height = inner.height as usize;
        let mut canvas: Vec<Vec<String>> = vec![vec![" ".to_string(); width]; height];

        // Create a map of node ID to node for quick lookup
        let node_map: HashMap<String, &TuiGraphNode> =
            self.graph_nodes.iter().map(|n| (n.id.clone(), n)).collect();

        // Draw edges first (so they appear behind nodes)
        for edge in &self.graph_edges {
            if let (Some(from_node), Some(to_node)) =
                (node_map.get(&edge.from), node_map.get(&edge.to))
            {
                self.draw_edge(
                    &mut canvas,
                    from_node,
                    to_node,
                    &edge.edge_type,
                    edge.active,
                    width,
                    height,
                );
            }
        }

        // Draw nodes on top of edges
        for node in &self.graph_nodes {
            self.draw_node(&mut canvas, node, width, height);
        }

        // Convert canvas to ratatui Lines
        let lines: Vec<Line> = canvas
            .iter()
            .map(|row| {
                let spans: Vec<Span> = row
                    .iter()
                    .map(|cell| Span::raw(cell.clone()))
                    .collect();
                Line::from(spans)
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        f.render_widget(paragraph, inner);
    }

    fn draw_node(&self, canvas: &mut [Vec<String>], node: &TuiGraphNode, width: usize, height: usize) {
        // Apply zoom and offset
        let x = ((node.x as f32 * self.graph_zoom) as i32 + self.graph_offset_x) as usize;
        let y = ((node.y as f32 * self.graph_zoom) as i32 + self.graph_offset_y) as usize;

        // Check bounds
        if y >= height {
            return;
        }

        match node.node_type {
            TuiNodeType::Process => {
                // Draw process as a circle using ● or ◉
                let symbol = if node.active { "●" } else { "○" };
                if x < width {
                    canvas[y][x] = symbol.to_string();
                }

                // Draw label next to the node
                let label = format!(" {}", node.label);
                for (i, ch) in label.chars().enumerate() {
                    let label_x = x + i + 1;
                    if label_x < width && y < height {
                        canvas[y][label_x] = ch.to_string();
                    }
                }
            }
            TuiNodeType::Topic => {
                // Draw topic as a box [topic_name]
                let label = format!("[{}]", node.label);
                for (i, ch) in label.chars().enumerate() {
                    let label_x = x + i;
                    if label_x < width && y < height {
                        canvas[y][label_x] = ch.to_string();
                    }
                }
            }
        }
    }

    fn draw_edge(
        &self,
        canvas: &mut [Vec<String>],
        from: &TuiGraphNode,
        to: &TuiGraphNode,
        edge_type: &TuiEdgeType,
        _active: bool,
        width: usize,
        height: usize,
    ) {
        // Apply zoom and offset
        let x1 = ((from.x as f32 * self.graph_zoom) as i32 + self.graph_offset_x) as usize;
        let y1 = ((from.y as f32 * self.graph_zoom) as i32 + self.graph_offset_y) as usize;
        let x2 = ((to.x as f32 * self.graph_zoom) as i32 + self.graph_offset_x) as usize;
        let y2 = ((to.y as f32 * self.graph_zoom) as i32 + self.graph_offset_y) as usize;

        // Draw a simple line using box-drawing characters
        if y1 == y2 {
            // Horizontal line
            let (start_x, end_x) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
            for x in start_x..=end_x {
                if x < width && y1 < height {
                    canvas[y1][x] = "─".to_string();
                }
            }

            // Add arrowhead pointing toward destination (to)
            // Arrow direction is based on actual data flow: from -> to
            // Place arrow one position before destination so it doesn't get overwritten by the node
            let (arrow, arrow_x) = if x1 < x2 {
                ("→", x2.saturating_sub(1))  // Left to right: arrow points right
            } else {
                ("←", x2 + 1)  // Right to left: arrow points left
            };
            if arrow_x < width && y2 < height && arrow_x != x2 {
                canvas[y2][arrow_x] = arrow.to_string();
            }
        } else if x1 == x2 {
            // Vertical line
            let (start_y, end_y) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
            for y in start_y..=end_y {
                if x1 < width && y < height {
                    canvas[y][x1] = "│".to_string();
                }
            }

            // Add arrowhead pointing toward destination (to)
            // Place arrow one position before destination so it doesn't get overwritten by the node
            let (arrow, arrow_y) = if y1 < y2 {
                ("↓", y2.saturating_sub(1))  // Top to bottom: arrow points down
            } else {
                ("↑", y2 + 1)  // Bottom to top: arrow points up
            };
            if x2 < width && arrow_y < height && arrow_y != y2 {
                canvas[arrow_y][x2] = arrow.to_string();
            }
        } else {
            // Diagonal or complex line - draw L-shaped connector
            // First horizontal, then vertical
            for x in x1.min(x2)..=x1.max(x2) {
                if x < width && y1 < height {
                    canvas[y1][x] = "─".to_string();
                }
            }

            for y in y1.min(y2)..=y1.max(y2) {
                if x2 < width && y < height {
                    canvas[y][x2] = "│".to_string();
                }
            }

            // Corner piece
            if x2 < width && y1 < height {
                let corner = if x1 < x2 && y1 < y2 {
                    "┐"
                } else if x1 < x2 && y1 > y2 {
                    "┘"
                } else if x1 > x2 && y1 < y2 {
                    "┌"
                } else {
                    "└"
                };
                canvas[y1][x2] = corner.to_string();
            }

            // Add arrowhead at destination pointing toward final position
            // For L-shaped connectors, the final segment is vertical
            // Place arrow one position before destination so it doesn't get overwritten by the node
            let (arrow, arrow_y) = if y1 < y2 {
                ("↓", y2.saturating_sub(1))  // Arrow points down to destination
            } else {
                ("↑", y2 + 1)  // Arrow points up to destination
            };
            if x2 < width && arrow_y < height && arrow_y != y2 {
                canvas[arrow_y][x2] = arrow.to_string();
            }
        }
    }

    fn draw_packages(&mut self, f: &mut Frame, area: Rect) {
        match self.package_view_mode {
            PackageViewMode::List => self.draw_workspace_list(f, area),
            PackageViewMode::WorkspaceDetails => self.draw_workspace_details(f, area),
        }
    }

    fn draw_workspace_list(&mut self, f: &mut Frame, area: Rect) {
        // Refresh workspace cache if needed (5 second TTL instead of every frame)
        self.refresh_workspace_cache_if_needed();

        let workspaces = &self.workspace_cache;
        let (_, global_packages) = get_installed_packages();

        // Split the area into two sections: workspaces (top) and global (bottom)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50), // Local workspaces
                Constraint::Percentage(50), // Global packages
            ])
            .split(area);

        // Determine which panel is focused
        let local_focused = self.package_panel_focus == PackagePanelFocus::LocalWorkspaces;
        let global_focused = self.package_panel_focus == PackagePanelFocus::GlobalPackages;

        // Draw workspaces table
        let workspace_rows: Vec<Row> = workspaces
            .iter()
            .enumerate()
            .map(|(idx, workspace)| {
                let is_selected = local_focused && idx == self.selected_index;

                // Build workspace name with current marker
                let workspace_display = if workspace.is_current {
                    format!("➜ {} (current)", workspace.name)
                } else {
                    workspace.name.clone()
                };

                // Style: selected gets reversed, current workspace gets green color
                let style = if is_selected {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else if workspace.is_current {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    Cell::from(workspace_display),
                    Cell::from(workspace.packages.len().to_string()),
                    Cell::from(workspace.path.clone()),
                ])
                .style(style)
            })
            .collect();

        let workspace_table = Table::new(workspace_rows)
            .header(
                Row::new(vec!["Workspace", "Packages", "Path"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(
                Block::default()
                    .title(format!(
                        "Local Workspaces ({}) {}",
                        workspaces.len(),
                        if local_focused { "[FOCUSED - Press ← →]" } else { "[Press → to focus]" }
                    ))
                    .borders(Borders::ALL)
                    .border_style(if local_focused {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }),
            )
            .widths(&[
                Constraint::Length(25),
                Constraint::Length(10),
                Constraint::Min(30),
            ]);

        f.render_widget(workspace_table, chunks[0]);

        // Draw global packages table with selection support
        let global_rows: Vec<Row> = global_packages
            .iter()
            .enumerate()
            .map(|(idx, (name, version, size))| {
                let is_selected = global_focused && idx == self.selected_index;
                let style = if is_selected {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    Cell::from(name.clone()),
                    Cell::from(version.clone()),
                    Cell::from(size.clone()),
                ])
                .style(style)
            })
            .collect();

        let global_table = Table::new(global_rows)
            .header(
                Row::new(vec!["Package", "Version", "Size"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(
                Block::default()
                    .title(format!(
                        "Global Packages ({}) {}",
                        global_packages.len(),
                        if global_focused { "[FOCUSED - Press ← →]" } else { "[Press ← to focus]" }
                    ))
                    .borders(Borders::ALL)
                    .border_style(if global_focused {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }),
            )
            .widths(&[
                Constraint::Min(30),
                Constraint::Length(15),
                Constraint::Length(12),
            ]);

        f.render_widget(global_table, chunks[1]);
    }

    fn draw_workspace_details(&self, f: &mut Frame, area: Rect) {
        if let Some(ref workspace) = self.selected_workspace {
            // Display packages inside the selected workspace
            let rows: Vec<Row> = workspace
                .packages
                .iter()
                .enumerate()
                .map(|(idx, pkg)| {
                    let is_selected = idx == self.selected_index;
                    let style = if is_selected {
                        Style::default().add_modifier(Modifier::REVERSED)
                    } else {
                        Style::default()
                    };

                    // Format nested packages as a comma-separated list
                    let installed = if pkg.installed_packages.is_empty() {
                        "-".to_string()
                    } else {
                        pkg.installed_packages
                            .iter()
                            .map(|(name, _)| name.clone())
                            .collect::<Vec<_>>()
                            .join(", ")
                    };

                    Row::new(vec![
                        Cell::from(pkg.name.clone()).style(Style::default().fg(Color::Cyan)),
                        Cell::from(pkg.version.clone()),
                        Cell::from(pkg.installed_packages.len().to_string()),
                        Cell::from(installed),
                    ])
                    .style(style)
                })
                .collect();

            let table = Table::new(rows)
                .header(
                    Row::new(vec!["Package", "Version", "Deps", "Installed Packages"])
                        .style(Style::default().add_modifier(Modifier::BOLD)),
                )
                .block(
                    Block::default()
                        .title(format!(
                            "Workspace: {} ({} packages) - Press Esc to return",
                            workspace.name,
                            workspace.packages.len()
                        ))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Green)),
                )
                .widths(&[
                    Constraint::Length(25),
                    Constraint::Length(12),
                    Constraint::Length(6),
                    Constraint::Min(30),
                ]);

            f.render_widget(table, area);
        } else {
            // Fallback: No workspace selected
            let block = Block::default()
                .title("No workspace selected - Press Esc to return")
                .borders(Borders::ALL);
            f.render_widget(block, area);
        }
    }

    fn draw_parameters(&self, f: &mut Frame, area: Rect) {
        // Get REAL runtime parameters from RuntimeParams
        let params_map = self.params.get_all();

        let params: Vec<_> = params_map
            .iter()
            .map(|(key, value)| {
                // Determine type from value using string matching to avoid version conflicts
                let type_str = if value.is_number() {
                    "number"
                } else if value.is_string() {
                    "string"
                } else if value.is_boolean() {
                    "bool"
                } else if value.is_array() {
                    "array"
                } else if value.is_object() {
                    "object"
                } else {
                    "null"
                };

                // Format value for display
                let value_str = if let Some(s) = value.as_str() {
                    s.to_string()
                } else {
                    value.to_string()
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
            .highlight_symbol(" ")
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
                    .title("Node Details - Use  to select, Enter to view logs")
                    .borders(Borders::ALL),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(" ")
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
            Line::from("  Tab        - Next tab (Overview  Nodes  Topics  Graph  Packages  Params)"),
            Line::from("  Shift+Tab  - Previous tab"),
            Line::from("  /        - Navigate lists"),
            Line::from("  PgUp/PgDn  - Scroll quickly"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "General Actions:",
                Style::default().fg(Color::Cyan),
            )]),
            Line::from("  p          - Pause/Resume updates"),
            Line::from("  q          - Quit dashboard"),
            Line::from("  ?/h        - Show this help"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Nodes/Topics Tab:",
                Style::default().fg(Color::Cyan),
            )]),
            Line::from("  Enter      - Open log panel for selected node/topic"),
            Line::from("  ESC        - Close log panel"),
            Line::from("  Shift+   - Switch between nodes/topics while log panel is open"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Graph Tab:",
                Style::default().fg(Color::Cyan),
            )]),
            Line::from("  L          - Toggle layout (Hierarchical/Vertical)"),
            Line::from("  +/-        - Zoom in/out"),
            Line::from("  Arrow keys - Pan the graph view"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Packages Tab:",
                Style::default().fg(Color::Cyan),
            )]),
            Line::from("  ← →        - Switch between Local Workspaces and Global Packages"),
            Line::from("  Enter      - Drill into selected workspace to view packages"),
            Line::from("  ESC        - Navigate back to workspace list"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Parameters Tab:",
                Style::default().fg(Color::Cyan),
            )]),
            Line::from("  a          - Add new parameter"),
            Line::from("  e          - Edit selected parameter"),
            Line::from("  d          - Delete selected parameter"),
            Line::from("  r          - Refresh parameters from disk"),
            Line::from("  s          - Save parameters to disk"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Tab Descriptions:",
                Style::default().fg(Color::Cyan),
            )]),
            Line::from("  Overview   - Summary of nodes and topics (top 10)"),
            Line::from("  Nodes      - Full list of detected HORUS nodes with details"),
            Line::from("  Topics     - Full list of shared memory topics"),
            Line::from("  Graph      - Node-topic graph visualization with pub/sub arrows"),
            Line::from("  Packages   - Local workspaces and global packages (hierarchical)"),
            Line::from("  Params     - Runtime configuration parameters (editable)"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Data Source: ", Style::default().fg(Color::Yellow)),
                Span::raw("Real-time from HORUS detect backend"),
            ]),
            Line::from("  • Nodes from /proc scan + registry"),
            Line::from("  • Topics from /dev/shm/horus/topics/"),
            Line::from("  • Packages from ~/.horus/cache + local .horus/ directories"),
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
        let (_title, logs) = match &self.panel_target {
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

        let help_text = format!("Showing {} logs |  Scroll | ESC Close", logs.len());

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
            "[ESC] Close | [] Scroll Logs | [Shift+] Switch Node/Topic | [Q] Quit"
        } else if self.active_tab == Tab::Parameters && self.param_edit_mode == ParamEditMode::None
        {
            "[A] Add | [E] Edit | [D] Delete | [R] Refresh | [S] Save | [TAB] Switch Tab | [?] Help | [Q] Quit"
        } else if self.active_tab == Tab::Parameters {
            "[TAB] Next Field | [ENTER] Confirm | [ESC] Cancel | [BACKSPACE] Delete Char"
        } else if self.active_tab == Tab::Packages
            && self.package_view_mode == PackageViewMode::List
        {
            "[ENTER] View Packages | [] Navigate | [TAB] Switch Tab | [?] Help | [Q] Quit"
        } else if self.active_tab == Tab::Packages
            && self.package_view_mode == PackageViewMode::WorkspaceDetails
        {
            "[ESC] Back to Workspaces | [] Navigate | [TAB] Switch Tab | [?] Help | [Q] Quit"
        } else if self.active_tab == Tab::Nodes || self.active_tab == Tab::Topics {
            "[ENTER] View Logs | [] Navigate | [TAB] Switch Tab | [P] Pause | [?] Help | [Q] Quit"
        } else {
            "[TAB] Switch Tab | [] Navigate | [P] Pause | [?] Help | [Q] Quit"
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

        // Update graph data
        self.update_graph_data();

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
            Tab::Packages => {
                if self.package_view_mode == PackageViewMode::List {
                    match self.package_panel_focus {
                        PackagePanelFocus::LocalWorkspaces => {
                            self.workspace_cache.len().saturating_sub(1)
                        }
                        PackagePanelFocus::GlobalPackages => {
                            let (_, global_packages) = get_installed_packages();
                            global_packages.len().saturating_sub(1)
                        }
                    }
                } else {
                    0
                }
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

    /// Get the count of active nodes, excluding placeholder entries
    fn get_active_node_count(&self) -> usize {
        if self.nodes.len() == 1 && self.nodes[0].name.contains("No HORUS nodes") {
            0
        } else {
            self.nodes.len()
        }
    }

    /// Get the count of active topics, excluding placeholder entries
    fn get_active_topic_count(&self) -> usize {
        if self.topics.len() == 1 && self.topics[0].name.contains("No active topics") {
            0
        } else {
            self.topics.len()
        }
    }

    fn handle_package_enter(&mut self) {
        match self.package_view_mode {
            PackageViewMode::List => {
                // Drill down into selected workspace (use cached data)
                if self.selected_index < self.workspace_cache.len() {
                    self.selected_workspace = Some(self.workspace_cache[self.selected_index].clone());
                    self.package_view_mode = PackageViewMode::WorkspaceDetails;
                    self.selected_index = 0;
                    self.scroll_offset = 0;
                }
            }
            PackageViewMode::WorkspaceDetails => {
                // Could expand nested packages here in the future
            }
        }
    }

    fn draw_param_edit_dialog(&self, f: &mut Frame) {
        // Create centered popup area
        let area = f.size();
        let popup_width = 60.min(area.width - 4);
        let popup_height = 10.min(area.height - 4);
        let popup_x = (area.width - popup_width) / 2;
        let popup_y = (area.height - popup_height) / 2;

        let popup_area = Rect {
            x: popup_x,
            y: popup_y,
            width: popup_width,
            height: popup_height,
        };

        // Clear the popup area
        let clear_block = Block::default().style(Style::default().bg(Color::Reset));
        f.render_widget(clear_block, popup_area);

        match &self.param_edit_mode {
            ParamEditMode::Add => {
                let title = "Add New Parameter [ESC to cancel]";
                let block = Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green));

                let inner = block.inner(popup_area);
                f.render_widget(block, popup_area);

                // Split into key and value sections
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3), // Key input
                        Constraint::Length(3), // Value input
                        Constraint::Min(1),    // Help text
                    ])
                    .split(inner);

                // Draw key input
                let key_focused = self.param_input_focus == ParamInputFocus::Key;
                let key_block = Block::default()
                    .title("Key")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(if key_focused {
                        Color::Yellow
                    } else {
                        Color::Gray
                    }));
                let key_text = Paragraph::new(self.param_input_key.as_str()).block(key_block);
                f.render_widget(key_text, chunks[0]);

                // Draw value input
                let value_focused = self.param_input_focus == ParamInputFocus::Value;
                let value_block = Block::default()
                    .title("Value (JSON or string)")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(if value_focused {
                        Color::Yellow
                    } else {
                        Color::Gray
                    }));
                let value_text = Paragraph::new(self.param_input_value.as_str()).block(value_block);
                f.render_widget(value_text, chunks[1]);

                // Help text
                let help = Paragraph::new("Press [Enter] to move to next field or confirm")
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center);
                f.render_widget(help, chunks[2]);
            }
            ParamEditMode::Edit(original_key) => {
                let title = format!("Edit Parameter: {} [ESC to cancel]", original_key);
                let block = Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan));

                let inner = block.inner(popup_area);
                f.render_widget(block, popup_area);

                // Split into key and value sections
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3), // Key input
                        Constraint::Length(3), // Value input
                        Constraint::Min(1),    // Help text
                    ])
                    .split(inner);

                // Draw key input
                let key_focused = self.param_input_focus == ParamInputFocus::Key;
                let key_block = Block::default()
                    .title("Key")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(if key_focused {
                        Color::Yellow
                    } else {
                        Color::Gray
                    }));
                let key_text = Paragraph::new(self.param_input_key.as_str()).block(key_block);
                f.render_widget(key_text, chunks[0]);

                // Draw value input
                let value_focused = self.param_input_focus == ParamInputFocus::Value;
                let value_block = Block::default()
                    .title("Value (JSON or string)")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(if value_focused {
                        Color::Yellow
                    } else {
                        Color::Gray
                    }));
                let value_text = Paragraph::new(self.param_input_value.as_str()).block(value_block);
                f.render_widget(value_text, chunks[1]);

                // Help text
                let help = Paragraph::new("Press [Enter] to move to next field or confirm")
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center);
                f.render_widget(help, chunks[2]);
            }
            ParamEditMode::Delete(key) => {
                let title = "Delete Parameter [ESC to cancel]";
                let block = Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red));

                let inner = block.inner(popup_area);
                f.render_widget(block, popup_area);

                // Show confirmation message
                let message = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("Delete parameter '{}'?", key),
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from("This action cannot be undone."),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Press ", Style::default().fg(Color::DarkGray)),
                        Span::styled("[Y]", Style::default().fg(Color::Green)),
                        Span::styled(" to confirm or ", Style::default().fg(Color::DarkGray)),
                        Span::styled("[N]", Style::default().fg(Color::Red)),
                        Span::styled(" to cancel", Style::default().fg(Color::DarkGray)),
                    ]),
                ];

                let paragraph = Paragraph::new(message).alignment(Alignment::Center);
                f.render_widget(paragraph, inner);
            }
            _ => {}
        }
    }

    // Parameter editing operations
    fn start_edit_parameter(&mut self) {
        let params_map = self.params.get_all();
        let params: Vec<_> = params_map.iter().collect();

        if self.selected_index < params.len() {
            let (key, value) = params[self.selected_index];
            self.param_edit_mode = ParamEditMode::Edit(key.clone());
            self.param_input_key = key.clone();
            self.param_input_value = value.to_string();
            self.param_input_focus = ParamInputFocus::Key;
        }
    }

    fn start_delete_parameter(&mut self) {
        let params_map = self.params.get_all();
        let params: Vec<_> = params_map.iter().collect();

        if self.selected_index < params.len() {
            let (key, _) = params[self.selected_index];
            self.param_edit_mode = ParamEditMode::Delete(key.clone());
        }
    }

    fn confirm_add_parameter(&mut self) {
        if !self.param_input_key.is_empty() {
            // Try to parse as JSON, fallback to string
            let value = if let Ok(json_value) = serde_json::from_str(&self.param_input_value) {
                json_value
            } else {
                serde_json::Value::String(self.param_input_value.clone())
            };

            // Create a mutable copy of the params
            let new_params = horus_core::RuntimeParams::default();
            for (k, v) in self.params.get_all().iter() {
                let _ = new_params.set(k, v.clone());
            }

            // Add the new parameter
            let _ = new_params.set(&self.param_input_key, value);

            // Replace the Arc
            self.params = std::sync::Arc::new(new_params);

            // Exit edit mode
            self.param_edit_mode = ParamEditMode::None;
            self.param_input_key.clear();
            self.param_input_value.clear();
        }
    }

    fn confirm_edit_parameter(&mut self) {
        if let ParamEditMode::Edit(original_key) = &self.param_edit_mode.clone() {
            // Try to parse as JSON, fallback to string
            let value = if let Ok(json_value) = serde_json::from_str(&self.param_input_value) {
                json_value
            } else {
                serde_json::Value::String(self.param_input_value.clone())
            };

            // Create a mutable copy of the params
            let new_params = horus_core::RuntimeParams::default();
            for (k, v) in self.params.get_all().iter() {
                if k != original_key {
                    let _ = new_params.set(k, v.clone());
                }
            }

            // Add the (possibly renamed) parameter with new value
            let _ = new_params.set(&self.param_input_key, value);

            // Replace the Arc
            self.params = std::sync::Arc::new(new_params);

            // Exit edit mode
            self.param_edit_mode = ParamEditMode::None;
            self.param_input_key.clear();
            self.param_input_value.clear();
        }
    }

    fn confirm_delete_parameter(&mut self) {
        if let ParamEditMode::Delete(key_to_delete) = &self.param_edit_mode.clone() {
            // Create a mutable copy of the params
            let new_params = horus_core::RuntimeParams::default();
            for (k, v) in self.params.get_all().iter() {
                if k != key_to_delete {
                    let _ = new_params.set(k, v.clone());
                }
            }

            // Replace the Arc
            self.params = std::sync::Arc::new(new_params);

            // Exit edit mode
            self.param_edit_mode = ParamEditMode::None;
        }
    }

    fn update_graph_data(&mut self) {
        // Discover graph data from the graph module
        let (graph_nodes, graph_edges) = crate::graph::discover_graph_data();

        // Convert to TUI graph nodes
        self.graph_nodes = graph_nodes
            .into_iter()
            .map(|node| TuiGraphNode {
                id: node.id,
                label: node.label,
                node_type: match node.node_type {
                    crate::graph::NodeType::Process => TuiNodeType::Process,
                    crate::graph::NodeType::Topic => TuiNodeType::Topic,
                },
                x: 0,  // Will be set by layout
                y: 0,
                pid: node.pid,
                active: node.active,
            })
            .collect();

        // Convert to TUI graph edges
        self.graph_edges = graph_edges
            .into_iter()
            .map(|edge| TuiGraphEdge {
                from: edge.from,
                to: edge.to,
                edge_type: match edge.edge_type {
                    crate::graph::EdgeType::Publish => TuiEdgeType::Publish,
                    crate::graph::EdgeType::Subscribe => TuiEdgeType::Subscribe,
                },
                active: edge.active,
            })
            .collect();

        // Apply layout algorithm
        self.apply_graph_layout();
    }

    fn apply_graph_layout(&mut self) {
        if self.graph_nodes.is_empty() {
            return;
        }

        match self.graph_layout {
            GraphLayout::Hierarchical => self.apply_hierarchical_layout(),
            GraphLayout::Vertical => self.apply_vertical_layout(),
        }
    }

    fn apply_hierarchical_layout(&mut self) {
        // Separate processes and topics
        let mut processes: Vec<&mut TuiGraphNode> = Vec::new();
        let mut topics: Vec<&mut TuiGraphNode> = Vec::new();

        for node in &mut self.graph_nodes {
            match node.node_type {
                TuiNodeType::Process => processes.push(node),
                TuiNodeType::Topic => topics.push(node),
            }
        }

        // Layout processes on the left
        let process_x = 5;
        let mut process_y = 3;
        let process_spacing = 3;

        for process in processes {
            process.x = process_x;
            process.y = process_y;
            process_y += process_spacing;
        }

        // Layout topics on the right
        let topic_x = 40;
        let mut topic_y = 3;
        let topic_spacing = 3;

        for topic in topics {
            topic.x = topic_x;
            topic.y = topic_y;
            topic_y += topic_spacing;
        }
    }

    fn apply_vertical_layout(&mut self) {
        // Separate processes and topics
        let mut processes: Vec<&mut TuiGraphNode> = Vec::new();
        let mut topics: Vec<&mut TuiGraphNode> = Vec::new();

        for node in &mut self.graph_nodes {
            match node.node_type {
                TuiNodeType::Process => processes.push(node),
                TuiNodeType::Topic => topics.push(node),
            }
        }

        // Layout processes on top
        let process_y = 3;
        let mut process_x = 5;
        let process_spacing = 15;

        for process in processes {
            process.x = process_x;
            process.y = process_y;
            process_x += process_spacing;
        }

        // Layout topics on bottom
        let topic_y = 12;
        let mut topic_x = 5;
        let topic_spacing = 15;

        for topic in topics {
            topic.x = topic_x;
            topic.y = topic_y;
            topic_x += topic_spacing;
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

fn get_local_workspaces(current_workspace_path: &Option<std::path::PathBuf>) -> Vec<WorkspaceData> {
    use std::fs;

    let mut workspaces = Vec::new();

    // Use the WorkspaceRegistry to get only registered HORUS workspaces
    let registry = match crate::workspace::WorkspaceRegistry::load() {
        Ok(reg) => reg,
        Err(_) => {
            // If registry is unavailable but we have a current workspace, include it
            if let Some(current_path) = current_workspace_path {
                let current_name = current_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("current")
                    .to_string();

                workspaces.push(WorkspaceData {
                    name: current_name,
                    path: current_path.to_string_lossy().to_string(),
                    packages: Vec::new(), // Will be populated below if .horus/packages exists
                    is_current: true,
                });
            }
            return workspaces;
        }
    };

    // Only process registered workspaces
    for ws in &registry.workspaces {
        let env_path_buf = &ws.path;
        let horus_dir = env_path_buf.join(".horus");

        // Verify the workspace still exists and has .horus/ directory
        if !horus_dir.exists() || !horus_dir.is_dir() {
            continue;
        }

        // Check if this is the current workspace
        let is_current = current_workspace_path
            .as_ref()
            .map(|p| p == env_path_buf)
            .unwrap_or(false);

        let env_name = ws.name.clone();
        let env_path = env_path_buf.to_string_lossy().to_string();

        // Get packages inside this environment
        let packages_dir = horus_dir.join("packages");
        let mut packages = Vec::new();

        if packages_dir.exists() {
            if let Ok(pkg_entries) = fs::read_dir(&packages_dir) {
                for pkg_entry in pkg_entries.flatten() {
                    if pkg_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        let pkg_name = pkg_entry.file_name().to_string_lossy().to_string();

                        // Try to get version from metadata.json
                        let metadata_path = pkg_entry.path().join("metadata.json");
                        let version = if metadata_path.exists() {
                            fs::read_to_string(&metadata_path)
                                .ok()
                                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                                .and_then(|j| {
                                    j.get("version")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string())
                                })
                                .unwrap_or_else(|| "unknown".to_string())
                        } else {
                            "unknown".to_string()
                        };

                        // Scan for installed packages inside this package's .horus/packages/
                        let nested_packages_dir = pkg_entry.path().join(".horus/packages");
                        let mut installed_packages = Vec::new();

                        if nested_packages_dir.exists() {
                            if let Ok(nested_entries) = fs::read_dir(&nested_packages_dir) {
                                for nested_entry in nested_entries.flatten() {
                                    if nested_entry
                                        .file_type()
                                        .map(|t| t.is_dir())
                                        .unwrap_or(false)
                                    {
                                        let nested_name =
                                            nested_entry.file_name().to_string_lossy().to_string();

                                        // Try to get version
                                        let nested_metadata_path =
                                            nested_entry.path().join("metadata.json");
                                        let nested_version = if nested_metadata_path.exists() {
                                            fs::read_to_string(&nested_metadata_path)
                                                .ok()
                                                .and_then(|s| {
                                                    serde_json::from_str::<serde_json::Value>(&s)
                                                        .ok()
                                                })
                                                .and_then(|j| {
                                                    j.get("version")
                                                        .and_then(|v| v.as_str())
                                                        .map(|s| s.to_string())
                                                })
                                                .unwrap_or_else(|| "unknown".to_string())
                                        } else {
                                            "unknown".to_string()
                                        };

                                        installed_packages.push((nested_name, nested_version));
                                    }
                                }
                            }
                        }

                        packages.push(PackageData {
                            name: pkg_name,
                            version,
                            installed_packages,
                        });
                    }
                }
            }

            workspaces.push(WorkspaceData {
                name: env_name,
                path: env_path,
                packages,
                is_current,
            });
        }
    }

    // Sort by: current workspace first, then alphabetically
    workspaces.sort_by(|a, b| {
        match (a.is_current, b.is_current) {
            (true, false) => std::cmp::Ordering::Less,    // Current workspace comes first
            (false, true) => std::cmp::Ordering::Greater, // Current workspace comes first
            _ => a.name.cmp(&b.name),                     // Otherwise sort alphabetically
        }
    });
    workspaces
}

fn get_installed_packages() -> (Vec<(String, String, String)>, Vec<(String, String, String)>) {
    let mut local_packages = Vec::new();
    let mut global_packages = Vec::new();
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

                            local_packages.push((name.to_string(), "latest".to_string(), size));
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

                            global_packages.push((name.to_string(), "latest".to_string(), size));
                        }
                    }
                }
            }
        }
    }

    // Sort both lists
    local_packages.sort_by(|a, b| a.0.cmp(&b.0));
    global_packages.sort_by(|a, b| a.0.cmp(&b.0));

    // Add placeholder if both are empty
    if local_packages.is_empty() && global_packages.is_empty() {
        local_packages.push((
            "No packages found".to_string(),
            "-".to_string(),
            "-".to_string(),
        ));
    }

    (local_packages, global_packages)
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
