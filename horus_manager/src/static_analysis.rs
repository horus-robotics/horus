//! Static analysis for HORUS code to detect common mistakes at compile time
//!
//! This module provides compile-time checks for:
//! - Multiple producers/consumers on the same Link (SPSC violation)
//! - Misuse of Link vs Hub
//! - Other potential IPC issues

use anyhow::{Context, Result};
use colored::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use syn::{visit::Visit, Expr, ExprCall, ExprMethodCall, ExprPath, File};

/// Tracks Link usage per topic to detect SPSC violations
#[derive(Debug, Default)]
struct LinkUsageTracker {
    /// topic_name -> (producer_count, consumer_count)
    topics: HashMap<String, (usize, usize)>,
}

impl LinkUsageTracker {
    fn add_producer(&mut self, topic: &str) {
        let entry = self.topics.entry(topic.to_string()).or_insert((0, 0));
        entry.0 += 1;
    }

    fn add_consumer(&mut self, topic: &str) {
        let entry = self.topics.entry(topic.to_string()).or_insert((0, 0));
        entry.1 += 1;
    }

    fn check_violations(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        for (topic, (producers, consumers)) in &self.topics {
            if *producers > 1 {
                warnings.push(format!(
                    "  Link topic '{}' has {} producers (expected 1 for SPSC)",
                    topic.yellow(),
                    producers.to_string().red().bold()
                ));
                warnings.push(format!(
                    "    Hint: Link is Single-Producer-Single-Consumer. Use Hub<T> for multiple producers."
                ));
            }

            if *consumers > 1 {
                warnings.push(format!(
                    "  Link topic '{}' has {} consumers (expected 1 for SPSC)",
                    topic.yellow(),
                    consumers.to_string().red().bold()
                ));
                warnings.push(format!(
                    "    Hint: Link is Single-Producer-Single-Consumer. Use Hub<T> for multiple consumers."
                ));
            }
        }

        warnings
    }
}

/// AST visitor that finds Link::producer() and Link::consumer() calls
struct LinkVisitor {
    tracker: LinkUsageTracker,
}

impl LinkVisitor {
    fn new() -> Self {
        Self {
            tracker: LinkUsageTracker::default(),
        }
    }

    /// Extract topic name from string literal expression
    fn extract_topic_name(&self, expr: &Expr) -> Option<String> {
        if let Expr::Lit(lit_expr) = expr {
            if let syn::Lit::Str(lit_str) = &lit_expr.lit {
                return Some(lit_str.value());
            }
        }
        None
    }
}

impl<'ast> Visit<'ast> for LinkVisitor {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        // Check for Link::producer("topic") or Link::consumer("topic")
        if let Expr::Path(ExprPath { path, .. }) = &*node.func {
            let path_str = path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");

            // Match Link::producer or Link::consumer
            if path_str.ends_with("Link::producer") || path_str == "producer" {
                if let Some(first_arg) = node.args.first() {
                    if let Some(topic) = self.extract_topic_name(first_arg) {
                        self.tracker.add_producer(&topic);
                    }
                }
            } else if path_str.ends_with("Link::consumer") || path_str == "consumer" {
                if let Some(first_arg) = node.args.first() {
                    if let Some(topic) = self.extract_topic_name(first_arg) {
                        self.tracker.add_consumer(&topic);
                    }
                }
            }
        }

        // Continue visiting child nodes
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        // Also check for turbofish syntax: Link::<T>::producer("topic")
        // This is handled by visit_expr_call above, but we keep this for completeness
        syn::visit::visit_expr_method_call(self, node);
    }
}

/// Check a Rust file for Link usage violations
pub fn check_link_usage(file_path: &Path) -> Result<()> {
    // Read the file
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    // Parse the Rust code
    let ast: File = syn::parse_file(&content)
        .with_context(|| format!("Failed to parse Rust code in: {}", file_path.display()))?;

    // Visit the AST to find Link usage
    let mut visitor = LinkVisitor::new();
    visitor.visit_file(&ast);

    // Check for violations
    let warnings = visitor.tracker.check_violations();

    if !warnings.is_empty() {
        eprintln!();
        eprintln!("{}", "━".repeat(80).yellow());
        eprintln!("{}", "  Static Analysis Warnings".yellow().bold());
        eprintln!("{}", "━".repeat(80).yellow());
        eprintln!();

        for warning in &warnings {
            eprintln!("  {}", warning);
        }

        eprintln!();
        eprintln!(
            "  {} These are warnings, not errors. Your code will still compile.",
            "ℹ".cyan()
        );
        eprintln!(
            "  {} Fix these to avoid undefined behavior at runtime.",
            "".yellow()
        );
        eprintln!("{}", "━".repeat(80).yellow());
        eprintln!();
    }

    Ok(())
}
