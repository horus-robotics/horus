//! Compile-time tests for HORUS macros
//!
//! These tests ensure the macros generate valid code and handle errors properly.

#[cfg(test)]
mod tests {
    use horus_macros::node;

    // Test that messages work with standard derive macros
    #[test]
    fn test_message_with_derives() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct TestMessage {
            value: f32,
            count: u32,
        }

        let msg = TestMessage {
            value: 1.0,
            count: 42,
        };

        // Test standard functionality
        let _cloned = msg.clone();
        let _debug = format!("{:?}", msg);
    }

    // Test that node macro generates valid code
    // Note: This is a compile-only test since we can't actually create Hub instances in tests
    #[test]
    fn test_node_macro_compiles() {
        use horus_core::communication::horus::Hub;
        use horus_core::core::node::{Node, NodeInfo};
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct TestData {
            value: i32,
        }

        #[node]
        struct TestNode {
            #[topic(publish)]
            output: Hub<TestData>,

            #[topic(subscribe, "input_topic")]
            input: Hub<TestData>,

            #[config]
            rate: f32,
        }

        // This test just ensures the macro generates compilable code
        // Actual functionality would be tested in integration tests
    }

    // Test that the node macro respects visibility
    #[test]
    fn test_public_node() {
        use horus_core::communication::horus::Hub;
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct PublicData {
            pub value: u32,
        }

        #[node]
        pub struct PublicNode {
            #[topic(publish)]
            pub output: Hub<PublicData>,
        }

        // This test just ensures the macro generates compilable code
        // with public visibility
    }
}
