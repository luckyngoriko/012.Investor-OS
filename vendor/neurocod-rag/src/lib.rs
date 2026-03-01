#[derive(Debug, Clone, Default)]
pub struct ModelRouter;

#[derive(Debug, Clone, Default)]
pub struct ModelProfile;

#[derive(Debug, Clone, Default)]
pub struct RoutingDecision;

#[derive(Debug, Clone)]
pub enum QueryPriority {
    Low,
    Normal,
    High,
    Critical,
}

impl Default for QueryPriority {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone)]
pub enum RagStrategy {
    Fast,
    Balanced,
    Comprehensive,
}

impl Default for RagStrategy {
    fn default() -> Self {
        Self::Balanced
    }
}

#[derive(Debug, Clone, Default)]
pub struct CognitiveKernel;

#[derive(Debug, Clone, Default)]
pub struct ContextBudget;

#[derive(Debug, Clone)]
pub enum BudgetPreset {
    Minimal,
    Standard,
    Extensive,
}

impl Default for BudgetPreset {
    fn default() -> Self {
        Self::Standard
    }
}

#[derive(Debug, Clone, Default)]
pub struct StreamingIndexer;

#[derive(Debug, Clone, Default)]
pub struct StreamEvent;

#[derive(Debug, Clone, Default)]
pub struct Ontology;

#[derive(Debug, Clone)]
pub enum RelationType {
    Parent,
    Child,
    Related,
}

impl Default for RelationType {
    fn default() -> Self {
        Self::Related
    }
}

#[derive(Debug, Clone, Default)]
pub struct CausalTracer;

#[derive(Debug, Clone, Default)]
pub struct TraceRecord;

#[derive(Debug, Clone, Default)]
pub struct AutonomousMonitor;

#[derive(Debug, Clone)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unavailable,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::Healthy
    }
}
