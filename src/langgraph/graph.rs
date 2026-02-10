//! Graph implementation - управление на nodes и edges

use super::{state::SharedState, edges::*, nodes::*, GraphError};
use async_trait::async_trait;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Trait за граф
#[async_trait]
pub trait Graph: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(&self, initial_state: SharedState) -> Result<SharedState, GraphError>;
    fn get_node(&self, name: &str) -> Option<&dyn Node>;
}

/// Главен executor
pub struct GraphExecutor {
    graph: Arc<dyn Graph>,
    telemetry: Option<super::Telemetry>,
    max_iterations: usize,
}

impl GraphExecutor {
    pub fn new(graph: Arc<dyn Graph>, telemetry: Option<super::Telemetry>) -> Self {
        Self {
            graph,
            telemetry,
            max_iterations: 100,
        }
    }
    
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }
    
    pub async fn run(&self, state: SharedState) -> Result<SharedState, GraphError> {
        self.graph.execute(state).await
    }
}

/// Конкретна имплементация с petgraph
pub struct ExecutableGraph {
    name: String,
    graph: DiGraph<NodeInfo, EdgeInfo>,
    node_indices: HashMap<String, NodeIndex>,
    start_node: Option<String>,
    end_nodes: Vec<String>,
    max_iterations: usize,
}

struct NodeInfo {
    name: String,
    node: Box<dyn Node>,
}

struct EdgeInfo {
    condition: EdgeCondition,
}

impl ExecutableGraph {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            graph: DiGraph::new(),
            node_indices: HashMap::new(),
            start_node: None,
            end_nodes: vec![],
            max_iterations: 100,
        }
    }
    
    pub fn add_node(&mut self, name: impl Into<String>, node: Box<dyn Node>) {
        let name = name.into();
        let idx = self.graph.add_node(NodeInfo {
            name: name.clone(),
            node,
        });
        self.node_indices.insert(name, idx);
    }
    
    pub fn add_edge(&mut self, from: &str, to: &str) {
        self.add_conditional_edge(from, |_| true, to);
    }
    
    pub fn add_conditional_edge<F>(&mut self, from: &str, condition: F, to: &str)
    where
        F: Fn(&SharedState) -> bool + Send + Sync + 'static,
    {
        let from_idx = *self.node_indices.get(from)
            .unwrap_or_else(|| panic!("Node '{}' not found", from));
        let to_idx = *self.node_indices.get(to)
            .unwrap_or_else(|| panic!("Node '{}' not found", to));
        
        self.graph.add_edge(
            from_idx, 
            to_idx, 
            EdgeInfo { condition: EdgeCondition::Boxed(Box::new(condition)) }
        );
    }
    
    pub fn set_start(&mut self, node: impl Into<String>) {
        self.start_node = Some(node.into());
    }
    
    pub fn set_end(&mut self, node: impl Into<String>) {
        self.end_nodes.push(node.into());
    }
    
    pub async fn execute(&self, mut state: SharedState) -> Result<SharedState, GraphError> {
        let start_node = self.start_node.as_ref()
            .ok_or(GraphError::ExecutionError("No start node set".to_string()))?;
        
        let mut current = *self.node_indices.get(start_node)
            .ok_or(GraphError::ExecutionError(format!("Start node '{}' not found", start_node)))?;
        
        let mut iterations = 0;
        let mut visited = HashSet::new();
        
        loop {
            if iterations >= self.max_iterations {
                return Err(GraphError::MaxIterationsExceeded);
            }
            iterations += 1;
            
            // Проверка за безкраен цикъл
            if !visited.insert(current) {
                // Вече сме били тук - проверяваме дали е валиден loop
            }
            
            // Изпълняваме текущия node
            let node_info = &self.graph[current];
            let start_time = std::time::Instant::now();
            
            match node_info.node.execute(state.clone()).await {
                Ok(NodeOutput::Continue(new_state)) => {
                    state = new_state;
                }
                Ok(NodeOutput::End(final_state)) => {
                    return Ok(final_state);
                }
                Ok(NodeOutput::Jump(target, new_state)) => {
                    state = new_state;
                    current = *self.node_indices.get(&target)
                        .ok_or(GraphError::NodeError(format!("Jump target '{}' not found", target)))?;
                    continue;
                }
                Err(e) => {
                    state.add_error(&node_info.name, &e.to_string());
                    // Може да имаме error handling edge
                    return Err(GraphError::NodeError(e.to_string()));
                }
            }
            
            state.record_node_execution(&node_info.name, start_time.elapsed().as_millis() as u64);
            
            // Намираме следващия node
            let neighbors: Vec<_> = self.graph.edges(current)
                .filter(|edge| edge.weight().condition.evaluate(&state))
                .map(|edge| edge.target())
                .collect();
            
            if neighbors.is_empty() {
                // Няма къде да продължим - проверяваме дали сме в end node
                if self.end_nodes.contains(&node_info.name) {
                    return Ok(state);
                }
                return Err(GraphError::ExecutionError(
                    format!("No valid transition from '{}'", node_info.name)
                ));
            }
            
            if neighbors.len() > 1 {
                // Няколко възможни пътя - взимаме първия (или може да е грешка)
                // TODO: Приоритети или deterministic избор
            }
            
            current = neighbors[0];
        }
    }
}

#[async_trait]
impl Graph for ExecutableGraph {
    fn name(&self) -> &str {
        &self.name
    }
    
    async fn execute(&self, initial_state: SharedState) -> Result<SharedState, GraphError> {
        self.execute(initial_state).await
    }
    
    fn get_node(&self, name: &str) -> Option<&dyn Node> {
        self.node_indices.get(name)
            .map(|&idx| self.graph[idx].node.as_ref())
    }
}

/// Builder за граф
pub struct GraphBuilder {
    graph: ExecutableGraph,
}

impl GraphBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            graph: ExecutableGraph::new(name),
        }
    }
    
    pub fn add_node<N: Node + 'static>(mut self, name: impl Into<String>, node: N) -> Self {
        self.graph.add_node(name, Box::new(node));
        self
    }
    
    pub fn add_edge(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.graph.add_edge(&from.into(), &to.into());
        self
    }
    
    pub fn add_conditional_edge<F>(
        mut self, 
        from: impl Into<String>,
        condition: F,
        to: impl Into<String>
    ) -> Self 
    where
        F: Fn(&SharedState) -> bool + Send + Sync + 'static,
    {
        self.graph.add_conditional_edge(&from.into(), condition, &to.into());
        self
    }
    
    pub fn add_loop<F>(
        self,
        node: impl Into<String>,
        condition: F,
        target: impl Into<String>
    ) -> Self
    where
        F: Fn(&SharedState) -> bool + Send + Sync + 'static,
    {
        // Loop е просто conditional edge
        self.add_conditional_edge(node, condition, target)
    }
    
    pub fn set_start(mut self, node: impl Into<String>) -> Self {
        self.graph.set_start(node);
        self
    }
    
    pub fn set_end(mut self, node: impl Into<String>) -> Self {
        self.graph.set_end(node);
        self
    }
    
    pub fn build(self) -> Result<ExecutableGraph, GraphError> {
        // Валидации
        if self.graph.start_node.is_none() {
            return Err(GraphError::ExecutionError("No start node".to_string()));
        }
        
        Ok(self.graph)
    }
}

/// Визуализация на графа (за debugging)
pub fn visualize_graph(graph: &ExecutableGraph) -> String {
    let mut dot = format!("digraph {} {{\n", graph.name);
    
    // Nodes
    for (name, idx) in &graph.node_indices {
        dot.push_str(&format!("    \"{}\" [label=\"{}\"];\n", idx.index(), name));
    }
    
    // Edges
    for edge in graph.graph.edge_indices() {
        let (from, to) = graph.graph.edge_endpoints(edge).unwrap();
        dot.push_str(&format!("    \"{}\" -> \"{}\";\n", from.index(), to.index()));
    }
    
    dot.push_str("}\n");
    dot
}
