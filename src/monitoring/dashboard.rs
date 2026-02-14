//! Live Dashboard
//!
//! Real-time dashboard with WebSocket updates

use chrono::{DateTime, Utc};
use std::collections::HashMap;

use super::{AnomalyType, DetectionResult, HealthStatus, MetricType};

/// Live dashboard
#[derive(Debug, Clone)]
pub struct LiveDashboard {
    pub widgets: Vec<Widget>,
    pub last_update: DateTime<Utc>,
    pub refresh_rate_ms: u32,
    pub is_connected: bool,
    pub connection_count: u32,
}

/// Dashboard widget
#[derive(Debug, Clone)]
pub struct Widget {
    pub id: String,
    pub widget_type: WidgetType,
    pub title: String,
    pub position: (u32, u32), // (x, y)
    pub size: (u32, u32),     // (width, height)
    pub data: WidgetData,
}

/// Widget type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetType {
    LineChart,
    BarChart,
    PieChart,
    Gauge,
    Number,
    Table,
    Heatmap,
    AlertList,
}

/// Widget data
#[derive(Debug, Clone)]
pub enum WidgetData {
    TimeSeries(Vec<(DateTime<Utc>, f64)>),
    SingleValue(f64),
    KeyValue(HashMap<String, String>),
    Table(Vec<Vec<String>>),
    AlertList(Vec<String>),
}

/// Metric value for dashboard
#[derive(Debug, Clone)]
pub struct MetricValue {
    pub metric_type: MetricType,
    pub value: f64,
    pub change_pct: f64,
    pub timestamp: DateTime<Utc>,
}

/// Dashboard update message
#[derive(Debug, Clone)]
pub struct DashboardUpdate {
    pub timestamp: DateTime<Utc>,
    pub update_type: UpdateType,
    pub payload: UpdatePayload,
}

/// Update type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateType {
    MetricUpdate,
    HealthUpdate,
    AlertUpdate,
    AnomalyDetected,
    PnlUpdate,
    ConnectionStatus,
}

/// Update payload
#[derive(Debug, Clone)]
pub enum UpdatePayload {
    Metric(MetricValue),
    Health(HealthStatus),
    Alert(String),
    Anomaly(AnomalyInfo),
    Pnl(f64),
    Connection(bool),
}

/// Anomaly info for dashboard
#[derive(Debug, Clone)]
pub struct AnomalyInfo {
    pub anomaly_type: AnomalyType,
    pub metric: String,
    pub score: f64,
    pub timestamp: DateTime<Utc>,
}

impl LiveDashboard {
    /// Create new dashboard
    pub fn new() -> Self {
        Self {
            widgets: Vec::new(),
            last_update: Utc::now(),
            refresh_rate_ms: 1000, // 1 second default
            is_connected: false,
            connection_count: 0,
        }
    }
    
    /// Add widget
    pub fn add_widget(&mut self, widget: Widget) {
        self.widgets.push(widget);
    }
    
    /// Remove widget
    pub fn remove_widget(&mut self, widget_id: &str) {
        self.widgets.retain(|w| w.id != widget_id);
    }
    
    /// Update widget data
    pub fn update_widget(&mut self, widget_id: &str, data: WidgetData) {
        if let Some(widget) = self.widgets.iter_mut().find(|w| w.id == widget_id) {
            widget.data = data;
            self.last_update = Utc::now();
        }
    }
    
    /// Update P&L display
    pub fn update_pnl(&mut self, total_pnl: f64) {
        self.last_update = Utc::now();
        // In real implementation, would broadcast to connected clients
    }
    
    /// Update health status
    pub fn update_health(&mut self, status: HealthStatus) {
        self.last_update = Utc::now();
        // Update health widget
    }
    
    /// Add anomaly to dashboard
    pub fn add_anomaly(&mut self, result: DetectionResult) {
        let info = AnomalyInfo {
            anomaly_type: result.anomaly_type,
            metric: result.metric_key,
            score: result.score,
            timestamp: Utc::now(),
        };
        
        // Add to anomalies widget
        self.last_update = Utc::now();
    }
    
    /// Set refresh rate
    pub fn set_refresh_rate(&mut self, rate_ms: u32) {
        self.refresh_rate_ms = rate_ms.clamp(100, 60000);
    }
    
    /// Client connected
    pub fn client_connected(&mut self) {
        self.connection_count += 1;
        self.is_connected = self.connection_count > 0;
    }
    
    /// Client disconnected
    pub fn client_disconnected(&mut self) {
        if self.connection_count > 0 {
            self.connection_count -= 1;
        }
        self.is_connected = self.connection_count > 0;
    }
    
    /// Get connection count
    pub fn connection_count(&self) -> u32 {
        self.connection_count
    }
    
    /// Create default dashboard layout
    pub fn create_default_layout(&mut self) {
        // P&L Widget
        self.add_widget(Widget {
            id: "pnl_widget".to_string(),
            widget_type: WidgetType::Number,
            title: "Total P&L".to_string(),
            position: (0, 0),
            size: (2, 1),
            data: WidgetData::SingleValue(0.0),
        });
        
        // Health Widget
        self.add_widget(Widget {
            id: "health_widget".to_string(),
            widget_type: WidgetType::Gauge,
            title: "System Health".to_string(),
            position: (2, 0),
            size: (1, 1),
            data: WidgetData::SingleValue(100.0),
        });
        
        // Metrics Chart
        self.add_widget(Widget {
            id: "metrics_chart".to_string(),
            widget_type: WidgetType::LineChart,
            title: "Key Metrics".to_string(),
            position: (0, 1),
            size: (3, 2),
            data: WidgetData::TimeSeries(Vec::new()),
        });
        
        // Alerts List
        self.add_widget(Widget {
            id: "alerts_list".to_string(),
            widget_type: WidgetType::AlertList,
            title: "Active Alerts".to_string(),
            position: (3, 0),
            size: (1, 3),
            data: WidgetData::AlertList(Vec::new()),
        });
    }
    
    /// Generate update message for WebSocket
    pub fn generate_update(&self, update_type: UpdateType, payload: UpdatePayload) -> DashboardUpdate {
        DashboardUpdate {
            timestamp: Utc::now(),
            update_type,
            payload,
        }
    }
    
    /// Get dashboard state as JSON-like structure
    pub fn to_state(&self) -> HashMap<String, serde_json::Value> {
        let mut state = HashMap::new();
        
        state.insert(
            "last_update".to_string(),
            serde_json::json!(self.last_update.to_rfc3339()),
        );
        state.insert(
            "connection_count".to_string(),
            serde_json::json!(self.connection_count),
        );
        state.insert(
            "is_connected".to_string(),
            serde_json::json!(self.is_connected),
        );
        state.insert(
            "widget_count".to_string(),
            serde_json::json!(self.widgets.len()),
        );
        
        state
    }
}

impl Default for LiveDashboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget {
    /// Create new widget
    pub fn new(id: &str, widget_type: WidgetType, title: &str) -> Self {
        Self {
            id: id.to_string(),
            widget_type,
            title: title.to_string(),
            position: (0, 0),
            size: (1, 1),
            data: WidgetData::SingleValue(0.0),
        }
    }
    
    /// Set position
    pub fn at_position(mut self, x: u32, y: u32) -> Self {
        self.position = (x, y);
        self
    }
    
    /// Set size
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.size = (width, height);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_creation() {
        let dashboard = LiveDashboard::new();
        assert!(!dashboard.is_connected);
        assert_eq!(dashboard.connection_count, 0);
        assert_eq!(dashboard.refresh_rate_ms, 1000);
    }

    #[test]
    fn test_add_remove_widget() {
        let mut dashboard = LiveDashboard::new();
        
        let widget = Widget::new("test", WidgetType::Number, "Test");
        dashboard.add_widget(widget);
        
        assert_eq!(dashboard.widgets.len(), 1);
        
        dashboard.remove_widget("test");
        assert!(dashboard.widgets.is_empty());
    }

    #[test]
    fn test_client_connections() {
        let mut dashboard = LiveDashboard::new();
        
        dashboard.client_connected();
        assert_eq!(dashboard.connection_count(), 1);
        assert!(dashboard.is_connected);
        
        dashboard.client_connected();
        assert_eq!(dashboard.connection_count(), 2);
        
        dashboard.client_disconnected();
        assert_eq!(dashboard.connection_count(), 1);
        
        dashboard.client_disconnected();
        assert_eq!(dashboard.connection_count(), 0);
        assert!(!dashboard.is_connected);
    }

    #[test]
    fn test_refresh_rate() {
        let mut dashboard = LiveDashboard::new();
        
        dashboard.set_refresh_rate(500);
        assert_eq!(dashboard.refresh_rate_ms, 500);
        
        // Test clamping
        dashboard.set_refresh_rate(50); // Too low
        assert_eq!(dashboard.refresh_rate_ms, 100);
        
        dashboard.set_refresh_rate(100000); // Too high
        assert_eq!(dashboard.refresh_rate_ms, 60000);
    }

    #[test]
    fn test_widget_builder() {
        let widget = Widget::new("chart", WidgetType::LineChart, "Metrics")
            .at_position(1, 2)
            .with_size(3, 2);
        
        assert_eq!(widget.position, (1, 2));
        assert_eq!(widget.size, (3, 2));
    }

    #[test]
    fn test_default_layout() {
        let mut dashboard = LiveDashboard::new();
        dashboard.create_default_layout();
        
        assert_eq!(dashboard.widgets.len(), 4);
        
        let pnl_widget = dashboard.widgets.iter().find(|w| w.id == "pnl_widget");
        assert!(pnl_widget.is_some());
    }

    #[test]
    fn test_widget_data() {
        let data = WidgetData::SingleValue(100.0);
        match data {
            WidgetData::SingleValue(v) => assert_eq!(v, 100.0),
            _ => panic!("Wrong data type"),
        }
    }

    #[test]
    fn test_metric_value() {
        let mv = MetricValue {
            metric_type: MetricType::Pnl,
            value: 1000.0,
            change_pct: 5.0,
            timestamp: Utc::now(),
        };
        
        assert_eq!(mv.value, 1000.0);
        assert_eq!(mv.change_pct, 5.0);
    }
}
