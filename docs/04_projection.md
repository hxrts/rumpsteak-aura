# Choreographic Projection Patterns

The projection algorithm transforms global choreographic protocols into local session types. Each participating role receives their own local session type. The algorithm supports patterns beyond basic send and receive operations.

## Supported Patterns

### 1. Basic Send/Receive

The global protocol specifies a send operation.

```rust
Protocol::Send {
    from: alice,
    to: bob,
    message: Data,
    continuation: End,
}
```

This defines Alice sending to Bob.

Alice's projection shows a send operation.

```rust
LocalType::Send {
    to: bob,
    message: Data,
    continuation: End,
}
```

Alice sends Data to Bob.

Bob's projection shows a receive operation.

```rust
LocalType::Receive {
    from: alice,
    message: Data,
    continuation: End,
}
```

Bob receives Data from Alice.

Charlie's projection shows no participation.

```rust
LocalType::End
```

Charlie is uninvolved in this exchange.

### 2. Communicated Choice

The global protocol specifies a choice.

```rust
Protocol::Choice {
    role: alice,
    branches: vec![
        Branch {
            label: "yes",
            protocol: Send { from: alice, to: bob, ... },
        },
        Branch {
            label: "no",
            protocol: Send { from: alice, to: bob, ... },
        },
    ],
}
```

Alice chooses between yes and no branches.

Alice's projection shows selection.

```rust
LocalType::Select {
    to: bob,
    branches: vec![
        ("yes", ...),
        ("no", ...),
    ],
}
```

Alice selects one branch and communicates it to Bob.

Bob's projection shows branching.

```rust
LocalType::Branch {
    from: alice,
    branches: vec![
        ("yes", ...),
        ("no", ...),
    ],
}
```

Bob waits for Alice's choice.

### 3. Local Choice

Local choice supports branches that do not start with send operations. This allows for local decisions without communication.

The global protocol defines a local choice.

```rust
Protocol::Choice {
    role: alice,
    branches: vec![
        Branch {
            label: "option1",
            protocol: End,
        },
        Branch {
            label: "option2",
            protocol: End,
        },
    ],
}
```

No send operation appears in the branches.

Alice's projection shows local choice.

```rust
LocalType::LocalChoice {
    branches: vec![
        ("option1", End),
        ("option2", End),
    ],
}
```

Alice makes a local decision.

The key difference is between `LocalChoice` and `Select`. Select indicates communicated choice where the selection is sent to another role. LocalChoice indicates local decision with no communication.

### 4. Loop with Conditions

Loop conditions are preserved in the projected local types.

The global protocol includes a loop.

```rust
Protocol::Loop {
    condition: Some(Condition::Count(5)),
    body: Send { from: alice, to: bob, ... },
}
```

The loop executes 5 times.

Alice's projection preserves the condition.

```rust
LocalType::Loop {
    condition: Some(Condition::Count(5)),
    body: Send { to: bob, ... },
}
```

The count condition is maintained.

Supported conditions include `Condition::Count(n)` for fixed iteration count. Use `Condition::RoleDecides(role)` for loops controlled by a role. Custom boolean expressions use `Condition::Custom(expr)`. Infinite loops use `None` and must be terminated externally.

### 5. Parallel Composition

Parallel merging includes conflict detection.

Compatible parallel composition has no conflicts.

```rust
Protocol::Parallel {
    protocols: vec![
        Send { from: alice, to: bob, ... },
        Send { from: alice, to: charlie, ... },
    ],
}
```

Different recipients avoid conflicts.

Alice's projection merges the operations.

```rust
LocalType::Send {
    to: bob,
    continuation: Send {
        to: charlie,
        continuation: End,
    },
}
```

Operations are merged sequentially. Order is non-deterministic at runtime.

Conflicting parallel composition causes errors.

```rust
Protocol::Parallel {
    protocols: vec![
        Send { from: alice, to: bob, ... },
        Send { from: alice, to: bob, ... },
    ],
}
```

Same recipient creates a conflict.

Alice's projection fails.

```rust
Err(ProjectionError::InconsistentParallel)
```

Cannot send to same recipient in parallel.

Conflict detection rules prevent invalid projections. Multiple sends to the same role create a conflict. Multiple receives from the same role create a conflict. Multiple selects to the same role create a conflict. Multiple branches from the same role create a conflict. Operations on different roles are allowed.

## Projection Rules Summary

### Chooser's View

When all branches start with send operations, the projection is `Select`. This indicates communicated choice. When branches do not start with send operations, the projection is `LocalChoice`. This indicates local decision.

### Receiver's View

When the role receives the choice, the projection is `Branch`. When the role is not involved, continuations are merged.

### Parallel Composition

When a role appears in zero branches, the projection is `End`. When a role appears in one branch, that projection is used. When a role appears in two or more branches, projections are merged if compatible. An error occurs if conflicts exist.

## Implementation Notes

### LocalType Variants

The enhanced projection algorithm uses these `LocalType` variants.

```rust
pub enum LocalType {
    Send { to, message, continuation },
    Receive { from, message, continuation },
    Select { to, branches },
    Branch { from, branches },
    LocalChoice { branches },
    Loop { condition, body },
    Rec { label, body },
    Var(label),
    End,
}
```

Each variant represents a different local type pattern.

### Code Generation

The `generate_type_expr` function in `codegen.rs` handles all variants. This includes the new `LocalChoice` and `Loop` types. Code generation transforms local types into Rust session types.
