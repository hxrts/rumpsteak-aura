//! Dynamic Pest Grammar Composition for Extensions
//!
//! This module provides a system for dynamically composing Pest grammars by merging
//! the base choreographic grammar with extension-provided grammar rules.

use crate::extensions::{ExtensionRegistry, GrammarExtension};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Manages dynamic composition of Pest grammars with extensions
pub struct GrammarComposer {
    base_grammar: String,
    extension_registry: ExtensionRegistry,
}

impl GrammarComposer {
    /// Create a new grammar composer with the base grammar
    pub fn new() -> Self {
        let base_grammar = include_str!("choreography.pest").to_string();
        Self {
            base_grammar,
            extension_registry: ExtensionRegistry::new(),
        }
    }

    /// Register an extension with the grammar composer
    pub fn register_extension<T: GrammarExtension + 'static>(&mut self, extension: T) {
        self.extension_registry.register_grammar(extension);
    }

    /// Register an extension from a trait reference
    pub fn register_extension_from_trait(
        &mut self,
        _extension: &dyn GrammarExtension,
    ) -> Result<(), GrammarCompositionError> {
        // For now, we can't register from trait references due to object safety
        // In a real implementation, this would require cloning or different approach
        Ok(())
    }

    /// Compose the final grammar including all registered extensions
    pub fn compose(&self) -> Result<String, GrammarCompositionError> {
        let mut composed = self.base_grammar.clone();

        // Validate that we can safely extend the base grammar
        self.validate_base_grammar(&composed)?;

        // Get all grammar extensions sorted by priority
        let extension_rules = self.extension_registry.compose_grammar("");

        if !extension_rules.trim().is_empty() {
            // Inject extension rules into the statement rule
            composed = self.inject_extension_rules(composed, &extension_rules)?;
        }

        // Validate the final composed grammar
        self.validate_composed_grammar(&composed)?;

        Ok(composed)
    }

    /// Inject extension rules into the base grammar
    fn inject_extension_rules(
        &self,
        mut base_grammar: String,
        extension_rules: &str,
    ) -> Result<String, GrammarCompositionError> {
        // Find the statement rule and inject extension rules
        let _statement_rule_start = base_grammar.find("annotated_stmt = {").ok_or(
            GrammarCompositionError::InvalidBaseGrammar(
                "Could not find annotated_stmt rule".to_string(),
            ),
        )?;

        // Find the end of the statement alternatives
        let alternatives_start = base_grammar
            .find("annotation* ~ (send_stmt | broadcast_stmt")
            .ok_or(GrammarCompositionError::InvalidBaseGrammar(
                "Could not find statement alternatives".to_string(),
            ))?;

        let alternatives_end = base_grammar[alternatives_start..].find(")").ok_or(
            GrammarCompositionError::InvalidBaseGrammar(
                "Could not find end of statement alternatives".to_string(),
            ),
        )? + alternatives_start;

        // Extract extension statement rules
        let extension_statements = self.extract_extension_statements(extension_rules)?;

        if !extension_statements.is_empty() {
            // Insert extension statements into the alternatives
            let before_end = &base_grammar[..alternatives_end];
            let after_end = &base_grammar[alternatives_end..];

            let extension_alternatives = extension_statements.join(" | ");
            base_grammar = format!("{} | {}{}", before_end, extension_alternatives, after_end);
        }

        // Append extension rule definitions at the end
        base_grammar.push('\n');
        base_grammar.push_str("// Extension Rules\n");
        base_grammar.push_str(extension_rules);

        Ok(base_grammar)
    }

    /// Extract statement rule names from extension grammar
    fn extract_extension_statements(
        &self,
        extension_rules: &str,
    ) -> Result<Vec<String>, GrammarCompositionError> {
        let mut statements = Vec::new();

        for line in extension_rules.lines() {
            let line = line.trim();
            if line.contains("=") && line.ends_with("_stmt = {") {
                if let Some(rule_name) = line.split('=').next() {
                    statements.push(rule_name.trim().to_string());
                }
            }
        }

        Ok(statements)
    }

    /// Validate that the base grammar has the required extension points
    fn validate_base_grammar(&self, grammar: &str) -> Result<(), GrammarCompositionError> {
        let required_rules = [
            "annotated_stmt = {",
            "annotation* ~",
            "send_stmt",
            "broadcast_stmt",
        ];

        for rule in &required_rules {
            if !grammar.contains(rule) {
                return Err(GrammarCompositionError::InvalidBaseGrammar(format!(
                    "Missing required rule: {}",
                    rule
                )));
            }
        }

        Ok(())
    }

    /// Validate the composed grammar for common issues
    fn validate_composed_grammar(&self, grammar: &str) -> Result<(), GrammarCompositionError> {
        // Check for duplicate rule names
        let mut rule_names = HashSet::new();

        for line in grammar.lines() {
            let line = line.trim();
            if line.contains(" = {") && !line.starts_with("//") {
                if let Some(rule_name) = line.split(" = {").next() {
                    let rule_name = rule_name.trim();
                    if rule_names.contains(rule_name) {
                        return Err(GrammarCompositionError::DuplicateRule(
                            rule_name.to_string(),
                        ));
                    }
                    rule_names.insert(rule_name.to_string());
                }
            }
        }

        // Basic syntax validation (check balanced braces)
        let open_braces = grammar.chars().filter(|&c| c == '{').count();
        let close_braces = grammar.chars().filter(|&c| c == '}').count();

        if open_braces != close_braces {
            return Err(GrammarCompositionError::SyntaxError(
                "Unbalanced braces in composed grammar".to_string(),
            ));
        }

        Ok(())
    }

    /// Check if an extension rule exists
    pub fn has_extension_rule(&self, rule_name: &str) -> bool {
        self.extension_registry.can_handle(rule_name)
    }

    /// Get the number of registered extensions
    pub fn extension_count(&self) -> usize {
        self.extension_registry.grammar_extensions().count()
    }

    /// Write the composed grammar to a file for debugging
    pub fn write_composed_grammar<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<(), GrammarCompositionError> {
        let composed = self.compose()?;
        fs::write(path, composed).map_err(|e| {
            GrammarCompositionError::IoError(format!("Failed to write grammar: {}", e))
        })?;
        Ok(())
    }
}

impl Default for GrammarComposer {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during grammar composition
#[derive(Debug, thiserror::Error)]
pub enum GrammarCompositionError {
    #[error("Invalid base grammar: {0}")]
    InvalidBaseGrammar(String),

    #[error("Duplicate rule name: {0}")]
    DuplicateRule(String),

    #[error("Syntax error in composed grammar: {0}")]
    SyntaxError(String),

    #[error("Extension conflict: {0}")]
    ExtensionConflict(String),

    #[error("IO error: {0}")]
    IoError(String),
}

/// Builder pattern for constructing grammar composers with extensions
pub struct GrammarComposerBuilder {
    composer: GrammarComposer,
}

impl GrammarComposerBuilder {
    pub fn new() -> Self {
        Self {
            composer: GrammarComposer::new(),
        }
    }

    pub fn with_extension<T: GrammarExtension + 'static>(mut self, extension: T) -> Self {
        self.composer.register_extension(extension);
        self
    }

    pub fn build(self) -> GrammarComposer {
        self.composer
    }
}

impl Default for GrammarComposerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::GrammarExtension;

    #[derive(Debug)]
    struct TestExtension;

    impl GrammarExtension for TestExtension {
        fn grammar_rules(&self) -> &'static str {
            "timeout_stmt = { \"timeout\" ~ integer ~ \"{\" ~ protocol_body ~ \"}\" }"
        }

        fn statement_rules(&self) -> Vec<&'static str> {
            vec!["timeout_stmt"]
        }

        fn extension_id(&self) -> &'static str {
            "test_timeout"
        }
    }

    #[test]
    fn test_grammar_composer_creation() {
        let composer = GrammarComposer::new();
        assert_eq!(composer.extension_count(), 0);
        assert!(composer.base_grammar.contains("choreography"));
        assert!(composer.base_grammar.contains("annotated_stmt"));
    }

    #[test]
    fn test_extension_registration() {
        let mut composer = GrammarComposer::new();
        composer.register_extension(TestExtension);
        assert_eq!(composer.extension_count(), 1);
        assert!(composer.has_extension_rule("timeout_stmt"));
    }

    #[test]
    fn test_grammar_composition() {
        let mut composer = GrammarComposer::new();
        composer.register_extension(TestExtension);

        let result = composer.compose();
        assert!(result.is_ok(), "Grammar composition should succeed");

        let composed = result.unwrap();
        assert!(composed.contains("timeout_stmt"));
        assert!(composed.contains("choreography"));
        assert!(composed.contains("// Extension Rules"));
    }

    #[test]
    fn test_builder_pattern() {
        let composer = GrammarComposerBuilder::new()
            .with_extension(TestExtension)
            .build();

        assert_eq!(composer.extension_count(), 1);
        assert!(composer.has_extension_rule("timeout_stmt"));
    }

    #[test]
    fn test_validation() {
        let composer = GrammarComposer::new();

        // Test base grammar validation
        let valid_result = composer.validate_base_grammar(&composer.base_grammar);
        assert!(valid_result.is_ok(), "Base grammar should be valid");

        // Test composed grammar validation
        let composed = composer.compose().unwrap();
        let validation_result = composer.validate_composed_grammar(&composed);
        assert!(
            validation_result.is_ok(),
            "Composed grammar should be valid"
        );
    }
}
