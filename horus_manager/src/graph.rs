use eframe::egui::{Pos2, Vec2};

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
    Publish,   // Process publishes to topic
    Subscribe, // Process subscribes from topic
}

// Graph edge representation
#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub edge_type: EdgeType,
    pub active: bool,
}

/// Discover runtime pub/sub relationships from Hub activity metadata
/// Returns (publishers, subscribers) for a given topic
fn discover_runtime_pubsub(topic_name: &str) -> Result<(Vec<String>, Vec<String>), std::io::Error> {
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    let metadata_dir = Path::new("/dev/shm/horus/pubsub_metadata");
    if !metadata_dir.exists() {
        return Ok((Vec::new(), Vec::new()));
    }

    let mut publishers = Vec::new();
    let mut subscribers = Vec::new();

    // Staleness threshold: 30 seconds
    let staleness_threshold_secs = 30;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Normalize topic name for file matching
    let safe_topic: String = topic_name
        .chars()
        .map(|c| if c == '/' || c == ' ' { '_' } else { c })
        .collect();

    // Scan metadata directory for files matching this topic
    for entry in fs::read_dir(metadata_dir)? {
        let entry = entry?;
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();

        // File format: {node_name}_{topic_name}_{direction}
        // e.g., JoystickInputNode_joystick_input_pub
        // Both node_name and topic_name can contain underscores!

        // Extract direction (last part after final underscore)
        let direction = if filename_str.ends_with("_pub") {
            "pub"
        } else if filename_str.ends_with("_sub") {
            "sub"
        } else {
            continue;
        };

        // Remove the direction suffix to get: {node_name}_{topic_name}
        let without_direction = if direction == "pub" {
            filename_str.strip_suffix("_pub").unwrap()
        } else {
            filename_str.strip_suffix("_sub").unwrap()
        };

        // Check if this file matches our topic by checking if it ends with the topic name
        // Format should be: {node_name}_{safe_topic}
        if without_direction.ends_with(&format!("_{}", safe_topic)) {
            // Check staleness: read timestamp from file
            let is_active = if let Ok(contents) = fs::read_to_string(entry.path()) {
                if let Ok(timestamp) = contents.trim().parse::<u64>() {
                    let age_secs = now.saturating_sub(timestamp);
                    age_secs < staleness_threshold_secs
                } else {
                    false // Invalid timestamp format
                }
            } else {
                false // Couldn't read file
            };

            // Only include active nodes (updated within threshold)
            if is_active {
                // Extract node name by removing the topic suffix
                let node_name = without_direction
                    .strip_suffix(&format!("_{}", safe_topic))
                    .unwrap()
                    .to_string();

                match direction {
                    "pub" => {
                        if !publishers.contains(&node_name) {
                            publishers.push(node_name);
                        }
                    }
                    "sub" => {
                        if !subscribers.contains(&node_name) {
                            subscribers.push(node_name);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok((publishers, subscribers))
}

/// Discover graph data including nodes (processes) and topics (shared memory) with their relationships
pub fn discover_graph_data() -> (Vec<GraphNode>, Vec<GraphEdge>) {
    let mut graph_nodes = Vec::new();
    let mut graph_edges = Vec::new();
    let mut process_index = 0;
    let mut topic_index = 0;

    // Helper to generate initial position for nodes
    let get_position = |node_type: &NodeType, index: usize, node_id: &str| -> Pos2 {
        // Hash the node ID for deterministic variation
        let mut hash: u64 = 0;
        for byte in node_id.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }

        match node_type {
            NodeType::Process => {
                let x_base = -300.0;
                let x_variation = (hash % 80) as f32 - 40.0;
                let vertical_spacing = 120.0;
                let y_variation = ((hash / 100) % 40) as f32 - 20.0;
                Pos2::new(
                    x_base + x_variation,
                    index as f32 * vertical_spacing + y_variation,
                )
            }
            NodeType::Topic => {
                let x_base = 300.0;
                let x_variation = (hash % 100) as f32 - 50.0;
                let vertical_spacing = 140.0;
                let y_variation = ((hash / 100) % 50) as f32 - 25.0;
                Pos2::new(
                    x_base + x_variation,
                    index as f32 * vertical_spacing + y_variation,
                )
            }
        }
    };

    // Discover processes
    if let Ok(nodes) = super::commands::monitor::discover_nodes() {
        for node in nodes {
            let node_id = format!("process_{}_{}", node.process_id, node.name);
            graph_nodes.push(GraphNode {
                id: node_id.clone(),
                label: node.name.clone(),
                node_type: NodeType::Process,
                position: get_position(&NodeType::Process, process_index, &node_id),
                velocity: Vec2::ZERO,
                pid: Some(node.process_id),
                active: node.status == "Running",
            });
            process_index += 1;
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
                position: get_position(&NodeType::Topic, topic_index, &topic_id),
                velocity: Vec2::ZERO,
                pid: None,
                active: topic.active,
            });
            topic_index += 1;

            // Create edges based on runtime pub/sub activity
            // Read from /dev/shm/horus/pubsub_metadata/ files created by Hub
            if let Ok((publishers, subscribers)) = discover_runtime_pubsub(&topic.topic_name) {
                // Publisher edges: Process -> Topic (processes that WRITE to this topic)
                for publisher_name in publishers {
                    // Find or create the process node by name
                    let process_node_id = if let Some(process_node) = graph_nodes
                        .iter()
                        .find(|n| n.node_type == NodeType::Process && n.label == publisher_name)
                    {
                        process_node.id.clone()
                    } else {
                        // Create virtual node for Python/interpreted language nodes
                        let node_id = format!("node_{}", publisher_name);
                        graph_nodes.push(GraphNode {
                            id: node_id.clone(),
                            label: publisher_name.clone(),
                            node_type: NodeType::Process,
                            position: get_position(&NodeType::Process, process_index, &node_id),
                            velocity: Vec2::ZERO,
                            pid: None, // No PID for virtual nodes
                            active: true,
                        });
                        process_index += 1;
                        node_id
                    };

                    graph_edges.push(GraphEdge {
                        from: process_node_id,
                        to: topic_id.clone(),
                        edge_type: EdgeType::Publish,
                        active: topic.active,
                    });
                }

                // Subscriber edges: Topic -> Process (processes that READ from this topic)
                for subscriber_name in subscribers {
                    // Find or create the process node by name
                    let process_node_id = if let Some(process_node) = graph_nodes
                        .iter()
                        .find(|n| n.node_type == NodeType::Process && n.label == subscriber_name)
                    {
                        process_node.id.clone()
                    } else {
                        // Create virtual node for Python/interpreted language nodes
                        let node_id = format!("node_{}", subscriber_name);
                        graph_nodes.push(GraphNode {
                            id: node_id.clone(),
                            label: subscriber_name.clone(),
                            node_type: NodeType::Process,
                            position: get_position(&NodeType::Process, process_index, &node_id),
                            velocity: Vec2::ZERO,
                            pid: None, // No PID for virtual nodes
                            active: true,
                        });
                        process_index += 1;
                        node_id
                    };

                    graph_edges.push(GraphEdge {
                        from: topic_id.clone(),
                        to: process_node_id,
                        edge_type: EdgeType::Subscribe,
                        active: topic.active,
                    });
                }
            }
        }
    }

    // Update process node activity based on active publish edges
    // A process is "active" if it's running AND has at least one active publish edge
    for node in &mut graph_nodes {
        if node.node_type == NodeType::Process {
            // Check if this node has any active outgoing (publish) edges
            let has_active_publish = graph_edges.iter().any(|edge| {
                edge.from == node.id && edge.edge_type == EdgeType::Publish && edge.active
            });

            // Keep node.active if it was already active (process running)
            // Only show as actively publishing if there's an active edge
            if node.active {
                node.active = has_active_publish;
            }
        }
    }

    // Only show real nodes and topics - no demo data

    (graph_nodes, graph_edges)
}
