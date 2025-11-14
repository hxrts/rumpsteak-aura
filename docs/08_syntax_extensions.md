# DSL Extensions Part 2: Syntax Extensions

The choreographic DSL supports syntax extensions that add new grammar rules and custom protocol constructs. This is Part 2 of the DSL extension guide.

Part 1 ([DSL Extensions Part 1: Runtime Effect System](07_effect_extensions.md)) covers runtime effects that execute during choreography interpretation.

## Overview

The syntax extension system provides four extension points. Grammar extensions add new syntax rules to the Pest grammar. Statement parsers handle parsing of custom statements. Protocol extensions define custom protocol behaviors with projection.

Code generation customizes output for extensions.

## Quick Start

This example adds a timeout extension to the choreographic DSL.

```rust
use rumpsteak_aura_choreography::{
    ExtensionParserBuilder, GrammarExtension, StatementParser, 
    ProtocolExtension, ParseContext, ProjectionContext, CodegenContext
};

// 1. Define grammar extension
#[derive(Debug)]
struct TimeoutGrammarExtension;

impl GrammarExtension for TimeoutGrammarExtension {
    fn grammar_rules(&self) -> &'static str {
        r#"timeout_stmt = { "timeout" ~ integer ~ "{" ~ protocol_body ~ "}" }"#
    }
    
    fn statement_rules(&self) -> Vec<&'static str> {
        vec!["timeout_stmt"]
    }
    
    fn extension_id(&self) -> &'static str {
        "timeout"
    }
}

// 2. Define statement parser
#[derive(Debug)]  
struct TimeoutStatementParser;

impl StatementParser for TimeoutStatementParser {
    fn can_parse(&self, rule_name: &str) -> bool {
        rule_name == "timeout_stmt"
    }
    
    fn parse_statement(
        &self, rule_name: &str, content: &str, context: &ParseContext
    ) -> Result<Box<dyn ProtocolExtension>, ParseError> {
        // Parse timeout duration and body
        let timeout_protocol = TimeoutProtocol { /* ... */ };
        Ok(Box::new(timeout_protocol))
    }
}

// 3. Define protocol extension
#[derive(Debug, Clone)]
struct TimeoutProtocol {
    duration_ms: u64,
    body: Protocol,
}

impl ProtocolExtension for TimeoutProtocol {
    fn type_name(&self) -> &'static str { "TimeoutProtocol" }
    
    fn project(&self, role: &Role, context: &ProjectionContext) 
        -> Result<LocalType, ProjectionError> {
        // Project the body and wrap with timeout
        let body_projected = project_protocol(&self.body, role)?;
        Ok(LocalType::Timeout { 
            duration: Duration::from_millis(self.duration_ms),
            body: Box::new(body_projected) 
        })
    }
    
    fn generate_code(&self, context: &CodegenContext) -> TokenStream {
        let duration = self.duration_ms;
        quote! { .with_timeout(Duration::from_millis(#duration)) }
    }
    
    // ... other required methods
}

// 4. Register and use
let parser = ExtensionParserBuilder::new()
    .with_extension(TimeoutGrammarExtension, TimeoutStatementParser)
    .build();

let choreography = parser.parse_with_extensions(r#"
    choreography Example {
        roles: Alice, Bob;
        
        timeout 5000 {
            Alice -> Bob: Request;
            Bob -> Alice: Response;
        }
    }
"#)?;
```

This example demonstrates the complete extension workflow. The `TimeoutGrammarExtension` defines new grammar rules. The `TimeoutStatementParser` converts parsed content to protocol objects. The `TimeoutProtocol` implements projection and code generation. The `ExtensionParserBuilder` registers extensions and creates a parser.

## Architecture

The extension system uses a layered architecture:

```text
┌───────────────────────────────────┐
│          User Extensions          │
│  (Grammar + Parser + Protocol)    │
├───────────────────────────────────┤
│        Extension Registry         │
│     (Composition & Dispatch)      │
├───────────────────────────────────┤
│       Grammar Composer            │
│   (Dynamic Pest Composition)      │
├───────────────────────────────────┤
│      Extension Parser             │
│  (Parse Tree → AST Conversion)    │
├───────────────────────────────────┤
│     Core Choreographic Parser     │
│    (Base Grammar & AST Types)     │
└───────────────────────────────────┘
```

The diagram shows the extension system architecture. User extensions define grammar, parsing, and protocol behavior. The extension registry coordinates composition and dispatch. The grammar composer combines extension rules with the base grammar. The extension parser converts parse trees to AST nodes.

## Creating Extensions

### Step 1: Grammar Extension

Define the syntax for your extension using Pest grammar rules:

```rust
impl GrammarExtension for MyExtension {
    fn grammar_rules(&self) -> &'static str {
        r#"
        my_stmt = { "my_keyword" ~ my_args ~ "{" ~ protocol_body ~ "}" }
        my_args = { ident ~ ("," ~ ident)* }
        "#
    }
    
    fn statement_rules(&self) -> Vec<&'static str> {
        vec!["my_stmt"]
    }
    
    fn priority(&self) -> u32 {
        200 // Higher priority = parsed first
    }
    
    fn extension_id(&self) -> &'static str {
        "my_extension"
    }
}
```

### Step 2: Statement Parser

Handle parsing the matched grammar rules into your protocol extension:

```rust
impl StatementParser for MyStatementParser {
    fn can_parse(&self, rule_name: &str) -> bool {
        rule_name == "my_stmt"
    }
    
    fn parse_statement(
        &self,
        rule_name: &str,
        content: &str, 
        context: &ParseContext
    ) -> Result<Box<dyn ProtocolExtension>, ParseError> {
        // Extract arguments from content
        let args = self.parse_arguments(content)?;
        
        // Validate against declared roles
        for arg in &args {
            if !context.declared_roles.iter().any(|r| r.name == arg) {
                return Err(ParseError::UnknownRole { role: arg.clone() });
            }
        }
        
        Ok(Box::new(MyProtocol { args }))
    }
}
```

### Step 3: Protocol Extension

Define how your extension behaves during projection and code generation:

```rust
impl ProtocolExtension for MyProtocol {
    fn type_name(&self) -> &'static str {
        "MyProtocol"  
    }
    
    fn mentions_role(&self, role: &Role) -> bool {
        self.args.iter().any(|arg| role.name == *arg)
    }
    
    fn validate(&self, roles: &[Role]) -> Result<(), ExtensionValidationError> {
        // Validate extension-specific constraints
        for arg in &self.args {
            if !roles.iter().any(|r| r.name == *arg) {
                return Err(ExtensionValidationError::UndeclaredRole { 
                    role: arg.clone() 
                });
            }
        }
        Ok(())
    }
    
    fn project(&self, role: &Role, context: &ProjectionContext) 
        -> Result<LocalType, ProjectionError> {
        // Define how this extension projects to each role
        if self.mentions_role(role) {
            Ok(LocalType::MyExtensionType { /* ... */ })
        } else {
            Ok(LocalType::End) // This role doesn't participate
        }
    }
    
    fn generate_code(&self, context: &CodegenContext) -> TokenStream {
        let args = &self.args;
        quote! {
            .with_my_extension(vec![#(#args),*])
        }
    }
    
    // Trait object boilerplate
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
    fn type_id(&self) -> TypeId { TypeId::of::<Self>() }
}
```

## Built-in Extensions

### Timeout Extension

The library includes a timeout extension as an example:

```rust
use rumpsteak_aura_choreography::extensions::timeout::register_timeout_extension;

let mut registry = ExtensionRegistry::new();
register_timeout_extension(&mut registry);

// Now you can use timeout syntax:
// timeout 5000 { Alice -> Bob: Message; }
```

## Grammar Composition

Extensions are composed with the base grammar automatically:

```rust
let composer = GrammarComposerBuilder::new()
    .with_extension(Extension1)
    .with_extension(Extension2)
    .build();

let composed_grammar = composer.compose()?;
```

The composer handles rule merging to combine extension rules with base grammar. Conflict resolution uses priority to resolve rule conflicts. Validation ensures the composed grammar is well-formed. Statement injection automatically adds extension statements to the parser.

## Error Handling

Extensions can produce detailed error messages with span information:

```rust
fn parse_my_statement(&self, content: &str) -> Result<MyProtocol, ParseError> {
    if !content.contains("required_keyword") {
        return Err(ParseError::InvalidSyntax {
            details: "My extension requires 'required_keyword'".to_string(),
        });
    }
    
    // ... rest of parsing
}
```

Errors include:
- Source location information
- Contextual error messages
- Suggestions for fixes

## Best Practices

### 1. Naming Conventions

- Use descriptive, unique rule names: `timeout_stmt`, `retry_stmt`
- Prefix with extension name to avoid conflicts: `myext_timeout_stmt`
- Use consistent naming patterns across your extension

### 2. Grammar Design

```rust
// Good: Specific, unambiguous rules
"timeout_stmt = { \"timeout\" ~ integer ~ time_unit? ~ \"{\" ~ protocol_body ~ \"}\" }"

// Avoid: Ambiguous rules that could conflict
"generic_stmt = { ident ~ ANY* }"
```

### 3. Error Messages

```rust
// Good: Specific, actionable error messages
Err(ParseError::InvalidSyntax {
    details: "Timeout duration must be a positive integer (got 'invalid')".to_string(),
})

// Avoid: Vague error messages
Err(ParseError::InvalidSyntax {
    details: "Invalid input".to_string(),
})
```

### 4. Validation

Validate extension constraints early:

```rust
impl ProtocolExtension for MyProtocol {
    fn validate(&self, roles: &[Role]) -> Result<(), ExtensionValidationError> {
        // Check role usage
        if self.participant_count > roles.len() {
            return Err(ExtensionValidationError::InvalidStructure {
                reason: format!(
                    "Extension requires {} participants but only {} roles declared",
                    self.participant_count, roles.len()
                ),
            });
        }
        
        // Check extension-specific constraints
        if self.timeout_duration > Duration::from_secs(3600) {
            return Err(ExtensionValidationError::InvalidStructure {
                reason: "Timeout duration cannot exceed 1 hour".to_string(),
            });
        }
        
        Ok(())
    }
}
```

### 5. Documentation

Document your extensions thoroughly:

```rust
/// Timeout extension for choreographic protocols
///
/// Syntax: `timeout <duration> [time_unit] { protocol_body }`
/// 
/// # Examples
/// 
/// ```ignore
/// timeout 5000 { Alice -> Bob: Request; }        // 5000ms timeout
/// timeout 30 s { Alice -> Bob: SlowRequest; }    // 30 second timeout
/// ```
/// 
/// # Projection
/// 
/// For participating roles, wraps the protocol body with a timeout.
/// Non-participating roles execute the body without timeout.
/// 
/// # Generated Code
/// 
/// ```ignore
/// .with_timeout(Duration::from_millis(5000))
/// ```
pub struct TimeoutProtocol { /* ... */ }
```

## Testing Extensions

Test your extensions thoroughly:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_timeout_parsing() {
        let parser = ExtensionParserBuilder::new()
            .with_extension(TimeoutGrammarExtension, TimeoutStatementParser)
            .build();
        
        let choreography = parser.parse_with_extensions(r#"
            choreography Test {
                roles: Alice, Bob;
                timeout 5000 {
                    Alice -> Bob: Message;
                }
            }
        "#).expect("Should parse timeout extension");
        
        // Verify the extension was parsed correctly
        assert!(matches!(choreography.protocol, Protocol::Extension { .. }));
    }
    
    #[test] 
    fn test_timeout_validation() {
        let timeout = TimeoutProtocol {
            duration: Duration::from_secs(10),
            roles: vec![Role { name: "Alice".into(), param: None }],
            body: Box::new(Protocol::End),
        };
        
        let roles = vec![Role { name: "Alice".into(), param: None }];
        assert!(timeout.validate(&roles).is_ok());
        
        // Test validation failure
        let empty_roles = vec![];
        assert!(timeout.validate(&empty_roles).is_err());
    }
    
    #[test]
    fn test_timeout_projection() {
        let timeout = TimeoutProtocol { /* ... */ };
        let alice = Role { name: "Alice".into(), param: None };
        let context = ProjectionContext { /* ... */ };
        
        let local_type = timeout.project(&alice, &context)
            .expect("Should project successfully");
        
        assert!(matches!(local_type, LocalType::Timeout { .. }));
    }
}
```

## Migration Guide

### From Manual DSL Changes

If you were previously modifying the core grammar, use extensions instead.

Before:
```pest
// Modified choreography.pest
statement = _{
    send_stmt | broadcast_stmt | choice_stmt | my_custom_stmt
}
my_custom_stmt = { "my_keyword" ~ ident }
```

After:
```rust
// Create extension
struct MyExtension;

impl GrammarExtension for MyExtension {
    fn grammar_rules(&self) -> &'static str {
        r#"my_custom_stmt = { "my_keyword" ~ ident }"#
    }
    
    fn statement_rules(&self) -> Vec<&'static str> {
        vec!["my_custom_stmt"]
    }
    
    fn extension_id(&self) -> &'static str { "my_extension" }
}
```

### From Custom Macros

If you were writing custom choreography macros:

Before:
```rust
// Custom macro for specific domain
my_choreography! {
    protocol MyProtocol {
        roles: A, B;
        A -> B: Message with_timeout 5000;
    }
}
```

After:
```rust
// Use extension system
let parser = ExtensionParserBuilder::new()
    .with_extension(TimeoutGrammarExtension, TimeoutStatementParser)
    .build();

// Standard syntax with extensions
choreography! {
    MyProtocol {
        roles: A, B;
        timeout 5000 {
            A -> B: Message;
        }
    }
}
```

## Troubleshooting

### Grammar Conflicts

If you get grammar composition errors:

1. Check rule names - Ensure no conflicts with base grammar
2. Adjust priorities - Use higher priority for more specific rules
3. Use namespaces - Prefix rules with extension name

```rust
impl GrammarExtension for MyExtension {
    fn priority(&self) -> u32 {
        300 // Higher priority than default (100)
    }
    
    fn grammar_rules(&self) -> &'static str {
        // Use prefixed rule names to avoid conflicts
        r#"myext_custom_stmt = { "my_keyword" ~ ident }"#
    }
}
```

### Parse Errors

For debugging parse errors:

```rust
// Enable detailed error reporting
let parser = ExtensionParserBuilder::new()
    .with_extension(MyExtension, MyParser)
    .build();

match parser.parse_with_extensions(input) {
    Err(ExtensionParseError::GrammarComposition(e)) => {
        eprintln!("Grammar composition failed: {}", e);
        // Check the composed grammar
        if let Ok(grammar) = parser.get_composed_grammar() {
            eprintln!("Composed grammar:\n{}", grammar);
        }
    }
    Err(e) => eprintln!("Parse error: {}", e),
    Ok(choreography) => { /* success */ }
}
```

### Performance Considerations

Extensions add some parsing overhead:

- Grammar composition happens once per parser creation
- Rule dispatch happens for each statement
- Extension validation happens during choreography validation

For performance-critical applications:
- Reuse composed parsers
- Minimize the number of extensions
- Use specific grammar rules (avoid overly general patterns)

## Examples

See `choreography/examples/extension_example.rs` for complete working examples of:

- Timeout extensions
- Priority annotations  
- Logging statements
- Extension composition
- Testing patterns

The example demonstrates best practices for creating robust, reusable extensions that integrate seamlessly with the choreographic DSL.
