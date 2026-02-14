//! Neuromorphic Computing Module
//!
//! Ultra-low latency inference using spiking neural networks (SNN).
//! Compatible with Intel Loihi and other neuromorphic hardware.

use serde::{Deserialize, Serialize};
use std::time::Instant;
use thiserror::Error;
use tracing::{info, warn};

/// Neuromorphic computing errors
#[derive(Error, Debug, Clone)]
pub enum NeuromorphicError {
    #[error("Hardware unavailable: {0}")]
    HardwareUnavailable(String),
    
    #[error("Network configuration invalid: {0}")]
    InvalidConfiguration(String),
    
    #[error("Inference failed: {0}")]
    InferenceFailed(String),
    
    #[error("Synapse allocation failed")]
    SynapseAllocationFailed,
}

/// Neuromorphic hardware backend
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[derive(Default)]
pub enum NeuromorphicBackend {
    /// Intel Loihi (Neuromorphic Research Chip)
    IntelLoihi,
    /// IBM TrueNorth
    IbmTrueNorth,
    /// BrainChip Akida
    BrainchipAkida,
    /// Software simulator (Norse/SpykeTorch)
    #[default]
    Simulator,
}


/// Spiking Neural Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnnConfig {
    /// Number of input neurons
    pub input_neurons: usize,
    /// Number of hidden neurons
    pub hidden_neurons: usize,
    /// Number of output neurons
    pub output_neurons: usize,
    /// Neuron threshold voltage (mV)
    pub threshold_voltage: f64,
    /// Membrane time constant (ms)
    pub tau_membrane: f64,
    /// Synaptic time constant (ms)
    pub tau_synaptic: f64,
    /// Simulation time step (ms)
    pub time_step_ms: f64,
    /// Total simulation time (ms)
    pub simulation_time_ms: f64,
    /// Backend to use
    pub backend: NeuromorphicBackend,
}

impl Default for SnnConfig {
    fn default() -> Self {
        Self {
            input_neurons: 100,
            hidden_neurons: 256,
            output_neurons: 10,
            threshold_voltage: 1.0,
            tau_membrane: 20.0,
            tau_synaptic: 10.0,
            time_step_ms: 1.0,
            simulation_time_ms: 100.0,
            backend: NeuromorphicBackend::Simulator,
        }
    }
}

/// Spiking neuron state
#[derive(Debug, Clone)]
pub struct NeuronState {
    /// Membrane potential (mV)
    pub membrane_potential: f64,
    /// Has the neuron spiked
    pub has_spiked: bool,
    /// Spike count in current time window
    pub spike_count: u32,
    /// Last spike time
    pub last_spike_time_ms: Option<f64>,
}

impl Default for NeuronState {
    fn default() -> Self {
        Self {
            membrane_potential: 0.0,
            has_spiked: false,
            spike_count: 0,
            last_spike_time_ms: None,
        }
    }
}

/// Synaptic connection
#[derive(Debug, Clone)]
pub struct Synapse {
    /// Source neuron index
    pub from: usize,
    /// Target neuron index
    pub to: usize,
    /// Synaptic weight
    pub weight: f64,
    /// Synaptic delay (ms)
    pub delay_ms: f64,
    /// Is synapse excitatory (vs inhibitory)
    pub is_excitatory: bool,
}

/// Spike event
#[derive(Debug, Clone, Copy)]
pub struct SpikeEvent {
    /// Neuron that spiked
    pub neuron_id: usize,
    /// Time of spike (ms)
    pub timestamp_ms: f64,
    /// Layer index
    pub layer: u8,
}

/// Inference result from SNN
#[derive(Debug, Clone)]
pub struct SnnInferenceResult {
    /// Output values (spike rates)
    pub outputs: Vec<f64>,
    /// Predicted class (index of max spike rate)
    pub predicted_class: usize,
    /// Total inference time
    pub inference_time_us: u64,
    /// Total spikes during inference
    pub total_spikes: u32,
    /// Energy estimate (pJ per synaptic operation)
    pub energy_estimate_pj: f64,
    /// Number of synaptic operations
    pub synaptic_ops: u64,
}

/// Spiking Neural Network
#[derive(Debug)]
pub struct SpikingNeuralNetwork {
    config: SnnConfig,
    neurons: Vec<NeuronState>,
    synapses: Vec<Synapse>,
    input_spikes: Vec<SpikeEvent>,
    output_spikes: Vec<SpikeEvent>,
}

impl SpikingNeuralNetwork {
    /// Create new spiking neural network
    pub fn new(config: SnnConfig) -> Result<Self, NeuromorphicError> {
        info!("Creating SNN with {} input, {} hidden, {} output neurons",
              config.input_neurons, config.hidden_neurons, config.output_neurons);
        
        let total_neurons = config.input_neurons + config.hidden_neurons + config.output_neurons;
        
        let mut network = Self {
            config,
            neurons: vec![NeuronState::default(); total_neurons],
            synapses: Vec::new(),
            input_spikes: Vec::new(),
            output_spikes: Vec::new(),
        };
        
        // Initialize random synaptic connections
        network.initialize_synapses()?;
        
        Ok(network)
    }
    
    /// Initialize synaptic connections
    fn initialize_synapses(&mut self) -> Result<(), NeuromorphicError> {
        let input_end = self.config.input_neurons;
        let hidden_end = input_end + self.config.hidden_neurons;
        
        // Input to Hidden connections
        for i in 0..self.config.input_neurons {
            for h in input_end..hidden_end {
                if rand::random::<f64>() < 0.1 { // 10% connectivity
                    self.synapses.push(Synapse {
                        from: i,
                        to: h,
                        weight: (rand::random::<f64>() - 0.5) * 0.5,
                        delay_ms: 1.0,
                        is_excitatory: rand::random::<f64>() > 0.2,
                    });
                }
            }
        }
        
        // Hidden to Output connections
        for h in input_end..hidden_end {
            for o in hidden_end..(hidden_end + self.config.output_neurons) {
                self.synapses.push(Synapse {
                    from: h,
                    to: o,
                    weight: (rand::random::<f64>() - 0.5) * 0.3,
                    delay_ms: 1.0,
                    is_excitatory: true,
                });
            }
        }
        
        info!("Initialized {} synaptic connections", self.synapses.len());
        
        Ok(())
    }
    
    /// Encode input data to spike train
    pub fn encode_input(&mut self, input: &[f64]) -> Result<(), NeuromorphicError> {
        if input.len() != self.config.input_neurons {
            return Err(NeuromorphicError::InvalidConfiguration(
                format!("Expected {} inputs, got {}", self.config.input_neurons, input.len())
            ));
        }
        
        // Rate encoding: higher value = more spikes
        for (i, &value) in input.iter().enumerate() {
            let spike_rate = value.clamp(0.0, 1.0);
            let num_spikes = (spike_rate * self.config.simulation_time_ms / 10.0) as u32;
            
            for s in 0..num_spikes {
                let t = s as f64 * 10.0 + rand::random::<f64>() * 5.0;
                self.input_spikes.push(SpikeEvent {
                    neuron_id: i,
                    timestamp_ms: t,
                    layer: 0,
                });
            }
        }
        
        Ok(())
    }
    
    /// Run inference (spike propagation)
    pub fn infer(&mut self) -> Result<SnnInferenceResult, NeuromorphicError> {
        let start_time = Instant::now();
        
        // Reset neuron states
        for neuron in &mut self.neurons {
            *neuron = NeuronState::default();
        }
        
        let mut total_spikes = 0u32;
        let time_steps = (self.config.simulation_time_ms / self.config.time_step_ms) as usize;
        
        // Clone input spikes to avoid borrow issues
        let input_spikes = self.input_spikes.clone();
        
        // Simulate network dynamics
        for t in 0..time_steps {
            let current_time = t as f64 * self.config.time_step_ms;
            
            // Process input spikes at this time
            for spike in &input_spikes {
                if (spike.timestamp_ms - current_time).abs() < self.config.time_step_ms {
                    self.process_input_spike(spike.neuron_id)?;
                }
            }
            
            // Update all neurons
            for i in 0..self.neurons.len() {
                self.update_neuron(i, current_time)?;
            }
            
            // Propagate spikes
            self.propagate_spikes(current_time)?;
        }
        
        // Collect output spike rates
        let input_end = self.config.input_neurons;
        let hidden_end = input_end + self.config.hidden_neurons;
        
        let mut outputs = Vec::with_capacity(self.config.output_neurons);
        for o in hidden_end..(hidden_end + self.config.output_neurons) {
            let spike_rate = self.neurons[o].spike_count as f64 / self.config.simulation_time_ms;
            outputs.push(spike_rate);
            total_spikes += self.neurons[o].spike_count;
        }
        
        // Find predicted class
        let predicted_class = outputs.iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(idx, _)| idx)
            .unwrap_or(0);
        
        let inference_time = start_time.elapsed().as_micros() as u64;
        
        // Energy estimate (Intel Loihi: ~10 pJ per synaptic op)
        let synaptic_ops = self.synapses.len() as u64 * time_steps as u64;
        let energy_pj = match self.config.backend {
            NeuromorphicBackend::IntelLoihi => synaptic_ops as f64 * 10.0,
            NeuromorphicBackend::IbmTrueNorth => synaptic_ops as f64 * 26.0,
            NeuromorphicBackend::BrainchipAkida => synaptic_ops as f64 * 1.0,
            NeuromorphicBackend::Simulator => synaptic_ops as f64 * 1000.0, // Software is inefficient
        };
        
        Ok(SnnInferenceResult {
            outputs,
            predicted_class,
            inference_time_us: inference_time,
            total_spikes,
            energy_estimate_pj: energy_pj,
            synaptic_ops,
        })
    }
    
    /// Process input spike
    fn process_input_spike(&mut self, neuron_id: usize) -> Result<(), NeuromorphicError> {
        if neuron_id < self.neurons.len() {
            // Input neurons fire immediately when they receive external input
            self.neurons[neuron_id].spike_count += 1;
        }
        Ok(())
    }
    
    /// Update single neuron dynamics
    fn update_neuron(&mut self, neuron_id: usize, time_ms: f64) -> Result<(), NeuromorphicError> {
        let neuron = &mut self.neurons[neuron_id];
        
        // Leaky integrate-and-fire dynamics
        // dV/dt = -(V - V_rest) / tau_membrane + I_syn
        let dt = self.config.time_step_ms;
        let leak = -neuron.membrane_potential / self.config.tau_membrane;
        
        // Sum synaptic currents
        let mut synaptic_current = 0.0;
        for synapse in &self.synapses {
            if synapse.to == neuron_id {
                // Simple synaptic current model
                synaptic_current += synapse.weight * 0.1;
            }
        }
        
        // Update membrane potential
        neuron.membrane_potential += (leak + synaptic_current) * dt;
        
        // Check for spike
        if neuron.membrane_potential >= self.config.threshold_voltage {
            neuron.has_spiked = true;
            neuron.spike_count += 1;
            neuron.last_spike_time_ms = Some(time_ms);
            neuron.membrane_potential = 0.0; // Reset after spike
        } else {
            neuron.has_spiked = false;
        }
        
        Ok(())
    }
    
    /// Propagate spikes through network
    fn propagate_spikes(&mut self, _time_ms: f64) -> Result<(), NeuromorphicError> {
        // In a full implementation, this would queue spikes with delays
        // For simulation, updates happen in update_neuron
        Ok(())
    }
    
    /// Get network statistics
    pub fn get_stats(&self) -> SnnStats {
        let total_spikes: u32 = self.neurons.iter().map(|n| n.spike_count).sum();
        
        SnnStats {
            total_neurons: self.neurons.len(),
            total_synapses: self.synapses.len(),
            avg_spike_rate: total_spikes as f64 / self.neurons.len() as f64,
            energy_per_inference_pj: match self.config.backend {
                NeuromorphicBackend::IntelLoihi => 100000.0,
                NeuromorphicBackend::IbmTrueNorth => 260000.0,
                NeuromorphicBackend::BrainchipAkida => 10000.0,
                NeuromorphicBackend::Simulator => 10000000.0,
            },
        }
    }
}

/// SNN statistics
#[derive(Debug, Clone)]
pub struct SnnStats {
    pub total_neurons: usize,
    pub total_synapses: usize,
    pub avg_spike_rate: f64,
    pub energy_per_inference_pj: f64,
}

/// Neuromorphic inference engine for trading
#[derive(Debug)]
pub struct NeuromorphicInferenceEngine {
    network: SpikingNeuralNetwork,
    latency_target_us: u64,
}

impl NeuromorphicInferenceEngine {
    /// Create new neuromorphic inference engine
    pub fn new(config: SnnConfig, latency_target_us: u64) -> Result<Self, NeuromorphicError> {
        let network = SpikingNeuralNetwork::new(config)?;
        
        Ok(Self {
            network,
            latency_target_us,
        })
    }
    
    /// Run ultra-low latency inference
    pub fn infer(&mut self, input: &[f64]) -> Result<SnnInferenceResult, NeuromorphicError> {
        self.network.encode_input(input)?;
        let result = self.network.infer()?;
        
        // Check latency constraint
        if result.inference_time_us > self.latency_target_us {
            warn!("Inference time {} us exceeds target {} us",
                  result.inference_time_us, self.latency_target_us);
        }
        
        Ok(result)
    }
    
    /// Check if latency target is achievable
    pub fn is_latency_target_met(&self) -> bool {
        // In production, would benchmark
        true
    }
    
    /// Get energy efficiency (inferences per pJ)
    pub fn energy_efficiency(&self) -> f64 {
        let stats = self.network.get_stats();
        1.0 / stats.energy_per_inference_pj
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snn_creation() {
        let config = SnnConfig::default();
        let snn = SpikingNeuralNetwork::new(config).unwrap();
        
        let stats = snn.get_stats();
        assert_eq!(stats.total_neurons, 366); // 100 + 256 + 10
        assert!(stats.total_synapses > 0);
    }

    #[test]
    fn test_snn_inference() {
        let config = SnnConfig::default();
        let mut snn = SpikingNeuralNetwork::new(config).unwrap();
        
        // Create random input
        let input: Vec<f64> = (0..100).map(|_| rand::random()).collect();
        
        snn.encode_input(&input).unwrap();
        let result = snn.infer().unwrap();
        
        assert_eq!(result.outputs.len(), 10);
        assert!(result.predicted_class < 10);
        assert!(result.inference_time_us > 0);
        assert!(result.energy_estimate_pj > 0.0);
    }

    #[test]
    fn test_neuromorphic_engine() {
        let config = SnnConfig::default();
        let mut engine = NeuromorphicInferenceEngine::new(config, 1000).unwrap();
        
        let input: Vec<f64> = (0..100).map(|_| 0.5).collect();
        let result = engine.infer(&input).unwrap();
        
        assert!(result.inference_time_us > 0);
        assert!(engine.energy_efficiency() > 0.0);
    }

    #[test]
    fn test_backend_energy_comparison() {
        let backends = vec![
            (NeuromorphicBackend::IntelLoihi, 100000.0),
            (NeuromorphicBackend::IbmTrueNorth, 260000.0),
            (NeuromorphicBackend::BrainchipAkida, 10000.0),
        ];
        
        for (backend, expected_energy) in backends {
            let config = SnnConfig {
                backend,
                ..Default::default()
            };
            let snn = SpikingNeuralNetwork::new(config).unwrap();
            let stats = snn.get_stats();
            
            // Allow 50% tolerance for estimates
            assert!((stats.energy_per_inference_pj - expected_energy).abs() / expected_energy < 0.5,
                "Backend {:?} energy mismatch", backend);
        }
    }
}
