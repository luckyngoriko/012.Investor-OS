//! Edge conditions for graph transitions

use super::state::SharedState;

/// Условие за преход
#[derive(Default)]
pub enum EdgeCondition {
    /// Винаги true
    #[default]
    Always,
    /// Custom predicate
    Boxed(Box<dyn Fn(&SharedState) -> bool + Send + Sync>),
}

impl EdgeCondition {
    pub fn evaluate(&self, state: &SharedState) -> bool {
        match self {
            EdgeCondition::Always => true,
            EdgeCondition::Boxed(predicate) => predicate(state),
        }
    }
}


/// Trait за edge
pub trait Edge: Send + Sync {
    fn from(&self) -> &str;
    fn to(&self) -> &str;
    fn can_transition(&self, state: &SharedState) -> bool;
}

/// Conditional edge с predicate
pub struct ConditionalEdge {
    from: String,
    to: String,
    condition: EdgeCondition,
}

impl ConditionalEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>, condition: EdgeCondition) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            condition,
        }
    }
    
    pub fn with_predicate<F>(from: impl Into<String>, to: impl Into<String>, predicate: F) -> Self
    where
        F: Fn(&SharedState) -> bool + Send + Sync + 'static,
    {
        Self::new(from, to, EdgeCondition::Boxed(Box::new(predicate)))
    }
}

impl Edge for ConditionalEdge {
    fn from(&self) -> &str { &self.from }
    fn to(&self) -> &str { &self.to }
    fn can_transition(&self, state: &SharedState) -> bool {
        self.condition.evaluate(state)
    }
}

/// Common conditions for trading graphs
pub mod conditions {
    use super::SharedState;
    
    /// Пазарът е trending
    pub fn is_trending(state: &SharedState) -> bool {
        matches!(state.market_regime, super::super::state::MarketRegime::Trending)
    }
    
    /// Пазарът е range-bound
    pub fn is_range_bound(state: &SharedState) -> bool {
        matches!(state.market_regime, super::super::state::MarketRegime::RangeBound)
    }
    
    /// Висока волатилност
    pub fn is_volatile(state: &SharedState) -> bool {
        matches!(state.market_regime, super::super::state::MarketRegime::Volatile)
    }
    
    /// CQ е достатъчно висок
    pub fn cq_above(threshold: f64) -> impl Fn(&SharedState) -> bool {
        move |state: &SharedState| {
            state.conviction_quotient.is_some_and(|cq| cq >= threshold)
        }
    }
    
    /// Risk approved
    pub fn risk_approved(state: &SharedState) -> bool {
        state.risk_approved
    }
    
    /// Има action
    pub fn has_action(state: &SharedState) -> bool {
        state.action.is_some()
    }
    
    /// Няма грешки
    pub fn no_errors(state: &SharedState) -> bool {
        state.errors.is_empty()
    }
    
    /// Комбинирано условие: AND
    pub fn and<A, B>(a: A, b: B) -> impl Fn(&SharedState) -> bool
    where
        A: Fn(&SharedState) -> bool,
        B: Fn(&SharedState) -> bool,
    {
        move |state: &SharedState| a(state) && b(state)
    }
    
    /// Комбинирано условие: OR
    pub fn or<A, B>(a: A, b: B) -> impl Fn(&SharedState) -> bool
    where
        A: Fn(&SharedState) -> bool,
        B: Fn(&SharedState) -> bool,
    {
        move |state: &SharedState| a(state) || b(state)
    }
    
    /// Комбинирано условие: NOT
    pub fn not<F>(f: F) -> impl Fn(&SharedState) -> bool
    where
        F: Fn(&SharedState) -> bool,
    {
        move |state: &SharedState| !f(state)
    }
}

/// Edge builder за fluent API
pub struct EdgeBuilder {
    from: String,
}

impl EdgeBuilder {
    pub fn from(node: impl Into<String>) -> Self {
        Self {
            from: node.into(),
        }
    }
    
    pub fn to(self, node: impl Into<String>) -> ConditionalEdge {
        ConditionalEdge::new(self.from, node, EdgeCondition::Always)
    }
    
    pub fn when<F>(self, condition: F) -> ConditionalEdgeBuilder
    where
        F: Fn(&SharedState) -> bool + Send + Sync + 'static,
    {
        ConditionalEdgeBuilder {
            from: self.from,
            condition: EdgeCondition::Boxed(Box::new(condition)),
        }
    }
}

pub struct ConditionalEdgeBuilder {
    from: String,
    condition: EdgeCondition,
}

impl ConditionalEdgeBuilder {
    pub fn to(self, node: impl Into<String>) -> ConditionalEdge {
        ConditionalEdge {
            from: self.from,
            to: node.into(),
            condition: self.condition,
        }
    }
}

/// Loop configuration
pub struct LoopConfig {
    pub max_iterations: usize,
    pub exit_condition: EdgeCondition,
}

impl LoopConfig {
    pub fn new(max_iterations: usize) -> Self {
        Self {
            max_iterations,
            exit_condition: EdgeCondition::Always,
        }
    }
    
    pub fn exit_when<F>(mut self, condition: F) -> Self
    where
        F: Fn(&SharedState) -> bool + Send + Sync + 'static,
    {
        self.exit_condition = EdgeCondition::Boxed(Box::new(condition));
        self
    }
}
