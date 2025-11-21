// Cloud LLM Chat Example
//
// Demonstrates cloud LLM integration with OpenAI and Anthropic APIs.
// This example shows how to:
// - Set up a cloud LLM node
// - Send chat requests
// - Receive and display responses
//
// Usage:
//   export OPENAI_API_KEY=your_key_here
//   # or
//   export ANTHROPIC_API_KEY=your_key_here
//
//   cargo run --example llm_chat --features ml-inference

use horus_core::{Hub, Node, NodeInfo, Scheduler};
use horus_library::messages::ml::{ChatMessage, LLMRequest, LLMResponse};
use horus_library::nodes::{CloudLLMNode, LLMConfig, LLMProvider};

/// LLM request generator node
struct LLMRequestNode {
    request_pub: Hub<LLMRequest>,
    sent_count: usize,
    questions: Vec<String>,
}

impl LLMRequestNode {
    fn new() -> horus_core::HorusResult<Self> {
        Ok(Self {
            request_pub: Hub::new("llm/requests")?,
            sent_count: 0,
            questions: vec![
                "What is the capital of France?".to_string(),
                "Explain quantum computing in one sentence.".to_string(),
                "What are the three laws of robotics?".to_string(),
            ],
        })
    }
}

impl Node for LLMRequestNode {
    fn name(&self) -> &'static str {
        "LLMRequestNode"
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        if self.sent_count < self.questions.len() {
            let question = &self.questions[self.sent_count];

            let request = LLMRequest {
                messages: vec![
                    ChatMessage {
                        role: "system".to_string(),
                        content: "You are a helpful assistant. Keep responses concise.".to_string(),
                    },
                    ChatMessage {
                        role: "user".to_string(),
                        content: question.clone(),
                    },
                ],
            };

            println!("\n[Question {}] {}", self.sent_count + 1, question);
            let _ = self.request_pub.send(request, &mut ctx);
            self.sent_count += 1;
        }
    }
}

/// LLM response display node
struct LLMResponseNode {
    response_sub: Hub<LLMResponse>,
    received_count: usize,
}

impl LLMResponseNode {
    fn new() -> horus_core::HorusResult<Self> {
        Ok(Self {
            response_sub: Hub::new("llm/responses")?,
            received_count: 0,
        })
    }
}

impl Node for LLMResponseNode {
    fn name(&self) -> &'static str {
        "LLMResponseNode"
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        while let Some(response) = self.response_sub.recv(&mut ctx) {
            self.received_count += 1;

            println!("\n[Answer {}] {}", self.received_count, response.response);
            println!("  Model: {}", response.model);
            println!("  Tokens: {}", response.tokens_used);
            println!("  Latency: {}ms", response.latency_ms);
            println!("  Finish: {}", response.finish_reason);
        }
    }
}

fn main() -> horus_core::HorusResult<()> {
    println!("HORUS Cloud LLM Chat Example");
    println!("=============================\n");

    // Determine which provider to use
    let (config, provider_name) = if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        (
            LLMConfig::anthropic_claude_sonnet(),
            "Anthropic Claude 3.5 Sonnet",
        )
    } else if std::env::var("OPENAI_API_KEY").is_ok() {
        (LLMConfig::openai_gpt35_turbo(), "OpenAI GPT-3.5 Turbo")
    } else {
        println!("ERROR: No API key found!");
        println!("\nPlease set one of the following environment variables:");
        println!("  export OPENAI_API_KEY=your_openai_key");
        println!("  export ANTHROPIC_API_KEY=your_anthropic_key");
        println!("\nGet API keys from:");
        println!("  OpenAI: https://platform.openai.com/api-keys");
        println!("  Anthropic: https://console.anthropic.com/settings/keys\n");
        return Ok(());
    };

    println!("Using: {}", provider_name);
    println!("Model: {}\n", config.model);
    println!("Initializing nodes...");

    // Create nodes
    let request_node = LLMRequestNode::new()?;

    let llm_node = CloudLLMNode::new("llm/requests", "llm/responses", config)?;

    let response_node = LLMResponseNode::new()?;

    println!("Starting conversation...\n");
    println!("=".repeat(60));

    // Create scheduler and add nodes
    let mut scheduler = Scheduler::new();
    scheduler.add(Box::new(request_node), 0, Some(false));
    scheduler.add(Box::new(llm_node), 1, Some(true));
    scheduler.add(Box::new(response_node), 2, Some(false));

    // Run for 60 seconds (enough time for API calls)
    scheduler.run_for(std::time::Duration::from_secs(60))?;

    println!("\n".repeat(2));
    println!("=".repeat(60));
    println!("\nExample completed successfully!");
    Ok(())
}
