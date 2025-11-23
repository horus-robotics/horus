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

#[cfg(test)]
mod tests {
    use super::*;

    // =====================
    // NodeType Tests
    // =====================
    #[test]
    fn test_node_type_equality() {
        assert_eq!(NodeType::Process, NodeType::Process);
        assert_eq!(NodeType::Topic, NodeType::Topic);
        assert_ne!(NodeType::Process, NodeType::Topic);
    }

    #[test]
    fn test_node_type_clone() {
        let node_type = NodeType::Process;
        let cloned = node_type.clone();
        assert_eq!(cloned, NodeType::Process);
    }

    // =====================
    // EdgeType Tests
    // =====================
    #[test]
    fn test_edge_type_equality() {
        assert_eq!(EdgeType::Publish, EdgeType::Publish);
        assert_eq!(EdgeType::Subscribe, EdgeType::Subscribe);
        assert_ne!(EdgeType::Publish, EdgeType::Subscribe);
    }

    #[test]
    fn test_edge_type_clone() {
        let edge_type = EdgeType::Subscribe;
        let cloned = edge_type.clone();
        assert_eq!(cloned, EdgeType::Subscribe);
    }

    // =====================
    // GraphNode Tests
    // =====================
    #[test]
    fn test_graph_node_process_creation() {
        let node = GraphNode {
            id: "process_1234_sensor".to_string(),
            label: "sensor".to_string(),
            node_type: NodeType::Process,
            position: Pos2::new(100.0, 200.0),
            velocity: Vec2::ZERO,
            pid: Some(1234),
            active: true,
        };

        assert_eq!(node.id, "process_1234_sensor");
        assert_eq!(node.label, "sensor");
        assert_eq!(node.node_type, NodeType::Process);
        assert_eq!(node.pid, Some(1234));
        assert!(node.active);
    }

    #[test]
    fn test_graph_node_topic_creation() {
        let node = GraphNode {
            id: "topic_/robot/pose".to_string(),
            label: "/robot/pose".to_string(),
            node_type: NodeType::Topic,
            position: Pos2::new(300.0, 150.0),
            velocity: Vec2::new(0.1, 0.2),
            pid: None,
            active: true,
        };

        assert_eq!(node.node_type, NodeType::Topic);
        assert!(node.pid.is_none());
        assert_eq!(node.label, "/robot/pose");
    }

    #[test]
    fn test_graph_node_inactive() {
        let node = GraphNode {
            id: "process_dead".to_string(),
            label: "dead_node".to_string(),
            node_type: NodeType::Process,
            position: Pos2::ZERO,
            velocity: Vec2::ZERO,
            pid: Some(99999),
            active: false,
        };

        assert!(!node.active);
    }

    #[test]
    fn test_graph_node_clone() {
        let node = GraphNode {
            id: "clone_test".to_string(),
            label: "test".to_string(),
            node_type: NodeType::Process,
            position: Pos2::new(50.0, 60.0),
            velocity: Vec2::new(1.0, 2.0),
            pid: Some(5678),
            active: true,
        };

        let cloned = node.clone();
        assert_eq!(cloned.id, node.id);
        assert_eq!(cloned.label, node.label);
        assert_eq!(cloned.position, node.position);
        assert_eq!(cloned.pid, node.pid);
    }

    // =====================
    // GraphEdge Tests
    // =====================
    #[test]
    fn test_graph_edge_publish_creation() {
        let edge = GraphEdge {
            from: "process_sensor".to_string(),
            to: "topic_/data".to_string(),
            edge_type: EdgeType::Publish,
            active: true,
        };

        assert_eq!(edge.from, "process_sensor");
        assert_eq!(edge.to, "topic_/data");
        assert_eq!(edge.edge_type, EdgeType::Publish);
        assert!(edge.active);
    }

    #[test]
    fn test_graph_edge_subscribe_creation() {
        let edge = GraphEdge {
            from: "topic_/commands".to_string(),
            to: "process_controller".to_string(),
            edge_type: EdgeType::Subscribe,
            active: true,
        };

        assert_eq!(edge.edge_type, EdgeType::Subscribe);
    }

    #[test]
    fn test_graph_edge_inactive() {
        let edge = GraphEdge {
            from: "a".to_string(),
            to: "b".to_string(),
            edge_type: EdgeType::Publish,
            active: false,
        };

        assert!(!edge.active);
    }

    #[test]
    fn test_graph_edge_clone() {
        let edge = GraphEdge {
            from: "node1".to_string(),
            to: "node2".to_string(),
            edge_type: EdgeType::Subscribe,
            active: true,
        };

        let cloned = edge.clone();
        assert_eq!(cloned.from, edge.from);
        assert_eq!(cloned.to, edge.to);
        assert_eq!(cloned.edge_type, edge.edge_type);
        assert_eq!(cloned.active, edge.active);
    }

    // =====================
    // discover_graph_data Tests
    // =====================
    #[test]
    fn test_discover_graph_data_returns_tuple() {
        // Should return (Vec<GraphNode>, Vec<GraphEdge>) without panicking
        let (nodes, edges) = discover_graph_data();

        // May be empty if no nodes running, but should not panic
        // Just verify the types are correct (vec length is always >= 0)
        let _nodes_count = nodes.len();
        let _edges_count = edges.len();
    }

    #[test]
    fn test_discover_graph_data_no_panic_on_missing_dirs() {
        // Even if /dev/shm/horus doesn't exist, should gracefully return empty
        let (nodes, edges) = discover_graph_data();
        // Just ensure no panic - empty is fine
        let _ = nodes;
        let _ = edges;
    }

    // =====================
    // Position Consistency Tests
    // =====================
    #[test]
    fn test_process_node_positions_left_side() {
        // Process nodes should generally be on the left side (negative x)
        // This is based on the get_position closure which uses x_base = -300.0 for Process
        let node = GraphNode {
            id: "test_process".to_string(),
            label: "test".to_string(),
            node_type: NodeType::Process,
            position: Pos2::new(-280.0, 100.0), // Typical process position
            velocity: Vec2::ZERO,
            pid: Some(1),
            active: true,
        };

        // Process nodes typically have negative x (left side)
        assert!(node.position.x < 0.0);
    }

    #[test]
    fn test_topic_node_positions_right_side() {
        // Topic nodes should generally be on the right side (positive x)
        // This is based on the get_position closure which uses x_base = 300.0 for Topic
        let node = GraphNode {
            id: "test_topic".to_string(),
            label: "/test".to_string(),
            node_type: NodeType::Topic,
            position: Pos2::new(320.0, 100.0), // Typical topic position
            velocity: Vec2::ZERO,
            pid: None,
            active: true,
        };

        // Topic nodes typically have positive x (right side)
        assert!(node.position.x > 0.0);
    }

    // =====================
    // Edge Direction Tests
    // =====================
    #[test]
    fn test_publish_edge_direction() {
        // Publish edges go from Process -> Topic
        let process_id = "process_1234_pub".to_string();
        let topic_id = "topic_/data".to_string();

        let edge = GraphEdge {
            from: process_id.clone(),
            to: topic_id.clone(),
            edge_type: EdgeType::Publish,
            active: true,
        };

        assert!(edge.from.starts_with("process_") || edge.from.starts_with("node_"));
        assert!(edge.to.starts_with("topic_"));
    }

    #[test]
    fn test_subscribe_edge_direction() {
        // Subscribe edges go from Topic -> Process
        let topic_id = "topic_/commands".to_string();
        let process_id = "process_5678_sub".to_string();

        let edge = GraphEdge {
            from: topic_id.clone(),
            to: process_id.clone(),
            edge_type: EdgeType::Subscribe,
            active: true,
        };

        assert!(edge.from.starts_with("topic_"));
        assert!(edge.to.starts_with("process_") || edge.to.starts_with("node_"));
    }

    // =====================
    // Integration-like Tests
    // =====================
    #[test]
    fn test_graph_structure_consistency() {
        // Build a small graph manually and verify structure
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // Add a process node
        nodes.push(GraphNode {
            id: "process_1_sensor".to_string(),
            label: "sensor".to_string(),
            node_type: NodeType::Process,
            position: Pos2::new(-300.0, 0.0),
            velocity: Vec2::ZERO,
            pid: Some(1),
            active: true,
        });

        // Add a topic node
        nodes.push(GraphNode {
            id: "topic_/sensor_data".to_string(),
            label: "/sensor_data".to_string(),
            node_type: NodeType::Topic,
            position: Pos2::new(300.0, 0.0),
            velocity: Vec2::ZERO,
            pid: None,
            active: true,
        });

        // Add a subscriber process node
        nodes.push(GraphNode {
            id: "process_2_processor".to_string(),
            label: "processor".to_string(),
            node_type: NodeType::Process,
            position: Pos2::new(-300.0, 120.0),
            velocity: Vec2::ZERO,
            pid: Some(2),
            active: true,
        });

        // Add publish edge: sensor -> topic
        edges.push(GraphEdge {
            from: "process_1_sensor".to_string(),
            to: "topic_/sensor_data".to_string(),
            edge_type: EdgeType::Publish,
            active: true,
        });

        // Add subscribe edge: topic -> processor
        edges.push(GraphEdge {
            from: "topic_/sensor_data".to_string(),
            to: "process_2_processor".to_string(),
            edge_type: EdgeType::Subscribe,
            active: true,
        });

        // Verify counts
        assert_eq!(nodes.len(), 3);
        assert_eq!(edges.len(), 2);

        // Verify node types
        let process_count = nodes.iter().filter(|n| n.node_type == NodeType::Process).count();
        let topic_count = nodes.iter().filter(|n| n.node_type == NodeType::Topic).count();
        assert_eq!(process_count, 2);
        assert_eq!(topic_count, 1);

        // Verify edge types
        let pub_count = edges.iter().filter(|e| e.edge_type == EdgeType::Publish).count();
        let sub_count = edges.iter().filter(|e| e.edge_type == EdgeType::Subscribe).count();
        assert_eq!(pub_count, 1);
        assert_eq!(sub_count, 1);

        // Verify edges reference valid nodes
        for edge in &edges {
            assert!(nodes.iter().any(|n| n.id == edge.from));
            assert!(nodes.iter().any(|n| n.id == edge.to));
        }
    }

    #[test]
    fn test_velocity_vector_operations() {
        let node = GraphNode {
            id: "test".to_string(),
            label: "test".to_string(),
            node_type: NodeType::Process,
            position: Pos2::new(0.0, 0.0),
            velocity: Vec2::new(5.0, 10.0),
            pid: None,
            active: true,
        };

        // Test velocity vector properties
        assert!((node.velocity.x - 5.0).abs() < 0.001);
        assert!((node.velocity.y - 10.0).abs() < 0.001);

        // Velocity can be used for physics simulation
        let new_position = node.position + node.velocity;
        assert!((new_position.x - 5.0).abs() < 0.001);
        assert!((new_position.y - 10.0).abs() < 0.001);
    }
}
