//! Physics Validation Test Suite
//!
//! Comprehensive validation of physics simulation accuracy by comparing
//! against analytical solutions and real-world physics principles.
//!
//! ## Test Categories
//!
//! - **free_fall**: Gravitational acceleration, kinematic equations
//! - **pendulum**: Periodic motion, energy conservation
//! - **collision**: Elastic/inelastic collisions, momentum conservation
//! - **friction**: Static/dynamic friction, sliding motion
//! - **joints**: Constraint satisfaction, joint limits

pub mod free_fall;
pub mod pendulum;
pub mod collision;
pub mod friction;

/// Physics validation report
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub test_name: String,
    pub passed: bool,
    pub error_percentage: f32,
    pub notes: String,
}

impl ValidationReport {
    pub fn new(test_name: impl Into<String>, passed: bool, error_percentage: f32) -> Self {
        Self {
            test_name: test_name.into(),
            passed,
            error_percentage,
            notes: String::new(),
        }
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = notes.into();
        self
    }

    pub fn print(&self) {
        let status = if self.passed { "✓ PASS" } else { "✗ FAIL" };
        println!("{}: {} (error: {:.2}%)", status, self.test_name, self.error_percentage);
        if !self.notes.is_empty() {
            println!("  Notes: {}", self.notes);
        }
    }
}

/// Run all physics validation tests and generate report
pub fn run_full_validation_suite() -> Vec<ValidationReport> {
    let mut reports = Vec::new();

    // Run Free Fall validation tests
    let free_fall_result = free_fall::run_validation();
    reports.push(free_fall_result);

    // Run Pendulum validation tests
    let pendulum_result = pendulum::run_validation();
    reports.push(pendulum_result);

    // Run Collision validation tests
    let collision_result = collision::run_validation();
    reports.push(collision_result);

    // Run Friction validation tests
    let friction_result = friction::run_validation();
    reports.push(friction_result);

    reports
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_report() {
        let report = ValidationReport::new("test", true, 0.5)
            .with_notes("All good");

        assert!(report.passed);
        assert_eq!(report.error_percentage, 0.5);
        assert!(!report.notes.is_empty());
    }
}
