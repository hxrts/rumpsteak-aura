//! DSL Extension System for Rumpsteak-Aura
//!
//! This module provides a clean, composable system for extending choreographic DSL syntax.
//! Extensions can add new grammar rules, custom statement parsers, and protocol behaviors
//! while maintaining compatibility with the core choreographic infrastructure.

use crate::ast::{LocalType, Role};
use crate::compiler::projection::ProjectionError;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;

/// Trait for adding new grammar rules to the choreographic DSL
pub trait GrammarExtension: Send + Sync + Debug {
    /// Return the Pest grammar rules this extension provides
    fn grammar_rules(&self) -> &'static str;

    /// List of statement rule names this extension handles
    fn statement_rules(&self) -> Vec<&'static str>;

    /// Priority for conflict resolution (higher = more precedence)
    fn priority(&self) -> u32 {
        100
    }

    /// Extension identifier for debugging and registration
    fn extension_id(&self) -> &'static str;
}

/// Trait for parsing custom protocol statements
pub trait StatementParser: Send + Sync + Debug {
    /// Check if this parser can handle the given rule name
    fn can_parse(&self, rule_name: &str) -> bool;

    /// Parse a statement into a protocol extension
    ///
    /// # Arguments
    /// * `rule_name` - The grammar rule name being parsed
    /// * `content` - The matched content as a string
    /// * `context` - Parsing context with declared roles
    ///
    /// # Returns
    /// A boxed protocol extension representing the parsed statement
    fn parse_statement(
        &self,
        rule_name: &str,
        content: &str,
        context: &ParseContext,
    ) -> Result<Box<dyn ProtocolExtension>, ParseError>;
}

/// Trait for custom protocol behaviors that can be projected and validated
pub trait ProtocolExtension: Send + Sync + Debug {
    /// Unique identifier for this protocol extension type
    fn type_name(&self) -> &'static str;

    /// Check if this protocol mentions a specific role
    fn mentions_role(&self, role: &Role) -> bool;

    /// Validate this protocol against declared roles
    fn validate(&self, roles: &[Role]) -> Result<(), ExtensionValidationError>;

    /// Project this protocol to a local type for a specific role
    fn project(
        &self,
        role: &Role,
        context: &ProjectionContext,
    ) -> Result<LocalType, ProjectionError>;

    /// Generate code for this protocol extension
    fn generate_code(&self, context: &CodegenContext) -> proc_macro2::TokenStream;

    /// For trait object safety and downcasting
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn type_id(&self) -> TypeId;
}

/// Registry for managing DSL extensions
#[derive(Debug, Default)]
pub struct ExtensionRegistry {
    grammar_extensions: HashMap<String, Box<dyn GrammarExtension>>,
    statement_parsers: HashMap<String, Box<dyn StatementParser>>,
    rule_to_parser: HashMap<String, String>,
}

impl ExtensionRegistry {
    /// Create a new empty extension registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a grammar extension
    pub fn register_grammar<T: GrammarExtension + 'static>(&mut self, extension: T) {
        let id = extension.extension_id().to_string();
        let rules = extension.statement_rules();

        // Map each rule to this extension
        for rule in rules {
            self.rule_to_parser.insert(rule.to_string(), id.clone());
        }

        self.grammar_extensions.insert(id, Box::new(extension));
    }

    /// Register a statement parser
    pub fn register_parser<T: StatementParser + 'static>(&mut self, parser: T, parser_id: String) {
        self.statement_parsers.insert(parser_id, Box::new(parser));
    }

    /// Get all grammar rules from registered extensions
    pub fn compose_grammar(&self, base_grammar: &str) -> String {
        let mut composed = base_grammar.to_string();

        // Sort extensions by priority (highest first)
        let mut extensions: Vec<_> = self.grammar_extensions.values().collect();
        extensions.sort_by_key(|b| std::cmp::Reverse(b.priority()));

        for extension in extensions {
            composed.push('\n');
            composed.push_str(extension.grammar_rules());
        }

        composed
    }

    /// Find parser for a given rule name
    pub fn find_parser(&self, rule_name: &str) -> Option<&dyn StatementParser> {
        if let Some(parser_id) = self.rule_to_parser.get(rule_name) {
            self.statement_parsers.get(parser_id).map(|p| p.as_ref())
        } else {
            None
        }
    }

    /// Check if a rule is handled by an extension
    pub fn can_handle(&self, rule_name: &str) -> bool {
        self.rule_to_parser.contains_key(rule_name)
    }
}

/// Context provided during statement parsing
#[derive(Debug)]
pub struct ParseContext<'a> {
    /// Roles declared in the choreography
    pub declared_roles: &'a [Role],
    /// Original input string for error reporting
    pub input: &'a str,
}

/// Context provided during projection
#[derive(Debug)]
pub struct ProjectionContext<'a> {
    /// All roles in the choreography
    pub all_roles: &'a [Role],
    /// Current role being projected
    pub current_role: &'a Role,
}

/// Context provided during code generation
#[derive(Debug)]
pub struct CodegenContext<'a> {
    /// The choreography being generated
    pub choreography_name: &'a str,
    /// All roles in the choreography
    pub roles: &'a [Role],
    /// Namespace for generated code
    pub namespace: Option<&'a str>,
}

impl<'a> Default for CodegenContext<'a> {
    fn default() -> Self {
        Self {
            choreography_name: "Default",
            roles: &[],
            namespace: None,
        }
    }
}

/// Errors that can occur during extension parsing
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Syntax error: {message}")]
    Syntax { message: String },

    #[error("Unknown role '{role}' used in extension")]
    UnknownRole { role: String },

    #[error("Invalid extension syntax: {details}")]
    InvalidSyntax { details: String },

    #[error("Extension conflict: {message}")]
    Conflict { message: String },
}

/// Validation errors for protocol extensions
#[derive(Debug, thiserror::Error)]
pub enum ExtensionValidationError {
    #[error("Role '{role}' not declared")]
    UndeclaredRole { role: String },

    #[error("Invalid protocol structure: {reason}")]
    InvalidStructure { reason: String },

    #[error("Extension validation failed: {message}")]
    ExtensionFailed { message: String },
}

/// Convenience macro for registering extensions
#[macro_export]
macro_rules! register_extension {
    ($registry:expr, $extension:expr) => {{
        let ext = $extension;
        let id = ext.extension_id().to_string();
        $registry.register_grammar(ext);
    }};
}

/// Utility trait for easy extension registration
pub trait RegisterExtension {
    fn register_all(registry: &mut ExtensionRegistry);
}

/// Built-in extensions
pub mod timeout;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct MockGrammarExtension;

    impl GrammarExtension for MockGrammarExtension {
        fn grammar_rules(&self) -> &'static str {
            "timeout_stmt = { \"timeout\" ~ integer ~ protocol_block }"
        }

        fn statement_rules(&self) -> Vec<&'static str> {
            vec!["timeout_stmt"]
        }

        fn extension_id(&self) -> &'static str {
            "mock_timeout"
        }
    }

    #[test]
    fn test_extension_registry() {
        let mut registry = ExtensionRegistry::new();

        // Register extension
        registry.register_grammar(MockGrammarExtension);

        // Test rule mapping
        assert!(registry.can_handle("timeout_stmt"));
        assert!(!registry.can_handle("unknown_rule"));

        // Test grammar composition
        let base = "basic_rule = { \"test\" }";
        let composed = registry.compose_grammar(base);
        assert!(composed.contains("basic_rule"));
        assert!(composed.contains("timeout_stmt"));
    }

    #[test]
    fn test_parse_context() {
        use proc_macro2::Span;
        let roles = vec![
            Role::new(proc_macro2::Ident::new("Alice", Span::call_site())),
            Role::new(proc_macro2::Ident::new("Bob", Span::call_site())),
        ];

        let context = ParseContext {
            declared_roles: &roles,
            input: "test input",
        };

        assert_eq!(context.declared_roles.len(), 2);
        assert_eq!(context.input, "test input");
    }
}
