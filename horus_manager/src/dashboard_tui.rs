// Terminal UI Dashboard for HORUS
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use horus_core::core::{LogType, GLOBAL_LOG_BUFFER};
use horus_core::memory::shm_topics_dir;
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
use tui_nodes::{Connection, LineType, NodeGraph, NodeLayout};

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

    // Overview panel focus
    overview_panel_focus: OverviewPanelFocus,

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

#[derive(Debug, Clone, PartialEq)]
enum OverviewPanelFocus {
    Nodes,  // Focused on nodes panel
    Topics, // Focused on topics panel
}

#[derive(Debug, Clone)]
struct WorkspaceData {
    name: String,
    path: String,
    packages: Vec<PackageData>,
    dependencies: Vec<DependencyData>, // Declared in horus.yaml but not installed
    is_current: bool, // True if this is the current workspace (detected via find_workspace_root)
}

#[derive(Debug, Clone)]
struct PackageData {
    name: String,
    version: String,
    installed_packages: Vec<(String, String)>, // (name, version) pairs
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used for future dependency tracking UI
struct DependencyData {
    name: String,
    declared_version: String, // Version string from horus.yaml (e.g., "package@1.0.0" or just "package")
    status: DependencyStatus,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // Variants for future dependency status display
enum DependencyStatus {
    Missing,   // Declared but not installed
    Installed, // Both declared and installed (shown in packages list)
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
#[allow(dead_code)] // Fields for future graph visualization
struct TuiGraphNode {
    id: String,
    label: String,
    node_type: TuiNodeType,
    x: i32, // TUI coordinates (character cells)
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
    ForceDirected, // Automatic physics-based layout
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
    Overview,
    Nodes,
    Topics,
    Graph,
    Network,
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
            Tab::Network => "Network",
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
            Tab::Network,
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

            overview_panel_focus: OverviewPanelFocus::Nodes,

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
    #[allow(dead_code)] // Reserved for manual refresh keybinding
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
                            // Toggle layout (cycles through all options)
                            self.graph_layout = match self.graph_layout {
                                GraphLayout::Hierarchical => GraphLayout::Vertical,
                                GraphLayout::Vertical => GraphLayout::ForceDirected,
                                GraphLayout::ForceDirected => GraphLayout::Hierarchical,
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
                        KeyCode::Left if self.active_tab == Tab::Graph && !self.show_log_panel => {
                            // Pan left
                            self.graph_offset_x += 5;
                        }
                        KeyCode::Right if self.active_tab == Tab::Graph && !self.show_log_panel => {
                            // Pan right
                            self.graph_offset_x -= 5;
                        }

                        // Switch between Nodes/Topics panels in Overview tab
                        KeyCode::Left
                            if self.active_tab == Tab::Overview && !self.show_log_panel =>
                        {
                            self.overview_panel_focus = OverviewPanelFocus::Nodes;
                            self.selected_index = 0;
                            self.scroll_offset = 0;
                        }
                        KeyCode::Right
                            if self.active_tab == Tab::Overview && !self.show_log_panel =>
                        {
                            self.overview_panel_focus = OverviewPanelFocus::Topics;
                            self.selected_index = 0;
                            self.scroll_offset = 0;
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
            .split(f.area());

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
                    Tab::Network => self.draw_network(f, horizontal_chunks[0]),
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
                    Tab::Network => self.draw_network(f, content_area),
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
            Span::raw("v0.1.5 | "),
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
        // Calculate how many rows can fit in the panel
        let available_height = area.height.saturating_sub(3); // Subtract borders and header
        let page_size = available_height as usize;

        let rows: Vec<Row> = self
            .nodes
            .iter()
            .skip(self.scroll_offset)
            .take(page_size)
            .map(|node| {
                let is_running = node.status == "active";
                let status_symbol = if is_running { "●" } else { "○" };
                let status_color = if is_running { Color::Green } else { Color::Red };

                Row::new(vec![
                    Cell::from(status_symbol).style(Style::default().fg(status_color)),
                    Cell::from(node.name.clone()),
                    Cell::from(node.process_id.to_string()),
                    Cell::from(format!("{} MB", node.memory_usage / 1024 / 1024)),
                ])
            })
            .collect();

        let is_focused = self.overview_panel_focus == OverviewPanelFocus::Nodes;
        let border_color = if is_focused {
            Color::Cyan
        } else {
            Color::White
        };

        let widths = [
            Constraint::Length(2),
            Constraint::Min(30),
            Constraint::Length(8),
            Constraint::Length(12),
        ];
        let table = Table::new(rows, widths)
            .header(
                Row::new(vec!["", "Name", "PID", "Memory"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(
                Block::default()
                    .title(format!(
                        "Active Nodes ({}) - Use Left/Right to switch panels",
                        self.get_active_node_count()
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color)),
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(" ");

        let mut table_state = TableState::default();
        if is_focused && !self.nodes.is_empty() {
            // Highlight the currently selected item within the visible page
            let selected = self.selected_index.min(self.nodes.len() - 1);
            if selected >= self.scroll_offset && selected < self.scroll_offset + page_size {
                table_state.select(Some(selected - self.scroll_offset));
            }
        }

        f.render_stateful_widget(table, area, &mut table_state);
    }

    fn draw_topic_summary(&self, f: &mut Frame, area: Rect) {
        // Calculate how many rows can fit in the panel
        let available_height = area.height.saturating_sub(3); // Subtract borders and header
        let page_size = available_height as usize;

        let rows: Vec<Row> = self
            .topics
            .iter()
            .skip(self.scroll_offset)
            .take(page_size)
            .map(|topic| {
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
            })
            .collect();

        let is_focused = self.overview_panel_focus == OverviewPanelFocus::Topics;
        let border_color = if is_focused {
            Color::Cyan
        } else {
            Color::White
        };

        let widths = [
            Constraint::Percentage(30),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Length(10),
        ];
        let table = Table::new(rows, widths)
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
                    .title(format!(
                        "Active Topics ({}) - Use Left/Right to switch panels",
                        self.get_active_topic_count()
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color)),
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(" ");

        let mut table_state = TableState::default();
        if is_focused && !self.topics.is_empty() {
            // Highlight the currently selected item within the visible page
            let selected = self.selected_index.min(self.topics.len() - 1);
            if selected >= self.scroll_offset && selected < self.scroll_offset + page_size {
                table_state.select(Some(selected - self.scroll_offset));
            }
        }

        f.render_stateful_widget(table, area, &mut table_state);
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

        let widths = [Constraint::Length(2), Constraint::Min(10)];
        let table = Table::new(rows, widths)
            .header(
                Row::new(vec!["", "Topic Name"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(Block::default().title("Topics").borders(Borders::ALL))
            .row_highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(" ");

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

        let widths = [
            Constraint::Percentage(25),
            Constraint::Percentage(20),
            Constraint::Length(8),
            Constraint::Percentage(27),
            Constraint::Percentage(28),
        ];
        let table = Table::new(rows, widths)
            .header(
                Row::new(vec!["Topic", "Type", "Hz", "Publishers", "Subscribers"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(
                Block::default()
                    .title("Topics - Use  to select, Enter to view logs")
                    .borders(Borders::ALL),
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(" ");

        // Create table state with current selection
        let mut table_state = TableState::default();
        if !self.topics.is_empty() {
            // Clamp selected_index to valid range
            let selected = self.selected_index.min(self.topics.len() - 1);
            table_state.select(Some(selected));
        }

        f.render_stateful_widget(table, area, &mut table_state);
    }

    fn draw_graph(&mut self, f: &mut Frame, area: Rect) {
        use ratatui::widgets::StatefulWidget;
        use std::collections::HashMap;

        // Create a block for the graph
        let block = Block::default()
            .title(format!(
                "Graph - {} nodes, {} edges | [L]ayout: {:?} | [R]efresh",
                self.graph_nodes.len(),
                self.graph_edges.len(),
                self.graph_layout
            ))
            .borders(Borders::ALL);

        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.graph_nodes.is_empty() {
            // Show message if no graph data
            let text = Paragraph::new(
                "No nodes or topics detected. Start some HORUS nodes to see the graph.",
            )
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);
            f.render_widget(text, inner);
            return;
        }

        // Build tui-nodes structures
        // We need to map our internal node IDs to indices for tui-nodes
        let mut id_to_index: HashMap<String, usize> = HashMap::new();

        // Separate processes and topics for layout
        let mut processes: Vec<&TuiGraphNode> = Vec::new();
        let mut topics: Vec<&TuiGraphNode> = Vec::new();

        for node in &self.graph_nodes {
            match node.node_type {
                TuiNodeType::Process => processes.push(node),
                TuiNodeType::Topic => topics.push(node),
            }
        }

        // Calculate node sizes based on label length
        let node_height = 3u16; // Fixed height for all nodes
        let min_width = 12u16;

        // First, collect all labels (to own the strings for the lifetime of this function)
        let mut labels: Vec<String> = Vec::new();
        let mut border_styles: Vec<Style> = Vec::new();
        let mut widths_vec: Vec<u16> = Vec::new();

        // Process labels
        for node in &processes {
            let label = if node.active {
                format!("● {}", node.label)
            } else {
                format!("○ {}", node.label)
            };
            let width = (label.len() as u16 + 4).max(min_width);
            let border_style = if node.active {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            labels.push(label);
            border_styles.push(border_style);
            widths_vec.push(width);
        }

        // Topic labels
        for node in &topics {
            let label = format!("◆ {}", node.label);
            let width = (label.len() as u16 + 4).max(min_width);
            let border_style = Style::default().fg(Color::Yellow);
            labels.push(label);
            border_styles.push(border_style);
            widths_vec.push(width);
        }

        // Create NodeLayout for each node
        // Order: processes first, then topics (for consistent indexing)
        let mut node_layouts: Vec<NodeLayout> = Vec::new();
        let mut current_index = 0usize;

        // Add process nodes
        for (i, node) in processes.iter().enumerate() {
            let layout = NodeLayout::new((widths_vec[i], node_height))
                .with_title(&labels[i])
                .with_border_style(border_styles[i]);

            node_layouts.push(layout);
            id_to_index.insert(node.id.clone(), current_index);
            current_index += 1;
        }

        // Add topic nodes
        let offset = processes.len();
        for (i, node) in topics.iter().enumerate() {
            let idx = offset + i;
            let layout = NodeLayout::new((widths_vec[idx], node_height))
                .with_title(&labels[idx])
                .with_border_style(border_styles[idx]);

            node_layouts.push(layout);
            id_to_index.insert(node.id.clone(), current_index);
            current_index += 1;
        }

        // Create connections
        // tui-nodes uses port indices: each node can have multiple input/output ports
        // For simplicity, we use port 0 for all connections
        let mut connections: Vec<Connection> = Vec::new();

        for edge in &self.graph_edges {
            if let (Some(&from_idx), Some(&to_idx)) =
                (id_to_index.get(&edge.from), id_to_index.get(&edge.to))
            {
                let line_style = match (&edge.edge_type, edge.active) {
                    (TuiEdgeType::Publish, true) => Style::default().fg(Color::Blue),
                    (TuiEdgeType::Publish, false) => Style::default().fg(Color::DarkGray),
                    (TuiEdgeType::Subscribe, true) => Style::default().fg(Color::Magenta),
                    (TuiEdgeType::Subscribe, false) => Style::default().fg(Color::DarkGray),
                };

                let connection = Connection::new(from_idx, 0, to_idx, 0)
                    .with_line_type(LineType::Rounded)
                    .with_line_style(line_style);

                connections.push(connection);
            }
        }

        // Create the NodeGraph widget
        let mut node_graph = NodeGraph::new(
            node_layouts,
            connections,
            inner.width as usize,
            inner.height as usize,
        );

        // Calculate layout
        node_graph.calculate();

        // Get the sub-areas for each node
        let node_areas = node_graph.split(inner);

        // Render the graph using StatefulWidget
        // NodeGraph requires a state, we use a unit tuple for now
        let mut state = ();
        StatefulWidget::render(node_graph, inner, f.buffer_mut(), &mut state);

        // Render node content inside each node area
        for (i, node_area) in node_areas.iter().enumerate() {
            if node_area.width > 2 && node_area.height > 2 {
                // Get the inner area (inside borders)
                let content_area = Rect {
                    x: node_area.x + 1,
                    y: node_area.y + 1,
                    width: node_area.width.saturating_sub(2),
                    height: node_area.height.saturating_sub(2),
                };

                // Determine if this is a process or topic
                let is_process = i < processes.len();
                let node = if is_process {
                    processes.get(i)
                } else {
                    topics.get(i - processes.len())
                };

                if let Some(node) = node {
                    // Show additional info in the node
                    let info = if is_process {
                        if let Some(pid) = node.pid {
                            format!("PID: {}", pid)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };

                    if !info.is_empty() && content_area.height > 0 {
                        let info_style = Style::default().fg(Color::Gray);
                        let info_text = Paragraph::new(info).style(info_style);
                        f.render_widget(info_text, content_area);
                    }
                }
            }
        }
    }

    fn draw_network(&self, f: &mut Frame, area: Rect) {
        let summary = crate::commands::monitor::get_network_summary();

        // Split area into summary panel and details table
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(5)])
            .split(area);

        // Draw summary panel
        let transport_info: String = if summary.transport_breakdown.is_empty() {
            "No active transports".to_string()
        } else {
            summary
                .transport_breakdown
                .iter()
                .map(|(t, c)| format!("{}: {}", t, c))
                .collect::<Vec<_>>()
                .join(" | ")
        };

        let summary_text = vec![
            Line::from(vec![
                Span::styled("Active Nodes: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("{}", summary.total_nodes),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Transports: ", Style::default().fg(Color::Cyan)),
                Span::raw(transport_info),
            ]),
            Line::from(vec![
                Span::styled("Bytes Sent: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format_bytes(summary.total_bytes_sent),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(" | "),
                Span::styled("Received: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format_bytes(summary.total_bytes_received),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
            Line::from(vec![
                Span::styled("Packets Sent: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("{}", summary.total_packets_sent),
                    Style::default().fg(Color::Magenta),
                ),
                Span::raw(" | "),
                Span::styled("Received: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("{}", summary.total_packets_received),
                    Style::default().fg(Color::Magenta),
                ),
            ]),
            Line::from(vec![
                Span::styled("Endpoints: ", Style::default().fg(Color::Cyan)),
                Span::raw(if summary.unique_endpoints.is_empty() {
                    "None discovered".to_string()
                } else {
                    summary.unique_endpoints.join(", ")
                }),
            ]),
        ];

        let summary_paragraph = Paragraph::new(summary_text).block(
            Block::default()
                .title("Network Summary")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );

        f.render_widget(summary_paragraph, chunks[0]);

        // Draw node network status table
        let rows: Vec<Row> = summary
            .node_statuses
            .iter()
            .map(|status| {
                let transport_color = match status.transport_type.as_str() {
                    "SharedMemory" => Color::Green,
                    "BatchUdp" | "Udp" => Color::Cyan,
                    "Quic" => Color::Magenta,
                    "UnixSocket" => Color::Yellow,
                    "IoUring" => Color::LightGreen,
                    _ => Color::White,
                };

                let endpoints = if status.remote_endpoints.is_empty() {
                    "-".to_string()
                } else {
                    status.remote_endpoints.join(", ")
                };

                let topics_pub = if status.network_topics_pub.is_empty() {
                    "-".to_string()
                } else {
                    status.network_topics_pub.join(", ")
                };

                Row::new(vec![
                    Cell::from(status.node_name.clone()),
                    Cell::from(status.transport_type.clone())
                        .style(Style::default().fg(transport_color)),
                    Cell::from(
                        status
                            .local_endpoint
                            .clone()
                            .unwrap_or_else(|| "-".to_string()),
                    ),
                    Cell::from(endpoints),
                    Cell::from(topics_pub),
                    Cell::from(format_bytes(status.bytes_sent)),
                    Cell::from(format_bytes(status.bytes_received)),
                ])
            })
            .collect();

        let widths = [
            Constraint::Percentage(15),
            Constraint::Length(12),
            Constraint::Percentage(15),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Length(10),
            Constraint::Length(10),
        ];

        let table = Table::new(rows, widths)
            .header(
                Row::new(vec![
                    "Node",
                    "Transport",
                    "Local",
                    "Remote",
                    "Topics",
                    "Sent",
                    "Recv",
                ])
                .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(
                Block::default()
                    .title("Node Network Status")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_widget(table, chunks[1]);
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
                    format!("> {} (current)", workspace.name)
                } else {
                    workspace.name.clone()
                };

                // Style: selected gets reversed, current workspace gets green color
                let style = if is_selected {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else if workspace.is_current {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                // Format package/dependency counts
                let pkg_count = workspace.packages.len();
                let missing_count = workspace.dependencies.len();
                let count_display = if missing_count > 0 {
                    format!("{} ({} missing)", pkg_count, missing_count)
                } else {
                    pkg_count.to_string()
                };

                Row::new(vec![
                    Cell::from(workspace_display),
                    Cell::from(count_display),
                    Cell::from(workspace.path.clone()),
                ])
                .style(style)
            })
            .collect();

        let workspace_widths = [
            Constraint::Length(25),
            Constraint::Length(10),
            Constraint::Min(30),
        ];
        let workspace_table = Table::new(workspace_rows, workspace_widths)
            .header(
                Row::new(vec!["Workspace", "Pkgs (Missing)", "Path"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(
                Block::default()
                    .title(format!(
                        "Local Workspaces ({}) {}",
                        workspaces.len(),
                        if local_focused {
                            "[FOCUSED - Press ← →]"
                        } else {
                            "[Press → to focus]"
                        }
                    ))
                    .borders(Borders::ALL)
                    .border_style(if local_focused {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }),
            )
            .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        // Add TableState for scrolling support
        let mut workspace_state = TableState::default();
        if local_focused && !workspaces.is_empty() {
            let selected = self.selected_index.min(workspaces.len() - 1);
            workspace_state.select(Some(selected));
        }

        f.render_stateful_widget(workspace_table, chunks[0], &mut workspace_state);

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

        let global_widths = [
            Constraint::Min(30),
            Constraint::Length(15),
            Constraint::Length(12),
        ];
        let global_table = Table::new(global_rows, global_widths)
            .header(
                Row::new(vec!["Package", "Version", "Size"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(
                Block::default()
                    .title(format!(
                        "Global Packages ({}) {}",
                        global_packages.len(),
                        if global_focused {
                            "[FOCUSED - Press ← →]"
                        } else {
                            "[Press ← to focus]"
                        }
                    ))
                    .borders(Borders::ALL)
                    .border_style(if global_focused {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }),
            )
            .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        let mut global_state = TableState::default();
        if global_focused && !global_packages.is_empty() {
            let selected = self.selected_index.min(global_packages.len() - 1);
            global_state.select(Some(selected));
        }
        f.render_stateful_widget(global_table, chunks[1], &mut global_state);
    }

    fn draw_workspace_details(&self, f: &mut Frame, area: Rect) {
        if let Some(ref workspace) = self.selected_workspace {
            // Split area into two sections: Installed Packages and Missing Dependencies
            let has_missing = !workspace.dependencies.is_empty();

            let chunks = if has_missing {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(60), // Installed packages
                        Constraint::Percentage(40), // Missing dependencies
                    ])
                    .split(area)
            } else {
                // Create a single-element slice for consistency
                use std::rc::Rc;
                Rc::from(vec![area])
            };

            // Display installed packages
            let package_rows: Vec<Row> = workspace
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
                        Cell::from(pkg.name.clone()).style(Style::default().fg(Color::Green)),
                        Cell::from(pkg.version.clone()),
                        Cell::from(pkg.installed_packages.len().to_string()),
                        Cell::from(installed),
                    ])
                    .style(style)
                })
                .collect();

            let package_widths = [
                Constraint::Length(25),
                Constraint::Length(12),
                Constraint::Length(6),
                Constraint::Min(30),
            ];
            let package_table = Table::new(package_rows, package_widths)
                .header(
                    Row::new(vec!["Package", "Version", "Deps", "Installed Packages"])
                        .style(Style::default().add_modifier(Modifier::BOLD)),
                )
                .block(
                    Block::default()
                        .title(format!(
                            "Workspace: {} - Installed Packages ({}) - Press Esc to return",
                            workspace.name,
                            workspace.packages.len()
                        ))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Green)),
                );

            f.render_widget(package_table, chunks[0]);

            // Display missing dependencies if any
            if has_missing {
                let dep_rows: Vec<Row> = workspace
                    .dependencies
                    .iter()
                    .map(|dep| {
                        Row::new(vec![
                            Cell::from(dep.name.clone()).style(Style::default().fg(Color::Yellow)),
                            Cell::from(dep.declared_version.clone()),
                            Cell::from("MISSING").style(Style::default().fg(Color::Red)),
                        ])
                    })
                    .collect();

                let dep_widths = [
                    Constraint::Length(25),
                    Constraint::Length(30),
                    Constraint::Min(15),
                ];
                let dep_table = Table::new(dep_rows, dep_widths)
                    .header(
                        Row::new(vec!["Package", "Declared (horus.yaml)", "Status"])
                            .style(Style::default().add_modifier(Modifier::BOLD)),
                    )
                    .block(
                        Block::default()
                            .title(format!(
                                "Missing Dependencies ({}) - Run 'horus run' to install",
                                workspace.dependencies.len()
                            ))
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::Red)),
                    );

                f.render_widget(dep_table, chunks[1]);
            }
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

        let widths = [
            Constraint::Percentage(35),
            Constraint::Percentage(50),
            Constraint::Percentage(15),
        ];
        let table = Table::new(rows, widths)
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
            );

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

        let widths = [Constraint::Length(2), Constraint::Min(10)];
        let table = Table::new(rows, widths)
            .header(
                Row::new(vec!["", "Node Name"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(Block::default().title("Nodes").borders(Borders::ALL))
            .row_highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(" ");

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

        let widths = [
            Constraint::Percentage(15),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Percentage(24),
            Constraint::Percentage(25),
        ];
        let table = Table::new(rows, widths)
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
            .row_highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(" ");

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
            Line::from(
                "  Tab        - Next tab (Overview  Nodes  Topics  Graph  Packages  Params)",
            ),
            Line::from("  Shift+Tab  - Previous tab"),
            Line::from("  ↑/↓        - Navigate lists"),
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
            Line::from("  Shift+↑↓   - Switch between nodes/topics while log panel is open"),
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
            Line::from(
                "  Enter      - Drill into selected workspace to view packages & dependencies",
            ),
            Line::from("  ESC        - Navigate back to workspace list"),
            Line::from("  Note       - Missing dependencies (from horus.yaml) shown in red"),
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
            Line::from(format!("  • Topics from {}", shm_topics_dir().display())),
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
            Tab::Overview => match self.overview_panel_focus {
                OverviewPanelFocus::Nodes => self.nodes.len().saturating_sub(1),
                OverviewPanelFocus::Topics => self.topics.len().saturating_sub(1),
            },
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
                    self.selected_workspace =
                        Some(self.workspace_cache[self.selected_index].clone());
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
        let area = f.area();
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
                x: 0, // Will be set by layout
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
            GraphLayout::ForceDirected => self.apply_force_directed_layout(),
        }
    }

    fn apply_hierarchical_layout(&mut self) {
        use std::collections::HashMap;

        // Barycenter Heuristic Layout: Minimizes edge crossings in bipartite graph
        // This is the same algorithm used in the web dashboard

        // Separate processes and topics
        let mut processes: Vec<String> = Vec::new();
        let mut topics: Vec<String> = Vec::new();

        for node in &self.graph_nodes {
            match node.node_type {
                TuiNodeType::Process => processes.push(node.id.clone()),
                TuiNodeType::Topic => topics.push(node.id.clone()),
            }
        }

        if processes.is_empty() && topics.is_empty() {
            return;
        }

        // Build adjacency maps
        let mut process_to_topics: HashMap<String, Vec<String>> = HashMap::new();
        let mut topic_to_processes: HashMap<String, Vec<String>> = HashMap::new();

        for edge in &self.graph_edges {
            if let (Some(from_node), Some(to_node)) = (
                self.graph_nodes.iter().find(|n| n.id == edge.from),
                self.graph_nodes.iter().find(|n| n.id == edge.to),
            ) {
                match (&from_node.node_type, &to_node.node_type) {
                    (TuiNodeType::Process, TuiNodeType::Topic) => {
                        process_to_topics
                            .entry(edge.from.clone())
                            .or_default()
                            .push(edge.to.clone());
                        topic_to_processes
                            .entry(edge.to.clone())
                            .or_default()
                            .push(edge.from.clone());
                    }
                    (TuiNodeType::Topic, TuiNodeType::Process) => {
                        topic_to_processes
                            .entry(edge.from.clone())
                            .or_default()
                            .push(edge.to.clone());
                        process_to_topics
                            .entry(edge.to.clone())
                            .or_default()
                            .push(edge.from.clone());
                    }
                    _ => {}
                }
            }
        }

        // Initial ordering (by ID for deterministic results)
        let mut process_order = processes.clone();
        process_order.sort();
        let mut topic_order = topics.clone();
        topic_order.sort();

        // Barycenter iterations (5 iterations for convergence)
        for _ in 0..5 {
            // Reorder topics based on average position of connected processes
            let mut topic_barycenters: Vec<(String, f32)> = topic_order
                .iter()
                .map(|topic_id| {
                    let connected_processes = topic_to_processes.get(topic_id);
                    let barycenter = if let Some(procs) = connected_processes {
                        if procs.is_empty() {
                            0.0
                        } else {
                            let sum: f32 = procs
                                .iter()
                                .filter_map(|proc_id| {
                                    process_order
                                        .iter()
                                        .position(|p| p == proc_id)
                                        .map(|i| i as f32)
                                })
                                .sum();
                            sum / procs.len() as f32
                        }
                    } else {
                        0.0
                    };
                    (topic_id.clone(), barycenter)
                })
                .collect();

            topic_barycenters
                .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            topic_order = topic_barycenters.into_iter().map(|(id, _)| id).collect();

            // Reorder processes based on average position of connected topics
            let mut process_barycenters: Vec<(String, f32)> = process_order
                .iter()
                .map(|process_id| {
                    let connected_topics = process_to_topics.get(process_id);
                    let barycenter = if let Some(tops) = connected_topics {
                        if tops.is_empty() {
                            0.0
                        } else {
                            let sum: f32 = tops
                                .iter()
                                .filter_map(|topic_id| {
                                    topic_order
                                        .iter()
                                        .position(|t| t == topic_id)
                                        .map(|i| i as f32)
                                })
                                .sum();
                            sum / tops.len() as f32
                        }
                    } else {
                        0.0
                    };
                    (process_id.clone(), barycenter)
                })
                .collect();

            process_barycenters
                .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            process_order = process_barycenters.into_iter().map(|(id, _)| id).collect();
        }

        // Calculate optimal spacing - more space for cleaner graph
        let process_spacing = 4; // Vertical space between processes
        let topic_spacing = 4; // Vertical space between topics

        // Calculate label width for dynamic positioning
        let max_label_len = process_order
            .iter()
            .filter_map(|p| self.graph_nodes.iter().find(|n| &n.id == p))
            .map(|n| n.label.len())
            .max()
            .unwrap_or(20);

        // Position processes on left, vertically distributed
        let process_x = 2;
        let process_start_y = 2;

        for (i, process_id) in process_order.iter().enumerate() {
            if let Some(node) = self.graph_nodes.iter_mut().find(|n| &n.id == process_id) {
                node.x = process_x;
                node.y = process_start_y + (i as i32 * process_spacing);
            }
        }

        // Position topics on right with good gap for edges
        let topic_x = process_x + max_label_len as i32 + 25; // More space for edges
        let topic_start_y = 2;

        for (i, topic_id) in topic_order.iter().enumerate() {
            if let Some(node) = self.graph_nodes.iter_mut().find(|n| &n.id == topic_id) {
                node.x = topic_x;
                node.y = topic_start_y + (i as i32 * topic_spacing);
            }
        }
    }

    fn apply_vertical_layout(&mut self) {
        use std::collections::HashMap;

        // Separate processes and topics
        let mut processes: Vec<String> = Vec::new();
        let mut topics: Vec<String> = Vec::new();

        for node in &self.graph_nodes {
            match node.node_type {
                TuiNodeType::Process => processes.push(node.id.clone()),
                TuiNodeType::Topic => topics.push(node.id.clone()),
            }
        }

        // Build adjacency map
        let mut process_to_topics: HashMap<String, Vec<String>> = HashMap::new();
        let mut topic_to_processes: HashMap<String, Vec<String>> = HashMap::new();

        for edge in &self.graph_edges {
            if let (Some(from_node), Some(to_node)) = (
                self.graph_nodes.iter().find(|n| n.id == edge.from),
                self.graph_nodes.iter().find(|n| n.id == edge.to),
            ) {
                match (&from_node.node_type, &to_node.node_type) {
                    (TuiNodeType::Process, TuiNodeType::Topic) => {
                        process_to_topics
                            .entry(edge.from.clone())
                            .or_default()
                            .push(edge.to.clone());
                    }
                    (TuiNodeType::Topic, TuiNodeType::Process) => {
                        topic_to_processes
                            .entry(edge.from.clone())
                            .or_default()
                            .push(edge.to.clone());
                    }
                    _ => {}
                }
            }
        }

        // Calculate dynamic spacing based on label lengths
        let max_process_len = processes
            .iter()
            .filter_map(|p| self.graph_nodes.iter().find(|n| &n.id == p))
            .map(|n| n.label.len())
            .max()
            .unwrap_or(15);

        let max_topic_len = topics
            .iter()
            .filter_map(|t| self.graph_nodes.iter().find(|n| &n.id == t))
            .map(|n| n.label.len() + 2) // +2 for brackets
            .max()
            .unwrap_or(15);

        // Layout processes on top
        let process_y = 2;
        let mut process_x = 3;
        let process_spacing = (max_process_len + 8) as i32; // Dynamic spacing

        for process_id in &processes {
            if let Some(node) = self.graph_nodes.iter_mut().find(|n| &n.id == process_id) {
                node.x = process_x;
                node.y = process_y;
                process_x += process_spacing;
            }
        }

        // Layout topics on bottom, aligned with their connected processes when possible
        let topic_y = 10;
        let mut topic_x = 3;
        let topic_spacing = (max_topic_len + 5) as i32; // Dynamic spacing
        let mut topic_positions: HashMap<String, i32> = HashMap::new();

        for topic_id in &topics {
            // Try to align with connected processes
            if let Some(connected_procs) = topic_to_processes.get(topic_id) {
                if !connected_procs.is_empty() {
                    // Calculate average X position of connected processes
                    let avg_x: f32 = connected_procs
                        .iter()
                        .filter_map(|p| self.graph_nodes.iter().find(|n| &n.id == p))
                        .map(|n| n.x as f32)
                        .sum::<f32>()
                        / connected_procs.len() as f32;

                    topic_x = avg_x as i32;
                }
            }

            // Ensure minimum spacing from previous topics
            if let Some(&last_x) = topic_positions.values().max() {
                if topic_x < (last_x + topic_spacing) {
                    topic_x = last_x + topic_spacing;
                }
            }

            if let Some(node) = self.graph_nodes.iter_mut().find(|n| &n.id == topic_id) {
                node.x = topic_x;
                node.y = topic_y;
                topic_positions.insert(topic_id.clone(), topic_x);
            }

            topic_x += topic_spacing;
        }
    }

    fn apply_force_directed_layout(&mut self) {
        use std::collections::HashMap;

        // Simple force-directed layout using spring physics
        // This creates a visually pleasing automatic layout
        const ITERATIONS: usize = 50;
        const REPULSION_STRENGTH: f32 = 100.0;
        const ATTRACTION_STRENGTH: f32 = 0.05;
        const DAMPING: f32 = 0.85;

        // Initialize positions if nodes have default (0,0) positions
        let node_count = self.graph_nodes.len();
        for (i, node) in self.graph_nodes.iter_mut().enumerate() {
            if node.x == 0 && node.y == 0 {
                // Spread nodes in a circle initially
                let angle = (i as f32 * 2.0 * std::f32::consts::PI) / node_count as f32;
                node.x = (50.0 + 20.0 * angle.cos()) as i32;
                node.y = (15.0 + 10.0 * angle.sin()) as i32;
            }
        }

        // Build edge map for attraction forces
        let mut edge_map: HashMap<String, Vec<String>> = HashMap::new();
        for edge in &self.graph_edges {
            edge_map
                .entry(edge.from.clone())
                .or_default()
                .push(edge.to.clone());
            edge_map
                .entry(edge.to.clone())
                .or_default()
                .push(edge.from.clone());
        }

        // Simulate physics
        for _ in 0..ITERATIONS {
            let mut forces: HashMap<String, (f32, f32)> = HashMap::new();

            // Initialize forces
            for node in &self.graph_nodes {
                forces.insert(node.id.clone(), (0.0, 0.0));
            }

            // Repulsion between all nodes
            for i in 0..self.graph_nodes.len() {
                for j in (i + 1)..self.graph_nodes.len() {
                    let node1 = &self.graph_nodes[i];
                    let node2 = &self.graph_nodes[j];

                    let dx = node2.x as f32 - node1.x as f32;
                    let dy = node2.y as f32 - node1.y as f32;
                    let dist = (dx * dx + dy * dy).sqrt().max(1.0);

                    let force = REPULSION_STRENGTH / (dist * dist);
                    let fx = (dx / dist) * force;
                    let fy = (dy / dist) * force;

                    if let Some(f) = forces.get_mut(&node1.id) {
                        f.0 -= fx;
                        f.1 -= fy;
                    }
                    if let Some(f) = forces.get_mut(&node2.id) {
                        f.0 += fx;
                        f.1 += fy;
                    }
                }
            }

            // Attraction along edges
            for (from_id, to_ids) in &edge_map {
                if let Some(from_node) = self.graph_nodes.iter().find(|n| &n.id == from_id) {
                    for to_id in to_ids {
                        if let Some(to_node) = self.graph_nodes.iter().find(|n| &n.id == to_id) {
                            let dx = to_node.x as f32 - from_node.x as f32;
                            let dy = to_node.y as f32 - from_node.y as f32;

                            let force_x = dx * ATTRACTION_STRENGTH;
                            let force_y = dy * ATTRACTION_STRENGTH;

                            if let Some(f) = forces.get_mut(from_id) {
                                f.0 += force_x;
                                f.1 += force_y;
                            }
                        }
                    }
                }
            }

            // Apply forces with damping
            for node in &mut self.graph_nodes {
                if let Some((fx, fy)) = forces.get(&node.id) {
                    node.x = ((node.x as f32 + fx * DAMPING) as i32).clamp(3, 120);
                    node.y = ((node.y as f32 + fy * DAMPING) as i32).clamp(2, 40);
                }
            }
        }

        // Apply type-based clustering to keep processes and topics somewhat separated
        let mut process_center_x = 0;
        let mut process_center_y = 0;
        let mut process_count = 0;
        let mut topic_center_x = 0;
        let mut topic_center_y = 0;
        let mut topic_count = 0;

        for node in &self.graph_nodes {
            match node.node_type {
                TuiNodeType::Process => {
                    process_center_x += node.x;
                    process_center_y += node.y;
                    process_count += 1;
                }
                TuiNodeType::Topic => {
                    topic_center_x += node.x;
                    topic_center_y += node.y;
                    topic_count += 1;
                }
            }
        }

        if process_count > 0 {
            process_center_x /= process_count;
            process_center_y /= process_count;
        }

        if topic_count > 0 {
            topic_center_x /= topic_count;
            topic_center_y /= topic_count;
        }

        // Gently push types toward their clusters for better organization
        for node in &mut self.graph_nodes {
            match node.node_type {
                TuiNodeType::Process if process_count > 0 => {
                    let dx = process_center_x - node.x;
                    let dy = process_center_y - node.y;
                    node.x += dx / 10;
                    node.y += dy / 10;
                }
                TuiNodeType::Topic if topic_count > 0 => {
                    let dx = topic_center_x - node.x;
                    let dy = topic_center_y - node.y;
                    node.x += dx / 10;
                    node.y += dy / 10;
                }
                _ => {}
            }
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
    use std::collections::HashSet;
    use std::fs;

    let mut workspaces = Vec::new();

    // Use unified workspace discovery
    let discovered = crate::workspace::discover_all_workspaces(current_workspace_path);

    for ws in discovered {
        let env_path_buf = ws.path;
        let horus_dir = env_path_buf.join(".horus");

        // Read dependencies from horus.yaml
        let horus_yaml_path = env_path_buf.join("horus.yaml");
        let yaml_dependencies = if horus_yaml_path.exists() {
            fs::read_to_string(&horus_yaml_path)
                .ok()
                .and_then(|content| serde_yaml::from_str::<serde_yaml::Value>(&content).ok())
                .and_then(|yaml| {
                    yaml.get("dependencies")
                        .and_then(|deps| deps.as_sequence())
                        .map(|seq| {
                            seq.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect::<Vec<String>>()
                        })
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Get packages inside this workspace
        let packages_dir = horus_dir.join("packages");
        let mut packages = Vec::new();
        let mut installed_packages_set: HashSet<String> = HashSet::new();

        if packages_dir.exists() {
            if let Ok(pkg_entries) = fs::read_dir(&packages_dir) {
                for pkg_entry in pkg_entries.flatten() {
                    // Check if it's a directory OR a symlink pointing to a directory
                    let is_pkg_dir = pkg_entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
                        || (pkg_entry
                            .file_type()
                            .map(|t| t.is_symlink())
                            .unwrap_or(false)
                            && pkg_entry.path().is_dir());

                    if is_pkg_dir {
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

                        installed_packages_set.insert(pkg_name.clone());
                        packages.push(PackageData {
                            name: pkg_name,
                            version,
                            installed_packages,
                        });
                    }
                }
            }
        }

        // Process dependencies from horus.yaml - only include those NOT already installed
        let dependencies: Vec<DependencyData> = yaml_dependencies
            .iter()
            .filter_map(|dep_str| {
                let dep_name = dep_str.split('@').next().unwrap_or(dep_str);

                // Skip if already in installed packages
                if installed_packages_set.contains(dep_name) {
                    return None;
                }

                // This dependency is declared but not installed
                Some(DependencyData {
                    name: dep_name.to_string(),
                    declared_version: dep_str.clone(),
                    status: DependencyStatus::Missing,
                })
            })
            .collect();

        // Always add the workspace, even if it has no packages
        workspaces.push(WorkspaceData {
            name: ws.name,
            path: env_path_buf.to_string_lossy().to_string(),
            packages,
            dependencies,
            is_current: ws.is_current,
        });
    }

    // Sort by: current workspace first, then alphabetically
    workspaces.sort_by(|a, b| {
        match (a.is_current, b.is_current) {
            (true, false) => std::cmp::Ordering::Less, // Current workspace comes first
            (false, true) => std::cmp::Ordering::Greater, // Current workspace comes first
            _ => a.name.cmp(&b.name),                  // Otherwise sort alphabetically
        }
    });
    workspaces
}

type PackageInfo = (String, String, String);
type InstalledPackages = (Vec<PackageInfo>, Vec<PackageInfo>);

/// Recursively calculate total size of a directory
fn calculate_dir_size(path: &std::path::Path) -> u64 {
    let mut total = 0;

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    total += metadata.len();
                } else if metadata.is_dir() {
                    total += calculate_dir_size(&entry.path());
                }
            }
        }
    }

    total
}

/// Read version from package metadata.json
fn get_package_version(pkg_path: &std::path::Path) -> String {
    let metadata_path = pkg_path.join("metadata.json");

    if let Ok(content) = std::fs::read_to_string(&metadata_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(version) = json.get("version").and_then(|v| v.as_str()) {
                return version.to_string();
            }
        }
    }

    "unknown".to_string()
}

fn get_installed_packages() -> InstalledPackages {
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
                            let pkg_path = entry.path();

                            // Calculate real directory size
                            let total_bytes = calculate_dir_size(&pkg_path);
                            let size = if total_bytes == 0 {
                                "Unknown".to_string()
                            } else {
                                let kb = total_bytes / 1024;
                                if kb < 1024 {
                                    format!("{} KB", kb)
                                } else {
                                    format!("{:.1} MB", kb as f64 / 1024.0)
                                }
                            };

                            // Read real version from metadata.json
                            let version = get_package_version(&pkg_path);

                            local_packages.push((name.to_string(), version, size));
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
                            let pkg_path = entry.path();

                            // Calculate real directory size
                            let total_bytes = calculate_dir_size(&pkg_path);
                            let size = if total_bytes == 0 {
                                "Unknown".to_string()
                            } else {
                                let kb = total_bytes / 1024;
                                if kb < 1024 {
                                    format!("{} KB", kb)
                                } else {
                                    format!("{:.1} MB", kb as f64 / 1024.0)
                                }
                            };

                            // Read real version from metadata.json
                            let version = get_package_version(&pkg_path);

                            global_packages.push((name.to_string(), version, size));
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

/// Format bytes into human-readable string (B, KB, MB, GB)
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

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

// ============================================================================
// TUI Dashboard Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Tab Navigation Tests
    // ========================================================================

    #[test]
    fn test_tab_as_str() {
        assert_eq!(Tab::Overview.as_str(), "Overview");
        assert_eq!(Tab::Nodes.as_str(), "Nodes");
        assert_eq!(Tab::Topics.as_str(), "Topics");
        assert_eq!(Tab::Graph.as_str(), "Graph");
        assert_eq!(Tab::Packages.as_str(), "Packages");
        assert_eq!(Tab::Parameters.as_str(), "Params");
    }

    #[test]
    fn test_tab_all_returns_all_tabs() {
        let tabs = Tab::all();
        assert_eq!(tabs.len(), 6);
        assert!(tabs.contains(&Tab::Overview));
        assert!(tabs.contains(&Tab::Nodes));
        assert!(tabs.contains(&Tab::Topics));
        assert!(tabs.contains(&Tab::Graph));
        assert!(tabs.contains(&Tab::Packages));
        assert!(tabs.contains(&Tab::Parameters));
    }

    // ========================================================================
    // TuiDashboard State Tests
    // ========================================================================

    #[test]
    fn test_tui_dashboard_new_defaults() {
        let dashboard = TuiDashboard::new();

        // Check initial state
        assert_eq!(dashboard.active_tab, Tab::Overview);
        assert_eq!(dashboard.selected_index, 0);
        assert_eq!(dashboard.scroll_offset, 0);
        assert!(!dashboard.paused);
        assert!(!dashboard.show_help);
        assert!(!dashboard.show_log_panel);
        assert!(dashboard.panel_target.is_none());
        assert_eq!(dashboard.param_edit_mode, ParamEditMode::None);
        assert_eq!(dashboard.package_view_mode, PackageViewMode::List);
        assert_eq!(
            dashboard.package_panel_focus,
            PackagePanelFocus::LocalWorkspaces
        );
        assert_eq!(dashboard.overview_panel_focus, OverviewPanelFocus::Nodes);
        assert_eq!(dashboard.graph_zoom, 1.0);
        assert_eq!(dashboard.graph_offset_x, 0);
        assert_eq!(dashboard.graph_offset_y, 0);
    }

    #[test]
    fn test_tui_dashboard_default_impl() {
        let dashboard1 = TuiDashboard::new();
        let dashboard2 = TuiDashboard::default();

        // Both should have same initial state
        assert_eq!(dashboard1.active_tab, dashboard2.active_tab);
        assert_eq!(dashboard1.selected_index, dashboard2.selected_index);
        assert_eq!(dashboard1.paused, dashboard2.paused);
    }

    // ========================================================================
    // Tab Navigation Logic Tests
    // ========================================================================

    #[test]
    fn test_next_tab_cycles_through_all() {
        let mut dashboard = TuiDashboard::new();
        assert_eq!(dashboard.active_tab, Tab::Overview);

        dashboard.next_tab();
        assert_eq!(dashboard.active_tab, Tab::Nodes);

        dashboard.next_tab();
        assert_eq!(dashboard.active_tab, Tab::Topics);

        dashboard.next_tab();
        assert_eq!(dashboard.active_tab, Tab::Graph);

        dashboard.next_tab();
        assert_eq!(dashboard.active_tab, Tab::Packages);

        dashboard.next_tab();
        assert_eq!(dashboard.active_tab, Tab::Parameters);

        // Should wrap around
        dashboard.next_tab();
        assert_eq!(dashboard.active_tab, Tab::Overview);
    }

    #[test]
    fn test_prev_tab_cycles_backwards() {
        let mut dashboard = TuiDashboard::new();
        assert_eq!(dashboard.active_tab, Tab::Overview);

        // Should wrap to Parameters
        dashboard.prev_tab();
        assert_eq!(dashboard.active_tab, Tab::Parameters);

        dashboard.prev_tab();
        assert_eq!(dashboard.active_tab, Tab::Packages);

        dashboard.prev_tab();
        assert_eq!(dashboard.active_tab, Tab::Graph);
    }

    // ========================================================================
    // Selection Navigation Tests
    // ========================================================================

    #[test]
    fn test_select_next_increments_index() {
        let mut dashboard = TuiDashboard::new();
        dashboard.nodes = vec![
            NodeStatus {
                name: "node1".to_string(),
                status: "running".to_string(),
                priority: 1,
                process_id: 1234,
                cpu_usage: 10.0,
                memory_usage: 1024,
                publishers: vec![],
                subscribers: vec![],
            },
            NodeStatus {
                name: "node2".to_string(),
                status: "running".to_string(),
                priority: 2,
                process_id: 5678,
                cpu_usage: 20.0,
                memory_usage: 2048,
                publishers: vec![],
                subscribers: vec![],
            },
        ];

        assert_eq!(dashboard.selected_index, 0);
        dashboard.select_next();
        assert_eq!(dashboard.selected_index, 1);
    }

    #[test]
    fn test_select_prev_decrements_index() {
        let mut dashboard = TuiDashboard::new();
        dashboard.selected_index = 2;
        dashboard.nodes = vec![
            NodeStatus {
                name: "node1".to_string(),
                status: "running".to_string(),
                priority: 1,
                process_id: 1234,
                cpu_usage: 10.0,
                memory_usage: 1024,
                publishers: vec![],
                subscribers: vec![],
            },
            NodeStatus {
                name: "node2".to_string(),
                status: "running".to_string(),
                priority: 2,
                process_id: 5678,
                cpu_usage: 20.0,
                memory_usage: 2048,
                publishers: vec![],
                subscribers: vec![],
            },
            NodeStatus {
                name: "node3".to_string(),
                status: "running".to_string(),
                priority: 3,
                process_id: 9999,
                cpu_usage: 30.0,
                memory_usage: 3072,
                publishers: vec![],
                subscribers: vec![],
            },
        ];

        dashboard.select_prev();
        assert_eq!(dashboard.selected_index, 1);
        dashboard.select_prev();
        assert_eq!(dashboard.selected_index, 0);
    }

    // ========================================================================
    // Pause Toggle Tests
    // ========================================================================

    #[test]
    fn test_pause_toggle() {
        let mut dashboard = TuiDashboard::new();
        assert!(!dashboard.paused);

        dashboard.paused = !dashboard.paused;
        assert!(dashboard.paused);

        dashboard.paused = !dashboard.paused;
        assert!(!dashboard.paused);
    }

    // ========================================================================
    // Log Panel Tests
    // ========================================================================

    #[test]
    fn test_log_panel_toggle() {
        let mut dashboard = TuiDashboard::new();
        assert!(!dashboard.show_log_panel);
        assert!(dashboard.panel_target.is_none());

        // Simulate opening log panel
        dashboard.show_log_panel = true;
        dashboard.panel_target = Some(LogPanelTarget::Node("test_node".to_string()));

        assert!(dashboard.show_log_panel);
        assert!(dashboard.panel_target.is_some());

        // Check target type
        match &dashboard.panel_target {
            Some(LogPanelTarget::Node(name)) => assert_eq!(name, "test_node"),
            _ => panic!("Expected Node target"),
        }
    }

    #[test]
    fn test_log_panel_target_topic() {
        let mut dashboard = TuiDashboard::new();
        dashboard.show_log_panel = true;
        dashboard.panel_target = Some(LogPanelTarget::Topic("sensors.lidar".to_string()));

        match &dashboard.panel_target {
            Some(LogPanelTarget::Topic(name)) => assert_eq!(name, "sensors.lidar"),
            _ => panic!("Expected Topic target"),
        }
    }

    // ========================================================================
    // Parameter Edit Mode Tests
    // ========================================================================

    #[test]
    fn test_param_edit_modes() {
        let mut dashboard = TuiDashboard::new();
        assert_eq!(dashboard.param_edit_mode, ParamEditMode::None);

        // Test Add mode
        dashboard.param_edit_mode = ParamEditMode::Add;
        assert_eq!(dashboard.param_edit_mode, ParamEditMode::Add);

        // Test Edit mode
        dashboard.param_edit_mode = ParamEditMode::Edit("my_key".to_string());
        match &dashboard.param_edit_mode {
            ParamEditMode::Edit(key) => assert_eq!(key, "my_key"),
            _ => panic!("Expected Edit mode"),
        }

        // Test Delete mode
        dashboard.param_edit_mode = ParamEditMode::Delete("delete_key".to_string());
        match &dashboard.param_edit_mode {
            ParamEditMode::Delete(key) => assert_eq!(key, "delete_key"),
            _ => panic!("Expected Delete mode"),
        }
    }

    #[test]
    fn test_param_input_focus() {
        let mut dashboard = TuiDashboard::new();
        assert_eq!(dashboard.param_input_focus, ParamInputFocus::Key);

        dashboard.param_input_focus = ParamInputFocus::Value;
        assert_eq!(dashboard.param_input_focus, ParamInputFocus::Value);
    }

    // ========================================================================
    // Package View Mode Tests
    // ========================================================================

    #[test]
    fn test_package_view_modes() {
        let mut dashboard = TuiDashboard::new();
        assert_eq!(dashboard.package_view_mode, PackageViewMode::List);

        dashboard.package_view_mode = PackageViewMode::WorkspaceDetails;
        assert_eq!(
            dashboard.package_view_mode,
            PackageViewMode::WorkspaceDetails
        );
    }

    #[test]
    fn test_package_panel_focus() {
        let mut dashboard = TuiDashboard::new();
        assert_eq!(
            dashboard.package_panel_focus,
            PackagePanelFocus::LocalWorkspaces
        );

        dashboard.package_panel_focus = PackagePanelFocus::GlobalPackages;
        assert_eq!(
            dashboard.package_panel_focus,
            PackagePanelFocus::GlobalPackages
        );
    }

    // ========================================================================
    // Overview Panel Focus Tests
    // ========================================================================

    #[test]
    fn test_overview_panel_focus() {
        let mut dashboard = TuiDashboard::new();
        assert_eq!(dashboard.overview_panel_focus, OverviewPanelFocus::Nodes);

        dashboard.overview_panel_focus = OverviewPanelFocus::Topics;
        assert_eq!(dashboard.overview_panel_focus, OverviewPanelFocus::Topics);
    }

    // ========================================================================
    // Graph State Tests
    // ========================================================================

    #[test]
    fn test_graph_zoom_bounds() {
        let mut dashboard = TuiDashboard::new();
        assert_eq!(dashboard.graph_zoom, 1.0);

        // Simulate zoom in
        dashboard.graph_zoom = 2.0;
        assert_eq!(dashboard.graph_zoom, 2.0);

        // Simulate zoom out
        dashboard.graph_zoom = 0.5;
        assert_eq!(dashboard.graph_zoom, 0.5);
    }

    #[test]
    fn test_graph_offset() {
        let mut dashboard = TuiDashboard::new();
        assert_eq!(dashboard.graph_offset_x, 0);
        assert_eq!(dashboard.graph_offset_y, 0);

        // Simulate panning
        dashboard.graph_offset_x = 10;
        dashboard.graph_offset_y = -5;

        assert_eq!(dashboard.graph_offset_x, 10);
        assert_eq!(dashboard.graph_offset_y, -5);
    }

    #[test]
    fn test_graph_layout_default() {
        let dashboard = TuiDashboard::new();
        assert_eq!(dashboard.graph_layout, GraphLayout::Hierarchical);
    }

    // ========================================================================
    // Data Model Tests
    // ========================================================================

    #[test]
    fn test_node_status_creation() {
        let node = NodeStatus {
            name: "test_node".to_string(),
            status: "running".to_string(),
            priority: 1,
            process_id: 12345,
            cpu_usage: 25.5,
            memory_usage: 1024 * 1024,
            publishers: vec!["topic1".to_string(), "topic2".to_string()],
            subscribers: vec!["topic3".to_string()],
        };

        assert_eq!(node.name, "test_node");
        assert_eq!(node.status, "running");
        assert_eq!(node.priority, 1);
        assert_eq!(node.process_id, 12345);
        assert!((node.cpu_usage - 25.5).abs() < 0.001);
        assert_eq!(node.memory_usage, 1024 * 1024);
        assert_eq!(node.publishers.len(), 2);
        assert_eq!(node.subscribers.len(), 1);
    }

    #[test]
    fn test_topic_info_creation() {
        let topic = TopicInfo {
            name: "sensors.lidar".to_string(),
            msg_type: "LidarScan".to_string(),
            publishers: 2,
            subscribers: 3,
            rate: 10.0,
            publisher_nodes: vec!["node1".to_string(), "node2".to_string()],
            subscriber_nodes: vec![
                "node3".to_string(),
                "node4".to_string(),
                "node5".to_string(),
            ],
        };

        assert_eq!(topic.name, "sensors.lidar");
        assert_eq!(topic.msg_type, "LidarScan");
        assert_eq!(topic.publishers, 2);
        assert_eq!(topic.subscribers, 3);
        assert!((topic.rate - 10.0).abs() < 0.001);
        assert_eq!(topic.publisher_nodes.len(), 2);
        assert_eq!(topic.subscriber_nodes.len(), 3);
    }

    #[test]
    fn test_workspace_data_creation() {
        let workspace = WorkspaceData {
            name: "my_robot".to_string(),
            path: "/home/user/my_robot".to_string(),
            packages: vec![PackageData {
                name: "controller".to_string(),
                version: "1.0.0".to_string(),
                installed_packages: vec![("lidar_driver".to_string(), "0.5.0".to_string())],
            }],
            dependencies: vec![DependencyData {
                name: "slam".to_string(),
                declared_version: "2.0.0".to_string(),
                status: DependencyStatus::Missing,
            }],
            is_current: true,
        };

        assert_eq!(workspace.name, "my_robot");
        assert!(workspace.is_current);
        assert_eq!(workspace.packages.len(), 1);
        assert_eq!(workspace.dependencies.len(), 1);
    }

    #[test]
    fn test_dependency_status() {
        assert_ne!(DependencyStatus::Missing, DependencyStatus::Installed);

        let dep = DependencyData {
            name: "test_dep".to_string(),
            declared_version: "1.0.0".to_string(),
            status: DependencyStatus::Missing,
        };
        assert_eq!(dep.status, DependencyStatus::Missing);
    }

    // ========================================================================
    // Workspace Cache Tests
    // ========================================================================

    #[test]
    fn test_workspace_cache_initialization() {
        let dashboard = TuiDashboard::new();

        // Cache should be empty initially
        assert!(dashboard.workspace_cache.is_empty());

        // Cache time should be set to force initial load
        assert!(dashboard.workspace_cache_time.elapsed().as_secs() >= 5);
    }

    // ========================================================================
    // Graph Node and Edge Tests
    // ========================================================================

    #[test]
    fn test_tui_graph_node_creation() {
        let node = TuiGraphNode {
            id: "node1".to_string(),
            label: "Node 1".to_string(),
            node_type: TuiNodeType::Process,
            x: 100,
            y: 200,
            pid: Some(1234),
            active: true,
        };

        assert_eq!(node.id, "node1");
        assert_eq!(node.label, "Node 1");
        assert_eq!(node.node_type, TuiNodeType::Process);
        assert_eq!(node.x, 100);
        assert_eq!(node.y, 200);
        assert_eq!(node.pid, Some(1234));
        assert!(node.active);
    }

    #[test]
    fn test_tui_graph_edge_creation() {
        let edge = TuiGraphEdge {
            from: "node1".to_string(),
            to: "topic1".to_string(),
            edge_type: TuiEdgeType::Publish,
            active: true,
        };

        assert_eq!(edge.from, "node1");
        assert_eq!(edge.to, "topic1");
        assert_eq!(edge.edge_type, TuiEdgeType::Publish);
        assert!(edge.active);
    }

    #[test]
    fn test_tui_node_types() {
        assert_ne!(TuiNodeType::Process, TuiNodeType::Topic);

        let process_node = TuiGraphNode {
            id: "p1".to_string(),
            label: "Process".to_string(),
            node_type: TuiNodeType::Process,
            x: 0,
            y: 0,
            pid: Some(1000),
            active: true,
        };

        let topic_node = TuiGraphNode {
            id: "t1".to_string(),
            label: "Topic".to_string(),
            node_type: TuiNodeType::Topic,
            x: 0,
            y: 0,
            pid: None,
            active: true,
        };

        assert_eq!(process_node.node_type, TuiNodeType::Process);
        assert_eq!(topic_node.node_type, TuiNodeType::Topic);
    }

    #[test]
    fn test_tui_edge_types() {
        assert_ne!(TuiEdgeType::Publish, TuiEdgeType::Subscribe);

        let pub_edge = TuiGraphEdge {
            from: "a".to_string(),
            to: "b".to_string(),
            edge_type: TuiEdgeType::Publish,
            active: true,
        };

        let sub_edge = TuiGraphEdge {
            from: "c".to_string(),
            to: "d".to_string(),
            edge_type: TuiEdgeType::Subscribe,
            active: false,
        };

        assert_eq!(pub_edge.edge_type, TuiEdgeType::Publish);
        assert_eq!(sub_edge.edge_type, TuiEdgeType::Subscribe);
    }

    #[test]
    fn test_graph_node_inactive() {
        let inactive_node = TuiGraphNode {
            id: "inactive".to_string(),
            label: "Inactive Node".to_string(),
            node_type: TuiNodeType::Process,
            x: 50,
            y: 50,
            pid: None,
            active: false,
        };

        assert!(!inactive_node.active);
        assert!(inactive_node.pid.is_none());
    }
}
