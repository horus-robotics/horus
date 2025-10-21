//! Compile-time tests for HORUS macros
//!
//! These tests ensure the macros generate valid code and handle errors properly.

#[cfg(test)]
mod tests {
    

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
    // Note: This test is disabled because the node! macro uses a custom DSL
    // See node_macro_test.rs for working tests
    #[test]
    #[ignore]
    fn test_node_macro_compiles() {
        // This test is kept for documentation purposes but is ignored
        // The node! macro uses a DSL like:
        // node! {
        //     NodeName {
        //         pub { field: Type -> "topic" }
        //         sub { field: Type <- "topic" }
        //         tick { /* code */ }
        //     }
        // }
    }

    // Test that the node macro respects visibility
    #[test]
    #[ignore]
    fn test_public_node() {
        // This test is disabled - see node_macro_test.rs for working tests
        // The node! macro uses a custom DSL, not attribute syntax
    }
}
