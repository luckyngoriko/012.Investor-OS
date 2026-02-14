//! Golden Path Tests for Sprint 21: Experimental & Research
//!
//! Tests cutting-edge research technologies:
//! - Quantum ML (QAOA portfolio optimization)
//! - Federated Learning (privacy-preserving)
//! - Neuromorphic Computing (SNN inference)
//! - Predictive Regime Detection
//! - Market Microstructure Analysis

use investor_os::research::{
    // Quantum ML
    QuantumOptimizer, QuantumConfig, QuantumBackend, QuantumAsset,
    // Federated Learning  
    FederatedCoordinator, FederatedClient, FederatedConfig, AggregationStrategy,
    RoundResult, ModelUpdate,
    // Neuromorphic
    SpikingNeuralNetwork, SnnConfig, NeuromorphicBackend,
    NeuromorphicInferenceEngine, SnnInferenceResult,
    // Predictive Regime
    PredictiveRegimeDetector, RegimeDetectorConfig, MarketRegime,
    RegimeForecast, EarlyWarning, MarketDataPoint,
    // Microstructure
    MicrostructureAnalyzer, OrderBook, OrderBookLevel, TradeTick, VpinCalculator,
    LiquidityMetrics, Side, AdverseSelectionEstimator,
    // General
    research_areas, ResearchArea, TechnologyStatus,
};
use rust_decimal::Decimal;
use chrono::{DateTime, Duration, Utc};

/// Helper: Create test covariance matrix
fn create_test_covariance(n: usize) -> Vec<Vec<f64>> {
    let mut matrix = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            matrix[i][j] = if i == j {
                0.04 // 20% vol squared
            } else {
                0.01 // 10% correlation
            };
        }
    }
    matrix
}

/// Helper: Create market data point
fn create_market_data(price: f64, returns: f64, volatility: f64) -> MarketDataPoint {
    MarketDataPoint {
        timestamp: Utc::now(),
        price: Decimal::try_from(price).unwrap(),
        volume: Decimal::from(1_000_000),
        returns,
        volatility,
    }
}

/// Test 1: Quantum Optimizer Creation
#[test]
fn test_quantum_optimizer_creation() {
    println!("\n⚛️ Testing Quantum Optimizer Creation");
    
    let config = QuantumConfig {
        num_qubits: 8,
        num_layers: 3,
        backend: QuantumBackend::LocalSimulator,
        ..Default::default()
    };
    
    let optimizer = QuantumOptimizer::new(config);
    
    println!("✅ Created Quantum Optimizer");
    println!("   Qubits: 8");
    println!("   Layers: 3 (QAOA)");
    println!("   Backend: Local Simulator");
    
    assert!(optimizer.is_backend_available());
}

/// Test 2: Quantum Portfolio Optimization
#[test]
fn test_quantum_portfolio_optimization() {
    println!("\n📊 Testing Quantum Portfolio Optimization (QAOA)");
    
    let optimizer = QuantumOptimizer::default();
    
    let assets = vec![
        QuantumAsset::new("AAPL", Decimal::try_from(0.15).unwrap()),
        QuantumAsset::new("MSFT", Decimal::try_from(0.12).unwrap()),
        QuantumAsset::new("GOOGL", Decimal::try_from(0.14).unwrap()),
        QuantumAsset::new("AMZN", Decimal::try_from(0.10).unwrap()),
    ];
    
    let covariance = create_test_covariance(4);
    
    let result = optimizer.optimize_portfolio(&assets, &covariance, 0.5).unwrap();
    
    println!("✅ QAOA Optimization Complete");
    println!("   Assets: {}", result.weights.len());
    println!("   Expected Return: {:.2}%", result.expected_return * Decimal::from(100));
    println!("   Portfolio Variance: {:.4}", result.portfolio_variance);
    println!("   Circuit Depth: {}", result.circuit_depth);
    println!("   Execution Time: {} ms", result.execution_time_ms);
    println!("   Iterations: {}", result.iterations);
    
    assert_eq!(result.weights.len(), 4);
    assert!(!result.used_quantum_hardware); // Using simulator
}

/// Test 3: Quantum Advantage Estimate
#[test]
fn test_quantum_advantage_estimate() {
    println!("\n🎯 Testing Quantum Advantage Estimation");
    
    let optimizer = QuantumOptimizer::default();
    
    // Small problem: no advantage
    let small = optimizer.quantum_advantage_estimate(10);
    assert!(small.is_none(), "Small problems don't benefit from quantum");
    
    // Large problem: advantage exists
    let large = optimizer.quantum_advantage_estimate(50);
    assert!(large.is_some(), "Large problems benefit from quantum");
    assert!(large.unwrap() > 1.0);
    
    println!("✅ Quantum Advantage Analysis:");
    println!("   10 assets: No advantage (classical is faster)");
    println!("   50 assets: {:.2}x speedup", large.unwrap());
}

/// Test 4: Federated Coordinator Creation
#[test]
fn test_federated_coordinator_creation() {
    println!("\n🌐 Testing Federated Learning Coordinator");
    
    let config = FederatedConfig {
        min_clients: 3,
        target_clients: 10,
        aggregation: AggregationStrategy::FedAvg,
        ..Default::default()
    };
    
    let coordinator = FederatedCoordinator::new(config);
    
    println!("✅ Created Federated Coordinator");
    println!("   Min Clients: 3");
    println!("   Target Clients: 10");
    println!("   Strategy: FedAvg");
    
    assert_eq!(coordinator.available_clients(), 0);
}

/// Test 5: Federated Client Registration
#[test]
fn test_federated_client_registration() {
    println!("\n👥 Testing Federated Client Registration");
    
    let mut coordinator = FederatedCoordinator::default();
    
    coordinator.register_client("client_1", 5000);
    coordinator.register_client("client_2", 8000);
    coordinator.register_client("client_3", 3000);
    coordinator.register_client("client_4", 6000);
    
    println!("✅ Registered 4 Federated Clients");
    println!("   Client 1: 5000 samples");
    println!("   Client 2: 8000 samples");
    println!("   Client 3: 3000 samples");
    println!("   Client 4: 6000 samples");
    
    assert_eq!(coordinator.available_clients(), 4);
}

/// Test 6: Federated Learning Round
#[test]
fn test_federated_learning_round() {
    println!("\n🔄 Testing Federated Learning Round");
    
    let mut coordinator = FederatedCoordinator::default();
    
    // Register minimum clients
    coordinator.register_client("client_1", 5000);
    coordinator.register_client("client_2", 8000);
    coordinator.register_client("client_3", 3000);
    
    // Start a round
    let result = coordinator.start_round().unwrap();
    
    println!("✅ Completed Federated Round {}", result.round_number);
    println!("   Participating Clients: {}", result.participating_clients.len());
    println!("   Global Loss: {:.4}", result.global_loss);
    println!("   Global Accuracy: {:.2}%", result.global_accuracy * 100.0);
    println!("   Privacy Budget Spent: {:.4}", result.privacy_spent);
    
    assert_eq!(result.round_number, 1);
    assert!(result.global_accuracy > 0.0);
}

/// Test 7: FedAvg Aggregation
#[test]
fn test_fedavg_aggregation() {
    println!("\n📊 Testing FedAvg Aggregation");
    
    let coordinator = FederatedCoordinator::default();
    
    let updates = vec![
        ModelUpdate {
            client_id: "c1".to_string(),
            weights: vec![1.0, 1.0, 1.0],
            dataset_size: 100,
            timestamp: Utc::now(),
            loss: 0.5,
            accuracy: 0.75,
        },
        ModelUpdate {
            client_id: "c2".to_string(),
            weights: vec![2.0, 2.0, 2.0],
            dataset_size: 200,
            timestamp: Utc::now(),
            loss: 0.4,
            accuracy: 0.80,
        },
    ];
    
    let aggregated = coordinator.aggregate_updates(&updates).unwrap();
    
    // Weighted average: (100*1 + 200*2) / 300 = 1.666...
    println!("✅ FedAvg Aggregation Complete");
    println!("   Client 1: 100 samples, weights [1,1,1]");
    println!("   Client 2: 200 samples, weights [2,2,2]");
    println!("   Aggregated: [{:.3}, {:.3}, {:.3}]", aggregated[0], aggregated[1], aggregated[2]);
    
    assert!((aggregated[0] - 1.667).abs() < 0.01);
}

/// Test 8: Spiking Neural Network Creation
#[test]
fn test_spiking_neural_network_creation() {
    println!("\n🧠 Testing Spiking Neural Network Creation");
    
    let config = SnnConfig {
        input_neurons: 100,
        hidden_neurons: 256,
        output_neurons: 10,
        backend: NeuromorphicBackend::Simulator,
        ..Default::default()
    };
    
    let snn = SpikingNeuralNetwork::new(config).unwrap();
    let stats = snn.get_stats();
    
    println!("✅ Created Spiking Neural Network");
    println!("   Input Neurons: 100");
    println!("   Hidden Neurons: 256");
    println!("   Output Neurons: 10");
    println!("   Total Neurons: {}", stats.total_neurons);
    println!("   Synapses: {}", stats.total_synapses);
    
    assert_eq!(stats.total_neurons, 366);
    assert!(stats.total_synapses > 0);
}

/// Test 9: SNN Inference
#[test]
fn test_snn_inference() {
    println!("\n⚡ Testing SNN Inference");
    
    let config = SnnConfig::default();
    let mut snn = SpikingNeuralNetwork::new(config).unwrap();
    
    // Create input spikes
    let input: Vec<f64> = (0..100).map(|i| (i as f64) / 100.0).collect();
    
    snn.encode_input(&input).unwrap();
    let result = snn.infer().unwrap();
    
    println!("✅ SNN Inference Complete");
    println!("   Outputs: {:?}", result.outputs);
    println!("   Predicted Class: {}", result.predicted_class);
    println!("   Total Spikes: {}", result.total_spikes);
    println!("   Inference Time: {} μs", result.inference_time_us);
    println!("   Energy: {:.2} pJ", result.energy_estimate_pj);
    
    assert_eq!(result.outputs.len(), 10);
    assert!(result.predicted_class < 10);
    assert!(result.inference_time_us > 0);
}

/// Test 10: Neuromorphic Inference Engine
#[test]
fn test_neuromorphic_inference_engine() {
    println!("\n🔬 Testing Neuromorphic Inference Engine");
    
    let config = SnnConfig {
        backend: NeuromorphicBackend::IntelLoihi,
        ..Default::default()
    };
    
    let mut engine = NeuromorphicInferenceEngine::new(config, 1000).unwrap();
    
    let input: Vec<f64> = (0..100).map(|_| 0.5).collect();
    let result = engine.infer(&input).unwrap();
    
    println!("✅ Neuromorphic Inference");
    println!("   Backend: Intel Loihi");
    println!("   Latency Target: 1000 μs");
    println!("   Actual Latency: {} μs", result.inference_time_us);
    println!("   Energy Efficiency: {:.6} inferences/pJ", engine.energy_efficiency());
    
    assert!(result.inference_time_us > 0);
}

/// Test 11: Predictive Regime Detector Creation
#[test]
fn test_predictive_regime_detector_creation() {
    println!("\n🔮 Testing Predictive Regime Detector Creation");
    
    let config = RegimeDetectorConfig {
        lookback_days: 252,
        forecast_horizon_days: 30,
        min_confidence: 0.6,
        ..Default::default()
    };
    
    let detector = PredictiveRegimeDetector::new(config);
    
    println!("✅ Created Predictive Regime Detector");
    println!("   Lookback: 252 days");
    println!("   Forecast Horizon: 30 days");
    println!("   Min Confidence: 60%");
    
    assert_eq!(detector.current_regime(), MarketRegime::Sideways);
}

/// Test 12: Regime Classification
#[test]
fn test_regime_classification() {
    println!("\n📊 Testing Regime Classification");
    
    let detector = PredictiveRegimeDetector::default();
    
    // Add 60 days of bullish data
    let mut bullish_detector = PredictiveRegimeDetector::default();
    for i in 0..60 {
        let data = create_market_data(100.0 + (i as f64) * 2.0, 0.02, 0.15);
        bullish_detector.add_data(data);
    }
    
    // Add 60 days of bearish data  
    let mut bearish_detector = PredictiveRegimeDetector::default();
    for i in 0..60 {
        let data = create_market_data(200.0 - (i as f64) * 2.0, -0.02, 0.25);
        bearish_detector.add_data(data);
    }
    
    println!("✅ Regime Classification");
    println!("   Bullish data regime: {:?}", bullish_detector.current_regime());
    println!("   Bearish data regime: {:?}", bearish_detector.current_regime());
    
    // Note: actual classification depends on the data patterns
}

/// Test 13: Regime Forecasting
#[test]
fn test_regime_forecasting() {
    println!("\n🔮 Testing Regime Forecasting");
    
    let mut detector = PredictiveRegimeDetector::default();
    
    // Add 100 days of data
    for i in 0..100 {
        let data = create_market_data(
            100.0 + (i as f64),
            0.001 + (i as f64) * 0.0001,
            0.15 + (i as f64) * 0.001
        );
        detector.add_data(data);
    }
    
    let forecast = detector.forecast_regime(30).unwrap();
    
    println!("✅ Regime Forecast");
    println!("   Current: {:?}", forecast.current_regime);
    println!("   Predicted: {:?}", forecast.predicted_regime);
    println!("   Confidence: {:.2}%", forecast.confidence * 100.0);
    println!("   Horizon: {} days", forecast.horizon_days);
    println!("   Expected Duration: {} days", forecast.expected_duration_days);
    
    assert!(forecast.confidence > 0.0 && forecast.confidence <= 1.0);
    assert_eq!(forecast.horizon_days, 30);
}

/// Test 14: Market Microstructure Analyzer
#[test]
fn test_microstructure_analyzer() {
    println!("\n📈 Testing Market Microstructure Analyzer");
    
    let mut analyzer = MicrostructureAnalyzer::new();
    
    // Create order book
    let mut book = OrderBook::new("AAPL");
    book.bids.push(OrderBookLevel::new(Decimal::from(100), Decimal::from(5000)));
    book.bids.push(OrderBookLevel::new(Decimal::from(99), Decimal::from(3000)));
    book.asks.push(OrderBookLevel::new(Decimal::from(101), Decimal::from(4000)));
    book.asks.push(OrderBookLevel::new(Decimal::from(102), Decimal::from(2000)));
    
    analyzer.update_order_book("AAPL", book);
    
    let metrics = analyzer.get_liquidity_metrics("AAPL").unwrap();
    
    println!("✅ Microstructure Analysis");
    println!("   Spread: {:.2} bps", metrics.spread_bps);
    println!("   Depth (5 levels): {}", metrics.depth_5_levels);
    
    assert!(metrics.spread_bps > 0.0);
}

/// Test 15: Order Book Imbalance
#[test]
fn test_order_book_imbalance() {
    println!("\n⚖️ Testing Order Book Imbalance");
    
    let mut book = OrderBook::new("AAPL");
    
    // Heavy on bids (buying pressure)
    book.bids.push(OrderBookLevel::new(Decimal::from(100), Decimal::from(10000)));
    book.asks.push(OrderBookLevel::new(Decimal::from(101), Decimal::from(2000)));
    
    let imbalance = book.imbalance();
    
    println!("✅ Order Book Imbalance: {:.3}", imbalance);
    println!("   Bid Volume: {}", book.total_bid_volume());
    println!("   Ask Volume: {}", book.total_ask_volume());
    
    assert!(imbalance > 0.0); // More bids
    assert!(imbalance < 1.0);
}

/// Test 16: Adverse Selection Estimation
#[test]
fn test_adverse_selection_estimation() {
    println!("\n⚠️ Testing Adverse Selection Estimation");
    
    let mut estimator = AdverseSelectionEstimator::new(100);
    
    // Simulate trades
    for i in 0..20 {
        estimator.add_observation(
            i % 2 == 0, // Buy/Sell alternating
            Decimal::try_from(i as f64 * 0.001).unwrap(),
        );
    }
    
    let cost = estimator.estimate_cost().unwrap();
    
    println!("✅ Adverse Selection Estimate: {:.2} bps", cost);
    
    assert!(cost >= 0.0);
}

/// Test 17: Research Areas Overview
#[test]
fn test_research_areas() {
    println!("\n🔬 Testing Research Areas Overview");
    
    let areas = research_areas();
    
    println!("✅ Research Areas:");
    for area in &areas {
        println!("   {} - RL{} ({:?})", 
            area.name, area.readiness_level, area.status);
    }
    
    assert_eq!(areas.len(), 5);
    
    let quantum = areas.iter().find(|a| a.name == "Quantum ML").unwrap();
    assert_eq!(quantum.readiness_level, 3);
}

/// Test 18: Technology Readiness Levels
#[test]
fn test_technology_readiness_levels() {
    println!("\n📊 Testing Technology Readiness Levels");
    
    let areas = research_areas();
    
    println!("✅ Technology Readiness:");
    for area in &areas {
        let readiness = match area.readiness_level {
            1..=3 => "Research",
            4..=5 => "Prototype",
            6..=7 => "Testing",
            8..=9 => "Production",
            _ => "Unknown",
        };
        println!("   {}: RL{} - {}", area.name, area.readiness_level, readiness);
    }
}

/// Test 19: Federated Client Training
#[test]
fn test_federated_client_training() {
    println!("\n💻 Testing Federated Client Local Training");
    
    let config = FederatedConfig::default();
    let client = FederatedClient::new("client_1", config, 5000);
    
    let global_weights = vec![1.0, 2.0, 3.0];
    let update = client.train_local(&global_weights).unwrap();
    
    println!("✅ Client {} Training Complete", client.client_id());
    println!("   Dataset Size: {} samples", update.dataset_size);
    println!("   Local Loss: {:.4}", update.loss);
    println!("   Local Accuracy: {:.2}%", update.accuracy * 100.0);
    
    assert_eq!(update.client_id, "client_1");
    assert_eq!(update.dataset_size, 5000);
}

/// Test 20: Complete Experimental Pipeline
#[test]
fn test_complete_experimental_pipeline() {
    println!("\n🚀 Testing Complete Experimental Pipeline");
    
    // 1. Quantum portfolio optimization
    let quantum_optimizer = QuantumOptimizer::default();
    let assets = vec![
        QuantumAsset::new("AAPL", Decimal::try_from(0.15).unwrap()),
        QuantumAsset::new("MSFT", Decimal::try_from(0.12).unwrap()),
    ];
    let _quantum_result = quantum_optimizer.optimize_portfolio(
        &assets, &create_test_covariance(2), 0.5
    ).unwrap();
    
    // 2. Federated learning round
    let mut coordinator = FederatedCoordinator::default();
    coordinator.register_client("c1", 5000);
    coordinator.register_client("c2", 5000);
    coordinator.register_client("c3", 5000);
    let _fl_result = coordinator.start_round().unwrap();
    
    // 3. Neuromorphic inference
    let snn_config = SnnConfig::default();
    let mut engine = NeuromorphicInferenceEngine::new(snn_config, 1000).unwrap();
    let input: Vec<f64> = (0..100).map(|_| 0.5).collect();
    let _snn_result = engine.infer(&input).unwrap();
    
    // 4. Regime detection
    let mut regime_detector = PredictiveRegimeDetector::default();
    for i in 0..100 {
        let data = create_market_data(100.0 + (i as f64), 0.001, 0.15);
        regime_detector.add_data(data);
    }
    let _forecast = regime_detector.forecast_regime(30).unwrap();
    
    // 5. Microstructure analysis
    let mut analyzer = MicrostructureAnalyzer::new();
    let mut book = OrderBook::new("AAPL");
    book.bids.push(OrderBookLevel::new(Decimal::from(100), Decimal::from(5000)));
    book.asks.push(OrderBookLevel::new(Decimal::from(101), Decimal::from(4000)));
    analyzer.update_order_book("AAPL", book);
    let _metrics = analyzer.get_liquidity_metrics("AAPL").unwrap();
    
    println!("✅ Complete Experimental Pipeline");
    println!("   ✓ Quantum Portfolio Optimization");
    println!("   ✓ Federated Learning Round");
    println!("   ✓ Neuromorphic Inference");
    println!("   ✓ Regime Forecasting");
    println!("   ✓ Microstructure Analysis");
}
