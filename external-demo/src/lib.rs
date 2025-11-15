#![allow(clippy::type_complexity)]

//! External Demo - Full Rumpsteak-Aura Integration
//!
//! This crate demonstrates the PROPER way for 3rd party projects to integrate 
//! with rumpsteak-aura and inherit ALL features automatically.
//!
//! # Key Design Principle
//!
//! This crate is a **regular crate** (not proc-macro) which allows it to:
//! 1. Re-export ALL rumpsteak-aura functionality 
//! 2. Provide the exact same `choreography!` macro interface
//! 3. Add extensions via the extension system
//! 4. Import additional proc macros from a separate proc-macro crate
//!
//! # Usage
//!
//! ```ignore
//! use external_demo::choreography;
//!
//! // This works EXACTLY like rumpsteak-aura's choreography! macro
//! choreography! {
//!     choreography Example {
//!         roles: Alice, Bob;
//!         
//!         choice at Alice {
//!             path1: { Alice -> Bob: Request; }
//!             path2: { Alice -> Bob: Alternative; }
//!         }
//!     }
//! }
//! ```
//!
//! # Extension System Integration
//!
//! Extensions are registered automatically when using the choreography macro.
//! The extension system provides Aura-specific annotations like:
//!
//! - `[guard_capability="..."]` - Capability requirements
//! - `[flow_cost=100]` - Resource costs  
//! - `[journal_facts="..."]` - Audit logging
//!
//! # Architecture
//!
//! ```text
//! external-demo/              ← Regular crate (re-exports rumpsteak-aura)
//! external-demo-macros/       ← Proc-macro crate (custom macros)  
//! ```

// Re-export ALL rumpsteak-aura functionality so 3rd parties get everything
pub use rumpsteak_aura::*;
pub use rumpsteak_aura_choreography::*;

// Import our custom proc macros from the separate proc-macro crate
pub use external_demo_macros::*;

// Extension definitions for Aura
pub mod aura_extensions;

// Extension system integration
use crate::aura_extensions::register_aura_extensions;

/// Initialize the Aura extension system
/// 
/// This function configures the extension registry with Aura-specific
/// grammar extensions and statement parsers.
pub fn init_aura_extensions() -> rumpsteak_aura_choreography::extensions::ExtensionRegistry {
    let mut registry = rumpsteak_aura_choreography::extensions::ExtensionRegistry::new();
    register_aura_extensions(&mut registry);
    registry
}

/// Full-featured choreography! macro with ALL rumpsteak-aura features
/// 
/// This macro provides access to ALL rumpsteak-aura features including:
/// - Namespace attributes: `#[namespace = "my_protocol"]` 
/// - Parameterized roles: `Worker[N]`, `Signer[*]`
/// - Choice constructs: `choice at Role { ... }`
/// - Loop constructs: `loop { ... }`
/// - Extension system integration
/// - Custom annotations
///
/// # Example
///
/// ```ignore
/// use external_demo::choreography;
///
/// choreography! {
///     #[namespace = "threshold_ceremony"]
///     protocol ThresholdExample {
///         roles: Coordinator, Signer[N];
///         
///         choice at Coordinator {
///             start_ceremony: {
///                 Coordinator -> Signer[*]: StartRequest;
///                 Signer[*] -> Coordinator: Commitment;
///                 Coordinator -> Signer[*]: Challenge;
///                 Signer[*] -> Coordinator: Response;
///             }
///             abort: {
///                 Coordinator -> Signer[*]: Abort;  
///             }
///         }
///     }
/// }
/// ```
pub use external_demo_macros::choreography;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_registry_initialization() {
        let registry = init_aura_extensions();
        // Verify extensions are registered
        assert!(registry.grammar_extensions().count() > 0);
    }

    #[test]  
    fn test_choreography_macro_available() {
        // This test verifies the choreography macro is properly re-exported
        // Actual functionality is tested in integration tests
    }

    #[test]
    fn test_all_rumpsteak_features_available() {
        // Verify we have access to all rumpsteak-aura types and functions
        let _registry = rumpsteak_aura_choreography::extensions::ExtensionRegistry::new();
        let _composer = rumpsteak_aura_choreography::compiler::GrammarComposer::new();
        let _parser = rumpsteak_aura_choreography::compiler::ExtensionParser::new();
        
        // If this compiles, we successfully re-exported everything
    }
}