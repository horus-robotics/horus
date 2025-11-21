//! Tests for the section-based node! macro

#[cfg(test)]
mod tests {
    use horus_macros::node;

    // Mock types for testing (since we can't import actual HORUS in macro tests)
    mod mock {
        pub mod horus_core {
            pub mod communication {
                pub struct Hub<T> {
                    _phantom: std::marker::PhantomData<T>,
                }

                impl<T> Hub<T> {
                    pub fn new(_topic: &str) -> Result<Self, Box<dyn std::error::Error>> {
                        Ok(Self {
                            _phantom: std::marker::PhantomData,
                        })
                    }

                    pub fn recv(
                        &self,
                        _ctx: Option<&mut crate::tests::mock::horus_core::core::NodeInfo>,
                    ) -> Option<T> {
                        None
                    }

                    pub fn send(
                        &self,
                        _data: T,
                        _ctx: Option<&mut crate::tests::mock::horus_core::core::NodeInfo>,
                    ) -> Result<(), Box<dyn std::error::Error>> {
                        Ok(())
                    }
                }
            }

            pub mod error {
                pub type HorusResult<T> = Result<T, HorusError>;

                #[derive(Debug)]
                pub enum HorusError {
                    Communication(String),
                }

                impl std::fmt::Display for HorusError {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        match self {
                            HorusError::Communication(msg) => {
                                write!(f, "Communication error: {}", msg)
                            }
                        }
                    }
                }

                impl std::error::Error for HorusError {}
            }

            pub mod core {
                pub struct NodeInfo;

                impl NodeInfo {
                    pub fn log_info(&mut self, _msg: &str) {}
                    pub fn log_debug(&mut self, _msg: &str) {}
                }

                pub mod node {
                    #[derive(Debug, Clone)]
                    pub struct TopicMetadata {
                        pub topic_name: String,
                        pub type_name: String,
                    }

                    pub trait Node {
                        fn name(&self) -> &'static str;
                        fn tick(&mut self, ctx: Option<&mut super::NodeInfo>);
                        fn init(
                            &mut self,
                            _ctx: &mut super::NodeInfo,
                        ) -> crate::tests::mock::horus_core::error::HorusResult<()>
                        {
                            Ok(())
                        }
                        fn shutdown(
                            &mut self,
                            _ctx: &mut super::NodeInfo,
                        ) -> crate::tests::mock::horus_core::error::HorusResult<()>
                        {
                            Ok(())
                        }
                        fn get_publishers(&self) -> Vec<TopicMetadata> {
                            Vec::new()
                        }
                        fn get_subscribers(&self) -> Vec<TopicMetadata> {
                            Vec::new()
                        }
                    }
                }
            }
        }
    }

    // Test basic node with all sections
    #[test]
    fn test_complete_node() {
        use crate::tests::mock::horus_core;

        #[derive(Debug, Clone)]
        struct TestData {
            value: i32,
        }

        node! {
            TestNode {
                pub {
                    output: TestData -> "test/output",
                }

                sub {
                    input: TestData -> "test/input",
                }

                data {
                    counter: u32 = 0,
                    buffer: Vec<u8> = Vec::new(),
                }

                tick(_ctx) {
                    // Process one message per tick for bounded execution
                    if let Some(data) = self.input.recv(None) {
                        self.counter += 1;
                        self.output.send(data, None).ok();
                    }
                }

                init(ctx) {
                    ctx.log_info("TestNode initialized");
                    Ok(())
                }

                impl {
                    pub fn reset(&mut self) {
                        self.counter = 0;
                        self.buffer.clear();
                    }
                }
            }
        }

        // Test that the generated code compiles
        let mut node = TestNode::new();
        assert_eq!(node.counter, 0);
        node.reset();

        // Test Node trait implementation
        use horus_core::core::node::Node;
        assert_eq!(node.name(), "test_node");
    }

    // Test minimal node (only required sections)
    #[test]
    fn test_minimal_node() {
        use crate::tests::mock::horus_core;

        #[derive(Debug, Clone)]
        struct Message {
            data: String,
        }

        node! {
            EchoNode {
                pub {
                    output: Message -> "output",
                }

                sub {
                    input: Message -> "input",
                }

                tick {
                    if let Some(msg) = self.input.recv(None) {
                        self.output.send(msg, None).ok();
                    }
                }
            }
        }

        let _node = EchoNode::new();
    }

    // Test node with empty pub/sub sections
    #[test]
    fn test_producer_only_node() {
        use crate::tests::mock::horus_core;

        #[derive(Debug, Clone)]
        struct SensorData {
            reading: f32,
        }

        node! {
            SensorNode {
                pub {
                    data: SensorData -> "sensor/data",
                }

                sub {}

                data {
                    last_reading: f32 = 0.0,
                }

                tick {
                    self.last_reading += 1.0;
                    let reading = SensorData { reading: self.last_reading };
                    self.data.send(reading, None).ok();
                }
            }
        }

        let node = SensorNode::new();
        assert_eq!(node.last_reading, 0.0);
    }

    // Test that snake_case conversion works
    #[test]
    fn test_snake_case_naming() {
        use crate::tests::mock::horus_core;
        use horus_core::core::node::Node;

        node! {
            MyComplexNodeName {
                pub {}
                sub {}
                tick {}
            }
        }

        let node = MyComplexNodeName::new();
        assert_eq!(node.name(), "my_complex_node_name");
    }
}
