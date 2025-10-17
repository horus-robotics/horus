use eframe::egui::{self, *};
use std::sync::{Arc, Mutex};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

// Graph node representation
#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub node_type: NodeType,
    pub position: Pos2,
    pub velocity: Vec2,
    pub pid: Option<u32>,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Process,
    Topic,
}

// Edge type to distinguish publishers from subscribers
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeType {
    Publish,    // Process publishes to topic
    Subscribe,  // Process subscribes from topic
}

// Graph edge representation
#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub edge_type: EdgeType,
    pub active: bool,
}

// Graph visualization state
pub struct GraphVisualization {
    nodes: HashMap<String, GraphNode>,
    edges: Vec<GraphEdge>,
    selected_node: Option<String>,
    dragging_node: Option<String>,
    camera_offset: Vec2,
    zoom: f32,
    auto_layout: bool,
    show_labels: bool,
    physics_enabled: bool,
    last_refresh: std::time::Instant,
    refresh_callback: Arc<Mutex<Box<dyn Fn() -> (Vec<GraphNode>, Vec<GraphEdge>) + Send>>>,
    search_query: String,
    highlighted_nodes: HashSet<String>,
    dark_mode: bool,
    graph_theme: GraphTheme,
    layout_algorithm: LayoutAlgorithm,
    force_strength: f32,
    repulsion_strength: f32,
}

#[derive(Debug, Clone, PartialEq)]
enum LayoutAlgorithm {
    ForceDirected,
    Hierarchical,
    Circular,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GraphTheme {
    Standard,     // Original near-black theme
    FullBlack,    // New fully black inverse theme
}

impl GraphVisualization {
    pub fn new(refresh_callback: Box<dyn Fn() -> (Vec<GraphNode>, Vec<GraphEdge>) + Send>) -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            selected_node: None,
            dragging_node: None,
            camera_offset: Vec2::ZERO,
            zoom: 1.0,
            auto_layout: true,
            show_labels: true,
            physics_enabled: true,
            last_refresh: std::time::Instant::now(),
            refresh_callback: Arc::new(Mutex::new(refresh_callback)),
            search_query: String::new(),
            highlighted_nodes: HashSet::new(),
            dark_mode: true,
            graph_theme: GraphTheme::FullBlack,  // Start with new fully black theme
            layout_algorithm: LayoutAlgorithm::ForceDirected,
            force_strength: 0.3,       // Almost no attraction
            repulsion_strength: 1.0,   // Minimal repulsion - very subtle
        }
    }

    fn refresh_data(&mut self) {
        let (new_nodes, new_edges) = {
            let callback = self.refresh_callback.lock().unwrap();
            callback()
        };

        // First, collect all new nodes that need positions
        let mut nodes_to_add: Vec<(String, GraphNode)> = Vec::new();

        // Count by type for proper indexing
        let mut process_index = self.nodes.values()
            .filter(|n| n.node_type == NodeType::Process)
            .count();
        let mut topic_index = self.nodes.values()
            .filter(|n| n.node_type == NodeType::Topic)
            .count();

        for node in &new_nodes {
            if !self.nodes.contains_key(&node.id) {
                // New node - assign initial position with correct index
                let mut new_node = node.clone();

                // Get the index for this specific node type
                let index = match node.node_type {
                    NodeType::Process => {
                        let idx = process_index;
                        process_index += 1;
                        idx
                    }
                    NodeType::Topic => {
                        let idx = topic_index;
                        topic_index += 1;
                        idx
                    }
                };

                new_node.position = self.get_initial_position_indexed(&node.id, &node.node_type, index);
                nodes_to_add.push((new_node.id.clone(), new_node));
            } else {
                // Existing node - update data but keep position
                if let Some(existing) = self.nodes.get_mut(&node.id) {
                    existing.label = node.label.clone();
                    existing.node_type = node.node_type.clone();
                    existing.pid = node.pid;
                    existing.active = node.active;
                }
            }
        }

        // Now insert all new nodes
        for (id, node) in nodes_to_add {
            self.nodes.insert(id, node);
        }

        // Remove nodes that no longer exist
        let current_ids: HashSet<String> = new_nodes.iter().map(|n| n.id.clone()).collect();
        self.nodes.retain(|id, _| current_ids.contains(id));

        // Update edges
        self.edges = new_edges;

        // Auto-center view if this is the first time we have nodes and camera hasn't been moved
        if !self.nodes.is_empty() && self.camera_offset == Vec2::ZERO && self.zoom == 1.0 {
            // Delay centering slightly to allow UI to establish size
            if self.last_refresh.elapsed().as_millis() > 100 {
                self.center_view(Vec2::new(800.0, 600.0));
            }
        }

        self.last_refresh = std::time::Instant::now();
    }

    fn get_initial_position_indexed(&self, node_id: &str, node_type: &NodeType, index: usize) -> Pos2 {
        // Hash the node ID to get a deterministic unique value for variation
        let mut hash: u64 = 0;
        for byte in node_id.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }

        match node_type {
            NodeType::Process => {
                // Processes: Vertical column on LEFT side with good spacing
                let x_base = -300.0;  // Fixed X position on left
                let x_variation = (hash % 80) as f32 - 40.0;  // ±40px horizontal variation
                let vertical_spacing = 120.0;  // Good vertical spacing
                let y_variation = ((hash / 100) % 40) as f32 - 20.0;  // ±20px vertical variation

                Pos2::new(
                    x_base + x_variation,
                    index as f32 * vertical_spacing + y_variation
                )
            }
            NodeType::Topic => {
                // Topics: Vertical column on RIGHT side with wide spacing
                let x_base = 300.0;  // Fixed X position on right
                let x_variation = (hash % 100) as f32 - 50.0;  // ±50px horizontal variation
                let vertical_spacing = 140.0;  // Even wider vertical spacing
                let y_variation = ((hash / 100) % 50) as f32 - 25.0;  // ±25px vertical variation

                Pos2::new(
                    x_base + x_variation,
                    index as f32 * vertical_spacing + y_variation
                )
            }
        }
    }

    fn center_view(&mut self, available_size: Vec2) {
        if self.nodes.is_empty() {
            self.camera_offset = Vec2::ZERO;
            return;
        }

        // Calculate bounding box of all nodes
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for node in self.nodes.values() {
            min_x = min_x.min(node.position.x);
            max_x = max_x.max(node.position.x);
            min_y = min_y.min(node.position.y);
            max_y = max_y.max(node.position.y);
        }

        // Calculate center of nodes
        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;

        // Calculate required zoom to fit all nodes with padding
        let width = max_x - min_x + 200.0; // Add padding
        let height = max_y - min_y + 200.0;

        let zoom_x = available_size.x / width;
        let zoom_y = available_size.y / height;
        let target_zoom = (zoom_x.min(zoom_y) * 0.8).clamp(0.1, 2.0); // 80% to leave margin

        // Set camera to center the nodes in the viewport
        self.camera_offset = available_size / 2.0 - Vec2::new(center_x, center_y) * target_zoom;
        self.zoom = target_zoom;
    }

    fn apply_physics(&mut self, dt: f32) {
        if !self.physics_enabled {
            return;
        }

        match self.layout_algorithm {
            LayoutAlgorithm::ForceDirected => self.apply_force_directed_layout(dt),
            LayoutAlgorithm::Hierarchical => self.apply_hierarchical_layout(),
            LayoutAlgorithm::Circular => self.apply_circular_layout(),
        }
    }

    fn apply_force_directed_layout(&mut self, dt: f32) {
        // Reset velocities
        for node in self.nodes.values_mut() {
            node.velocity = Vec2::ZERO;
        }

        // Apply very gentle repulsion between all nodes - prevent stacking
        let node_ids: Vec<String> = self.nodes.keys().cloned().collect();
        let min_distance = 60.0;   // Absolute minimum - nodes can't get closer
        let ideal_distance = 100.0;  // Target comfortable spacing

        for i in 0..node_ids.len() {
            for j in i + 1..node_ids.len() {
                let id1 = &node_ids[i];
                let id2 = &node_ids[j];

                if let (Some(node1), Some(node2)) = (self.nodes.get(id1), self.nodes.get(id2)) {
                    let diff = node2.position - node1.position;
                    let dist = diff.length();

                    // Safety: If nodes are stacked (very close), add small offset
                    if dist < 5.0 {
                        // Nodes are on top of each other - add tiny random offset
                        let offset_x = ((i * 17 + j * 31) % 50) as f32 - 25.0;
                        let offset_y = ((i * 23 + j * 41) % 50) as f32 - 25.0;

                        if let Some(node2) = self.nodes.get_mut(id2) {
                            node2.position.x += offset_x;
                            node2.position.y += offset_y;
                        }
                        continue;
                    }

                    // Only apply gentle repulsion if nodes are closer than ideal
                    if dist < ideal_distance {
                        // Very gentle, smooth repulsion
                        let repel_factor = (ideal_distance - dist) / ideal_distance;
                        let force = self.repulsion_strength * repel_factor * diff.normalized();

                        if let Some(node1) = self.nodes.get_mut(id1) {
                            node1.velocity -= force;
                        }
                        if let Some(node2) = self.nodes.get_mut(id2) {
                            node2.velocity += force;
                        }
                    }
                }
            }
        }

        // Apply attraction along edges - keep connected nodes together
        for edge in &self.edges {
            if let (Some(node1), Some(node2)) = (self.nodes.get(&edge.from), self.nodes.get(&edge.to)) {
                let diff = node2.position - node1.position;
                let dist = diff.length().max(1.0);

                // Only pull together if they're too far apart
                if dist > 200.0 {
                    let force = (self.force_strength * (dist - 200.0) / dist) * diff.normalized();

                    if let Some(node1) = self.nodes.get_mut(&edge.from) {
                        node1.velocity += force * 0.3;
                    }
                    if let Some(node2) = self.nodes.get_mut(&edge.to) {
                        node2.velocity -= force * 0.3;
                    }
                }
            }
        }

        // Apply velocities with strong damping and velocity limiting
        let max_velocity = 0.3;  // Extremely low speed cap - almost imperceptible
        for node in self.nodes.values_mut() {
            if self.dragging_node.as_ref() != Some(&node.id) {
                // Limit velocity to prevent wild movements
                let speed = node.velocity.length();
                if speed > max_velocity {
                    node.velocity = node.velocity.normalized() * max_velocity;
                }

                node.position += node.velocity * dt * 0.003;  // Extremely slow, almost imperceptible movement
                node.velocity *= 0.95; // Very strong damping - quick settling
            }
        }
    }

    fn apply_hierarchical_layout(&mut self) {
        // Separate processes and topics
        let mut processes: Vec<String> = Vec::new();
        let mut topics: Vec<String> = Vec::new();
        
        for (id, node) in &self.nodes {
            match node.node_type {
                NodeType::Process => processes.push(id.clone()),
                NodeType::Topic => topics.push(id.clone()),
            }
        }

        // Layout processes on top
        let process_spacing = 150.0;
        for (i, id) in processes.iter().enumerate() {
            if let Some(node) = self.nodes.get_mut(id) {
                node.position = Pos2::new(
                    200.0 + i as f32 * process_spacing,
                    200.0
                );
            }
        }

        // Layout topics on bottom
        let topic_spacing = 120.0;
        for (i, id) in topics.iter().enumerate() {
            if let Some(node) = self.nodes.get_mut(id) {
                node.position = Pos2::new(
                    200.0 + i as f32 * topic_spacing,
                    400.0
                );
            }
        }
    }

    fn apply_circular_layout(&mut self) {
        let center = Pos2::new(400.0, 300.0);
        let radius = 250.0;
        let node_count = self.nodes.len();
        
        for (i, (_, node)) in self.nodes.iter_mut().enumerate() {
            let angle = (i as f32 / node_count as f32) * std::f32::consts::TAU;
            node.position = center + Vec2::new(angle.cos(), angle.sin()) * radius;
        }
    }

    fn world_to_screen(&self, pos: Pos2) -> Pos2 {
        (pos.to_vec2() * self.zoom + self.camera_offset).to_pos2()
    }

    fn screen_to_world(&self, pos: Pos2) -> Pos2 {
        ((pos.to_vec2() - self.camera_offset) / self.zoom).to_pos2()
    }

    fn update_search_highlights(&mut self) {
        self.highlighted_nodes.clear();

        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            for (id, node) in &self.nodes {
                if node.label.to_lowercase().contains(&query) {
                    self.highlighted_nodes.insert(id.clone());
                }
            }
        }
    }

    // Render method for embedding in another UI
    fn get_theme_colors(&self) -> (Color32, Color32, Color32, Color32, Color32, Color32, Color32) {
        match self.graph_theme {
            GraphTheme::Standard => {
                // Original near-black theme
                if self.dark_mode {
                    (
                        Color32::from_gray(30),                               // background
                        Color32::from_rgba_premultiplied(100, 110, 120, 20), // dot_color
                        Color32::from_rgb(70, 130, 180),                     // process_color
                        Color32::from_rgb(255, 215, 0),                      // topic_color
                        Color32::from_rgb(34, 197, 94),                      // publish_edge
                        Color32::from_rgb(59, 130, 246),                     // subscribe_edge
                        Color32::WHITE,                                       // text_color
                    )
                } else {
                    (
                        Color32::from_gray(240),                               // background
                        Color32::from_rgba_premultiplied(180, 185, 190, 30), // dot_color
                        Color32::from_rgb(40, 80, 120),                       // process_color
                        Color32::from_rgb(180, 140, 0),                       // topic_color
                        Color32::from_rgb(34, 150, 80),                       // publish_edge
                        Color32::from_rgb(40, 100, 200),                      // subscribe_edge
                        Color32::BLACK,                                        // text_color
                    )
                }
            },
            GraphTheme::FullBlack => {
                // New fully black inverse theme
                (
                    Color32::BLACK,                                        // background - pure black
                    Color32::from_rgba_premultiplied(255, 255, 255, 15), // dot_color - bright white dots
                    Color32::from_rgb(0, 255, 127),                      // process_color - bright green
                    Color32::from_rgb(255, 20, 147),                     // topic_color - deep pink
                    Color32::from_rgb(0, 255, 255),                      // publish_edge - cyan
                    Color32::from_rgb(255, 69, 0),                       // subscribe_edge - red orange
                    Color32::WHITE,                                        // text_color - white
                )
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        // Auto refresh every 2 seconds
        if self.last_refresh.elapsed() > Duration::from_secs(2) {
            self.refresh_data();
        }

        // Apply physics
        self.apply_physics(0.016); // ~60 FPS

        // Controls bar
        ui.horizontal(|ui| {
            // Search
            ui.label("Search:");
            if ui.text_edit_singleline(&mut self.search_query).changed() {
                self.update_search_highlights();
            }

            ui.separator();

            // Layout selector
            ui.label("Layout:");
            egui::ComboBox::from_id_source("layout_selector")
                .selected_text(format!("{:?}", self.layout_algorithm))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.layout_algorithm, LayoutAlgorithm::ForceDirected, "Force Directed");
                    ui.selectable_value(&mut self.layout_algorithm, LayoutAlgorithm::Hierarchical, "Hierarchical");
                    ui.selectable_value(&mut self.layout_algorithm, LayoutAlgorithm::Circular, "Circular");
                });

            // Options
            ui.checkbox(&mut self.physics_enabled, "Physics");
            ui.checkbox(&mut self.show_labels, "Labels");

            ui.separator();

            // Zoom controls
            if ui.button("Zoom In").clicked() {
                self.zoom *= 1.2;
            }
            ui.label(format!("{:.0}%", self.zoom * 100.0));
            if ui.button("Zoom Out").clicked() {
                self.zoom /= 1.2;
            }
            if ui.button("Reset").clicked() {
                self.center_view(ui.available_size());
            }
            if ui.button("Center").clicked() {
                self.center_view(ui.available_size());
            }

            ui.separator();

            if ui.button("Refresh").clicked() {
                self.refresh_data();
            }
        });

        ui.separator();

        // Main graph area
        let response = ui.allocate_response(ui.available_size(), Sense::click_and_drag());
        let painter = ui.painter();
        let rect = response.rect;

        // Handle pan with middle mouse or right mouse
        if response.dragged_by(PointerButton::Middle) || response.dragged_by(PointerButton::Secondary) {
            self.camera_offset += response.drag_delta();
        }

        // Handle zoom with scroll wheel
        if let Some(pointer) = ui.ctx().input(|i| i.pointer.hover_pos()) {
            if rect.contains(pointer) {
                let scroll = ui.ctx().input(|i| i.scroll_delta.y);
                if scroll != 0.0 {
                    let zoom_factor = 1.0 + scroll * 0.001;
                    self.zoom *= zoom_factor;
                    self.zoom = self.zoom.clamp(0.1, 5.0);
                }
            }
        }

        // Get theme colors
        let (background_color, _dot_color, process_color, topic_color, publish_edge_color, subscribe_edge_color, text_color) = self.get_theme_colors();

        // Background
        painter.rect_filled(rect, 0.0, background_color);

        // Draw edges
        for edge in &self.edges {
            if let (Some(from_node), Some(to_node)) = (self.nodes.get(&edge.from), self.nodes.get(&edge.to)) {
                let from_pos = self.world_to_screen(from_node.position) + rect.min.to_vec2();
                let to_pos = self.world_to_screen(to_node.position) + rect.min.to_vec2();

                let color = match edge.edge_type {
                    EdgeType::Publish => publish_edge_color,
                    EdgeType::Subscribe => subscribe_edge_color,
                };

                let stroke = if edge.active {
                    Stroke::new(2.0 * self.zoom, color)
                } else {
                    Stroke::new(1.0 * self.zoom, color.gamma_multiply(0.5))
                };

                painter.line_segment([from_pos, to_pos], stroke);

                // Draw arrow
                let dir = (to_pos - from_pos).normalized();
                let arrow_size = 10.0 * self.zoom;
                let arrow_angle = std::f32::consts::PI / 6.0;

                let arrow_point1 = to_pos - dir * arrow_size;
                let arrow_point2 = arrow_point1 - Vec2::angled(dir.angle() + arrow_angle) * arrow_size;
                let arrow_point3 = arrow_point1 - Vec2::angled(dir.angle() - arrow_angle) * arrow_size;

                painter.line_segment([to_pos, arrow_point2], stroke);
                painter.line_segment([to_pos, arrow_point3], stroke);
            }
        }

        // Draw nodes
        for (id, node) in &self.nodes {
            let pos = self.world_to_screen(node.position) + rect.min.to_vec2();

            let is_highlighted = self.highlighted_nodes.contains(id);
            let is_selected = self.selected_node.as_ref() == Some(id);

            let radius = match node.node_type {
                NodeType::Process => 20.0 * self.zoom,
                NodeType::Topic => 15.0 * self.zoom,
            };

            let fill_color = match node.node_type {
                NodeType::Process => {
                    if node.active {
                        process_color
                    } else {
                        process_color.gamma_multiply(0.5)
                    }
                }
                NodeType::Topic => {
                    if node.active {
                        topic_color
                    } else {
                        topic_color.gamma_multiply(0.5)
                    }
                }
            };

            let stroke = if is_selected {
                Stroke::new(3.0, Color32::from_rgb(255, 200, 0))
            } else if is_highlighted {
                Stroke::new(2.0, Color32::from_rgb(255, 255, 0))
            } else {
                Stroke::new(1.0, text_color.gamma_multiply(0.8))
            };

            // Draw node
            match node.node_type {
                NodeType::Process => painter.circle(pos, radius, fill_color, stroke),
                NodeType::Topic => painter.rect_filled(
                    Rect::from_center_size(pos, Vec2::splat(radius * 2.0)),
                    Rounding::same(4.0),
                    fill_color,
                ),
            }

            // Draw label
            if self.show_labels {
                painter.text(
                    pos + Vec2::new(0.0, radius + 5.0),
                    egui::Align2::CENTER_TOP,
                    &node.label,
                    FontId::proportional(12.0 * self.zoom),
                    text_color,
                );
            }

            // Handle node interaction
            let node_rect = Rect::from_center_size(pos, Vec2::splat(radius * 2.0));
            if node_rect.contains(response.hover_pos().unwrap_or_default()) {
                if response.clicked() {
                    self.selected_node = Some(id.clone());
                }
                if response.drag_started_by(PointerButton::Primary) {
                    self.dragging_node = Some(id.clone());
                }
            }
        }

        // Handle node dragging
        if response.dragged_by(PointerButton::Primary) {
            if let Some(dragging_id) = &self.dragging_node {
                if let Some(node) = self.nodes.get_mut(dragging_id) {
                    let delta = response.drag_delta() / self.zoom;
                    node.position += delta;
                    node.velocity = Vec2::ZERO; // Stop physics for dragged node
                }
            }
        }

        if response.drag_released() {
            self.dragging_node = None;
        }
    }
}

impl eframe::App for GraphVisualization {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Auto refresh every 2 seconds
        if self.last_refresh.elapsed() > Duration::from_secs(2) {
            self.refresh_data();
        }

        // Apply theme
        let visuals = if self.dark_mode {
            Visuals::dark()
        } else {
            Visuals::light()
        };
        ctx.set_visuals(visuals);

        // Top panel with controls
        egui::TopBottomPanel::top("graph_controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("HORUS Node-Topic Graph");
                
                ui.separator();
                
                // Search
                ui.label("Search:");
                if ui.text_edit_singleline(&mut self.search_query).changed() {
                    self.update_search_highlights();
                }
                
                ui.separator();
                
                // Layout controls
                ui.label("Layout:");
                ui.selectable_value(&mut self.layout_algorithm, LayoutAlgorithm::ForceDirected, "Force");
                ui.selectable_value(&mut self.layout_algorithm, LayoutAlgorithm::Hierarchical, "Hierarchy");
                ui.selectable_value(&mut self.layout_algorithm, LayoutAlgorithm::Circular, "Circle");
                
                ui.separator();
                
                // Options
                ui.checkbox(&mut self.physics_enabled, "Physics");
                ui.checkbox(&mut self.show_labels, "Labels");
                ui.checkbox(&mut self.dark_mode, "Dark Mode");

                // Theme selector
                ui.horizontal(|ui| {
                    ui.label("Theme:");
                    ui.selectable_value(&mut self.graph_theme, GraphTheme::Standard, "Standard");
                    ui.selectable_value(&mut self.graph_theme, GraphTheme::FullBlack, "Full Black");
                });

                ui.separator();
                
                // Zoom controls
                if ui.button("Zoom In").clicked() {
                    self.zoom *= 1.2;
                }
                ui.label(format!("{:.0}%", self.zoom * 100.0));
                if ui.button("Zoom Out").clicked() {
                    self.zoom /= 1.2;
                }
                if ui.button("Reset").clicked() {
                    self.center_view(Vec2::new(800.0, 600.0)); // Reasonable default size
                }
                if ui.button("Center").clicked() {
                    self.center_view(Vec2::new(800.0, 600.0));
                }

                ui.separator();
                
                if ui.button("Refresh").clicked() {
                    self.refresh_data();
                }
                
                ui.separator();
                
                // Legend
                ui.label("Legend:");
                ui.horizontal(|ui| {
                    let (_, _, _, _, publish_color, subscribe_color, _) = self.get_theme_colors();
                    ui.colored_label(publish_color, ">");
                    ui.label("Publishes (output)");
                    ui.separator();
                    ui.colored_label(subscribe_color, "<");
                    ui.label("Subscribes (input)");
                });
                ui.horizontal(|ui| {
                    ui.small("> = Publisher (data sent)");
                    ui.separator();
                    ui.small("< = Subscriber (data received)");
                });
            });
        });

        // Side panel with node details
        egui::SidePanel::right("node_details")
            .min_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Node Details");
                ui.separator();
                
                if let Some(selected_id) = &self.selected_node {
                    if let Some(node) = self.nodes.get(selected_id) {
                        ui.label(format!("Name: {}", node.label));
                        ui.label(format!("Type: {:?}", node.node_type));
                        if let Some(pid) = node.pid {
                            ui.label(format!("PID: {}", pid));
                        }
                        ui.label(format!("Active: {}", if node.active { "Yes" } else { "No" }));
                        
                        ui.separator();
                        
                        // Show connected edges with types
                        ui.label("Connections:");
                        
                        // Show what this node publishes to
                        let mut has_publishes = false;
                        for edge in &self.edges {
                            if edge.from == *selected_id && edge.edge_type == EdgeType::Publish {
                                if let Some(target) = self.nodes.get(&edge.to) {
                                    if !has_publishes {
                                        ui.label("Publishes to:");
                                        has_publishes = true;
                                    }
                                    ui.label(format!("  → {}", target.label));
                                }
                            }
                        }
                        
                        // Show what this node subscribes from
                        let mut has_subscribes = false;
                        for edge in &self.edges {
                            if edge.to == *selected_id && edge.edge_type == EdgeType::Subscribe {
                                if let Some(source) = self.nodes.get(&edge.from) {
                                    if !has_subscribes {
                                        ui.label("Subscribes from:");
                                        has_subscribes = true;
                                    }
                                    ui.label(format!("  ← {}", source.label));
                                }
                            }
                        }
                        
                        // For topics, show publishers and subscribers
                        if node.node_type == NodeType::Topic {
                            let mut publishers = Vec::new();
                            let mut subscribers = Vec::new();
                            
                            for edge in &self.edges {
                                if edge.to == *selected_id && edge.edge_type == EdgeType::Publish {
                                    if let Some(pub_node) = self.nodes.get(&edge.from) {
                                        publishers.push(pub_node.label.clone());
                                    }
                                } else if edge.from == *selected_id && edge.edge_type == EdgeType::Subscribe {
                                    if let Some(sub_node) = self.nodes.get(&edge.to) {
                                        subscribers.push(sub_node.label.clone());
                                    }
                                }
                            }
                            
                            if !publishers.is_empty() {
                                ui.label("Publishers:");
                                for pub_name in publishers {
                                    ui.label(format!("  ← {}", pub_name));
                                }
                            }
                            
                            if !subscribers.is_empty() {
                                ui.label("Subscribers:");
                                for sub_name in subscribers {
                                    ui.label(format!("  → {}", sub_name));
                                }
                            }
                        }
                    }
                } else {
                    ui.label("Click a node to see details");
                }
                
                ui.separator();
                
                // Statistics
                ui.heading("Statistics");
                ui.label(format!("Nodes: {}", self.nodes.len()));
                ui.label(format!("Edges: {}", self.edges.len()));
                
                let process_count = self.nodes.values()
                    .filter(|n| n.node_type == NodeType::Process)
                    .count();
                let topic_count = self.nodes.values()
                    .filter(|n| n.node_type == NodeType::Topic)
                    .count();
                
                ui.label(format!("Processes: {}", process_count));
                ui.label(format!("Topics: {}", topic_count));
            });

        // Main graph area
        egui::CentralPanel::default().show(ctx, |ui| {
            let response = ui.allocate_response(ui.available_size(), Sense::click_and_drag());
            let painter = ui.painter();
            let rect = response.rect;
            
            // Handle pan with middle mouse or right mouse
            if response.dragged_by(PointerButton::Middle) || response.dragged_by(PointerButton::Secondary) {
                self.camera_offset += response.drag_delta();
            }
            
            // Handle zoom with scroll
            if response.hovered() {
                let scroll_delta = ui.input(|i| i.scroll_delta.y);
                if scroll_delta != 0.0 {
                    let zoom_factor = 1.0 + scroll_delta * 0.001;
                    self.zoom *= zoom_factor;
                    self.zoom = self.zoom.clamp(0.1, 5.0);
                }
            }
            
            // Apply physics
            self.apply_physics(0.016); // ~60 FPS

            // Get theme colors
            let (background_color, dot_color, process_color, topic_color, publish_edge_color, subscribe_edge_color, text_color) = self.get_theme_colors();

            // Background
            painter.rect_filled(rect, 0.0, background_color);

            // Check for bidirectional edges and create offset map
            let mut edge_offsets: HashMap<(String, String), f32> = HashMap::new();
            for edge in &self.edges {
                let key = if edge.from < edge.to {
                    (edge.from.clone(), edge.to.clone())
                } else {
                    (edge.to.clone(), edge.from.clone())
                };
                
                // Check if reverse edge exists
                let has_reverse = self.edges.iter().any(|e| 
                    e.from == edge.to && e.to == edge.from && e.edge_type != edge.edge_type
                );
                
                if has_reverse && !edge_offsets.contains_key(&key) {
                    edge_offsets.insert(key, 15.0 * self.zoom); // Offset for curved edges
                }
            }
            
            // Draw edges with arrows
            for edge in &self.edges {
                if let (Some(from_node), Some(to_node)) = 
                    (self.nodes.get(&edge.from), self.nodes.get(&edge.to)) {
                    
                    let from_pos = self.world_to_screen(from_node.position);
                    let to_pos = self.world_to_screen(to_node.position);
                    
                    // Different colors for publish vs subscribe - theme based
                    let color = match edge.edge_type {
                        EdgeType::Publish => {
                            if edge.active {
                                publish_edge_color
                            } else {
                                publish_edge_color.gamma_multiply(0.6)
                            }
                        }
                        EdgeType::Subscribe => {
                            if edge.active {
                                subscribe_edge_color
                            } else {
                                subscribe_edge_color.gamma_multiply(0.6)
                            }
                        }
                    };
                    
                    // Check if this edge needs to be curved (bidirectional)
                    let key = if edge.from < edge.to {
                        (edge.from.clone(), edge.to.clone())
                    } else {
                        (edge.to.clone(), edge.from.clone())
                    };
                    
                    let curve_offset = edge_offsets.get(&key).copied().unwrap_or(0.0);
                    
                    if curve_offset > 0.0 {
                        // Draw curved edge for bidirectional connections
                        let mid_point = from_pos + (to_pos - from_pos) * 0.5;
                        let direction = (to_pos - from_pos).normalized();
                        let perp = vec2(-direction.y, direction.x);
                        
                        // Offset based on edge type to separate pub/sub curves
                        let offset_dir = if edge.edge_type == EdgeType::Publish { 1.0 } else { -1.0 };
                        let control_point = mid_point + perp * curve_offset * offset_dir;
                        
                        // Draw quadratic bezier curve
                        let mut points = Vec::new();
                        for t in 0..=20 {
                            let t = t as f32 / 20.0;
                            let point = (1.0 - t).powi(2) * from_pos.to_vec2()
                                + 2.0 * (1.0 - t) * t * control_point.to_vec2()
                                + t.powi(2) * to_pos.to_vec2();
                            points.push(point.to_pos2());
                        }
                        
                        for i in 0..points.len() - 1 {
                            painter.line_segment([points[i], points[i + 1]], Stroke::new(2.0, color));
                        }
                        
                        // Calculate arrow direction from last curve segment
                        let arrow_direction = (to_pos - control_point).normalized();
                        
                        // Draw arrow at the end
                        let target_radius = match to_node.node_type {
                            NodeType::Process => 15.0 * self.zoom,
                            NodeType::Topic => 10.0 * self.zoom,
                        };
                        let arrow_tip = to_pos - arrow_direction * target_radius;
                        let arrow_length = match edge.edge_type {
                            EdgeType::Publish => 12.0 * self.zoom.min(2.0),   // Larger for publish
                            EdgeType::Subscribe => 10.0 * self.zoom.min(2.0), // Smaller for subscribe
                        };
                        let arrow_width = match edge.edge_type {
                            EdgeType::Publish => 7.0 * self.zoom.min(2.0),    // Wider for publish
                            EdgeType::Subscribe => 5.0 * self.zoom.min(2.0),  // Narrower for subscribe
                        };
                        let arrow_base = arrow_tip - arrow_direction * arrow_length;
                        
                        let perp = vec2(-arrow_direction.y, arrow_direction.x);
                        let wing1 = arrow_base + perp * arrow_width;
                        let wing2 = arrow_base - perp * arrow_width;
                        
                        // Different arrow styles
                        match edge.edge_type {
                            EdgeType::Publish => {
                                // Filled triangle for publish (data output)
                                let arrow_points = vec![arrow_tip, wing1, wing2];
                                painter.add(Shape::convex_polygon(arrow_points, color, Stroke::NONE));
                            }
                            EdgeType::Subscribe => {
                                // Open arrow (chevron) for subscribe (data input)
                                painter.line_segment([wing1, arrow_tip], Stroke::new(2.5, color));
                                painter.line_segment([wing2, arrow_tip], Stroke::new(2.5, color));
                            }
                        }
                    } else {
                        // Draw straight edge
                        painter.line_segment([from_pos, to_pos], Stroke::new(2.0, color));
                        
                        // Calculate arrow position and direction
                        let direction = (to_pos - from_pos).normalized();
                        let arrow_length = match edge.edge_type {
                            EdgeType::Publish => 12.0 * self.zoom.min(2.0),   // Larger for publish
                            EdgeType::Subscribe => 10.0 * self.zoom.min(2.0), // Smaller for subscribe
                        };
                        let arrow_width = match edge.edge_type {
                            EdgeType::Publish => 7.0 * self.zoom.min(2.0),    // Wider for publish
                            EdgeType::Subscribe => 5.0 * self.zoom.min(2.0),  // Narrower for subscribe
                        };
                        
                        // Position arrow at edge endpoint (adjusted for node radius)
                        let target_radius = match to_node.node_type {
                            NodeType::Process => 15.0 * self.zoom,
                            NodeType::Topic => 10.0 * self.zoom,
                        };
                        let arrow_tip = to_pos - direction * target_radius;
                        let arrow_base = arrow_tip - direction * arrow_length;
                        
                        // Calculate perpendicular vector for arrow wings
                        let perp = vec2(-direction.y, direction.x);
                        let wing1 = arrow_base + perp * arrow_width;
                        let wing2 = arrow_base - perp * arrow_width;
                        
                        // Different arrow styles for publish vs subscribe
                        match edge.edge_type {
                            EdgeType::Publish => {
                                // Filled triangle for publish (data output)
                                let arrow_points = vec![arrow_tip, wing1, wing2];
                                painter.add(Shape::convex_polygon(arrow_points, color, Stroke::NONE));
                            }
                            EdgeType::Subscribe => {
                                // Open arrow (chevron) for subscribe (data input)
                                painter.line_segment([wing1, arrow_tip], Stroke::new(2.5, color));
                                painter.line_segment([wing2, arrow_tip], Stroke::new(2.5, color));
                            }
                        }
                    }
                }
            }
            
            // Draw nodes
            for (id, node) in &self.nodes {
                let pos = self.world_to_screen(node.position);
                
                // Skip if outside view
                if !rect.contains(pos) {
                    continue;
                }
                
                let radius = match node.node_type {
                    NodeType::Process => 15.0,
                    NodeType::Topic => 10.0,
                };
                
                let color = match node.node_type {
                    NodeType::Process => {
                        if node.active {
                            process_color
                        } else {
                            process_color.gamma_multiply(0.6)
                        }
                    }
                    NodeType::Topic => {
                        if node.active {
                            topic_color
                        } else {
                            topic_color.gamma_multiply(0.6)
                        }
                    }
                };
                
                // Highlight if searched
                let stroke = if self.highlighted_nodes.contains(id) {
                    Stroke::new(3.0, Color32::YELLOW)
                } else if self.selected_node.as_ref() == Some(id) {
                    Stroke::new(2.0, Color32::WHITE)
                } else {
                    Stroke::new(1.0, Color32::from_gray(100))
                };
                
                painter.circle(pos, radius * self.zoom, color, stroke);
                
                // Draw label
                if self.show_labels {
                    let text_pos = pos + vec2(0.0, radius * self.zoom + 5.0);
                    painter.text(
                        text_pos,
                        Align2::CENTER_TOP,
                        &node.label,
                        FontId::proportional(10.0 * self.zoom.min(1.5)),
                        text_color,
                    );
                }
                
                // Handle interactions
                let node_rect = Rect::from_center_size(pos, vec2(radius * 2.0, radius * 2.0) * self.zoom);
                if node_rect.contains(response.hover_pos().unwrap_or_default()) {
                    // Show tooltip on hover
                    egui::show_tooltip_at_pointer(ui.ctx(), Id::new(format!("tooltip_{}", id)), |ui| {
                        ui.label(&node.label);
                    });
                    
                    // Handle click
                    if response.clicked() {
                        self.selected_node = Some(id.clone());
                    }
                    
                    // Handle drag
                    if response.drag_started_by(PointerButton::Primary) {
                        self.dragging_node = Some(id.clone());
                    }
                }
            }
            
            // Update dragged node position
            if let Some(dragged_id) = &self.dragging_node {
                if response.dragged_by(PointerButton::Primary) {
                    let world_pos = self.screen_to_world(response.hover_pos().unwrap_or_default());
                    if let Some(node) = self.nodes.get_mut(dragged_id) {
                        node.position = world_pos;
                    }
                } else {
                    self.dragging_node = None;
                }
            }
            
            // Request repaint for animations
            ctx.request_repaint();
        });
    }
}

/// Discover graph data including nodes (processes) and topics (shared memory) with their relationships
pub fn discover_graph_data() -> (Vec<GraphNode>, Vec<GraphEdge>) {
    let mut graph_nodes = Vec::new();
    let mut graph_edges = Vec::new();
    
    // Discover processes
    if let Ok(nodes) = super::commands::monitor::discover_nodes() {
        for node in nodes {
            graph_nodes.push(GraphNode {
                id: format!("process_{}_{}", node.process_id, node.name),
                label: node.name.clone(),
                node_type: NodeType::Process,
                position: Pos2::new(0.0, 0.0), // Will be set by layout
                velocity: Vec2::ZERO,
                pid: Some(node.process_id),
                active: node.status == "Running",
            });
        }
    }
    
    // Discover topics
    if let Ok(topics) = super::commands::monitor::discover_shared_memory() {
        for topic in topics {
            let topic_id = format!("topic_{}", topic.topic_name);

            graph_nodes.push(GraphNode {
                id: topic_id.clone(),
                label: topic.topic_name.clone(),
                node_type: NodeType::Topic,
                position: Pos2::new(0.0, 0.0), // Will be set by layout
                velocity: Vec2::ZERO,
                pid: None,
                active: topic.active,
            });

            // Create edges based on actual publishers and subscribers (RQT-style)

            // Publisher edges: Process -> Topic (processes that WRITE to this topic)
            for publisher_name in &topic.publishers {
                // Find the process node by name
                if let Some(process_node) = graph_nodes.iter().find(|n|
                    n.node_type == NodeType::Process && n.label == *publisher_name
                ) {
                    graph_edges.push(GraphEdge {
                        from: process_node.id.clone(),
                        to: topic_id.clone(),
                        edge_type: EdgeType::Publish,
                        active: topic.active,
                    });
                }
            }

            // Subscriber edges: Topic -> Process (processes that READ from this topic)
            for subscriber_name in &topic.subscribers {
                // Find the process node by name
                if let Some(process_node) = graph_nodes.iter().find(|n|
                    n.node_type == NodeType::Process && n.label == *subscriber_name
                ) {
                    graph_edges.push(GraphEdge {
                        from: topic_id.clone(),
                        to: process_node.id.clone(),
                        edge_type: EdgeType::Subscribe,
                        active: topic.active,
                    });
                }
            }
        }
    }

    // Only show real nodes and topics - no demo data
    
    (graph_nodes, graph_edges)
}

// Entry point for the graph visualization
pub fn show_graph_visualization() -> anyhow::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("HORUS Node-Topic Graph")
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "HORUS Graph",
        native_options,
        Box::new(|_cc| {
            Box::new(GraphVisualization::new(Box::new(discover_graph_data)))
        }),
    ).map_err(|e| anyhow::anyhow!("Failed to run graph visualization: {}", e))
}


// Implement rand::random for demo purposes (replace with proper random in production)
mod rand {
    pub fn random<T>() -> T 
    where 
        T: RandomValue
    {
        T::random()
    }
    
    pub trait RandomValue {
        fn random() -> Self;
    }
    
    impl RandomValue for f32 {
        fn random() -> Self {
            // Simple pseudo-random using system time
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos();
            (nanos % 1000) as f32 / 1000.0
        }
    }
}