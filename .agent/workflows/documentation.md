---
description: Documentation standards for Investor OS
---
// turbo-all

# Documentation Workflow

## Required Documentation Per Crate

### README.md Template
```markdown
# {crate-name}

> Purpose: {one-line description}

## Architecture
{Brief architecture overview}

## Key Types
- `{Type}` — {description}

## Usage
\```rust
use {crate_name}::{MainType};
\```

## Tests
\```bash
cargo test -p {crate-name}
\```
```

### Inline Documentation
```rust
/// Calculate the Conviction Quotient from component signals.
///
/// # Arguments
/// * `signals` - The 6 component scores (PEGY, Insider, Sentiment, Regime, Breakout, ATR)
///
/// # Returns
/// A [`Score`] in [0.0, 1.0] representing trade conviction strength.
///
/// # Example
/// ```
/// let cq = ConvictionQuotient::from_signals(&signals);
/// assert!(cq.value() >= 0.0 && cq.value() <= 1.0);
/// ```
pub fn from_signals(signals: &SignalSet) -> Score {
    // ...
}
```

## Quality Rules
- Every public function has `///` doc comment
- Every module has `//!` module-level doc
- Every `pub struct` has field-level docs
- Examples compile (tested via `cargo test --doc`)
- Financial formulas include the math: `/// CQ = Σ(weight_i × score_i)`
