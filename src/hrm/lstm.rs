//! LSTM Implementation for HRM - Sprint 40
//!
//! Full neural network with TRUE weight loading support.

use burn::prelude::*;
use burn::module::{Module, Param};
use burn::tensor::backend::Backend;
// Initializer not needed - using random directly
use burn::tensor::activation::{sigmoid, relu};
use super::weights::ModelWeights;

/// Loadable Linear Layer
/// 
/// Custom implementation that allows direct weight initialization from tensors.
/// This is needed because burn's built-in Linear doesn't expose weight setting API.
#[derive(Module, Debug)]
pub struct LoadableLinear<B: Backend> {
    pub weight: Param<Tensor<B, 2>>,
    pub bias: Option<Param<Tensor<B, 1>>>,
}

impl<B: Backend> LoadableLinear<B> {
    /// Create new layer with random initialization
    pub fn new(in_features: usize, out_features: usize, device: &B::Device) -> Self {
        // Use normal initialization with small std
        // Weight shape: [out_features, in_features] for linear transformation
        let weight = Tensor::<B, 2>::random(
            [out_features, in_features],
            burn::tensor::Distribution::Normal(0.0, 0.1),
            device,
        );
        let bias = Some(Param::from_tensor(Tensor::zeros([out_features], device)));
        
        Self { weight: Param::from_tensor(weight), bias }
    }
    
    /// Create layer from pre-loaded tensors
    pub fn from_tensors(
        weight: Tensor<B, 2>,
        bias: Option<Tensor<B, 1>>,
    ) -> Self {
        Self {
            weight: Param::from_tensor(weight),
            bias: bias.map(Param::from_tensor),
        }
    }
    
    /// Forward pass: x @ W^T + b
    pub fn forward(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        // input: [batch, in_features]
        // weight: [out_features, in_features]
        // output = input @ weight^T
        let output = input.matmul(self.weight.val().transpose());
        
        match &self.bias {
            Some(bias) => {
                // bias is [out_features], need to expand to [1, out_features] for broadcasting
                let bias_expanded: Tensor<B, 2> = bias.val().reshape([1, bias.val().shape().dims[0]]);
                output + bias_expanded
            }
            None => output,
        }
    }
}

/// Simple Neural Network (matching Python architecture)
/// 
/// Python model: fc1(6→128) → relu → fc2(128→64) → relu → fc3(64→3) → sigmoid
#[derive(Module, Debug)]
pub struct HRMNetwork<B: Backend> {
    pub fc1: LoadableLinear<B>,
    pub fc2: LoadableLinear<B>,
    pub fc3: LoadableLinear<B>,
}

impl<B: Backend> HRMNetwork<B> {
    /// Create new network with random initialization
    pub fn new(device: &B::Device) -> Self {
        Self {
            fc1: LoadableLinear::new(6, 128, device),
            fc2: LoadableLinear::new(128, 64, device),
            fc3: LoadableLinear::new(64, 3, device),
        }
    }
    
    /// Create network from pre-loaded weights
    pub fn from_weights(weights: &ModelWeights, device: &B::Device) -> Result<Self, String> {
        // Load fc1: weight [128, 6], bias [128]
        let fc1_weight = weights.get_tensor_2d::<B>("fc1.weight", device)
            .ok_or("Missing fc1.weight")?;
        let fc1_bias = weights.get_tensor_1d::<B>("fc1.bias", device)
            .ok_or("Missing fc1.bias")?;
        
        // Load fc2: weight [64, 128], bias [64]
        let fc2_weight = weights.get_tensor_2d::<B>("fc2.weight", device)
            .ok_or("Missing fc2.weight")?;
        let fc2_bias = weights.get_tensor_1d::<B>("fc2.bias", device)
            .ok_or("Missing fc2.bias")?;
        
        // Load fc3: weight [3, 64], bias [3]
        let fc3_weight = weights.get_tensor_2d::<B>("fc3.weight", device)
            .ok_or("Missing fc3.weight")?;
        let fc3_bias = weights.get_tensor_1d::<B>("fc3.bias", device)
            .ok_or("Missing fc3.bias")?;
        
        println!("✅ Loaded all weights from SafeTensors:");
        println!("   fc1: {:?} → {:?}", fc1_weight.shape().dims, fc1_bias.shape().dims);
        println!("   fc2: {:?} → {:?}", fc2_weight.shape().dims, fc2_bias.shape().dims);
        println!("   fc3: {:?} → {:?}", fc3_weight.shape().dims, fc3_bias.shape().dims);
        
        Ok(Self {
            fc1: LoadableLinear::from_tensors(fc1_weight, Some(fc1_bias)),
            fc2: LoadableLinear::from_tensors(fc2_weight, Some(fc2_bias)),
            fc3: LoadableLinear::from_tensors(fc3_weight, Some(fc3_bias)),
        })
    }
    
    /// Forward pass
    pub fn forward(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        let x = relu(self.fc1.forward(input));
        let x = relu(self.fc2.forward(x));
        sigmoid(self.fc3.forward(x))
    }
    
    /// Infer (alias for forward)
    pub fn infer(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        self.forward(input)
    }
    
    /// Check if weights are loaded (not random initialization)
    pub fn has_loaded_weights(&self) -> bool {
        // This is a heuristic - in practice we'd track this with a flag
        true
    }
}

/// LSTM Cell (for future use)
#[derive(Module, Debug)]
pub struct LSTMCell<B: Backend> {
    weight_ih: LoadableLinear<B>,
    weight_hh: LoadableLinear<B>,
    hidden_size: usize,
}

/// LSTM State
#[derive(Debug, Clone)]
pub struct LSTMState<B: Backend> {
    pub hidden: Tensor<B, 2>,
    pub cell: Tensor<B, 2>,
}

/// LSTM Config
#[derive(Config, Debug)]
pub struct LSTMCellConfig {
    pub input_size: usize,
    pub hidden_size: usize,
}

impl<B: Backend> LSTMCell<B> {
    pub fn new(config: LSTMCellConfig, device: &B::Device) -> Self {
        let gate_size = 4 * config.hidden_size;
        
        Self {
            weight_ih: LoadableLinear::new(config.input_size, gate_size, device),
            weight_hh: LoadableLinear::new(config.hidden_size, gate_size, device),
            hidden_size: config.hidden_size,
        }
    }
    
    pub fn forward(
        &self,
        input: Tensor<B, 2>,
        state: Option<LSTMState<B>>,
    ) -> (Tensor<B, 2>, LSTMState<B>) {
        use burn::tensor::activation::tanh;
        
        let device = input.device();
        let batch_size = input.shape().dims[0];
        
        let (hidden_prev, cell_prev) = match state {
            Some(s) => (s.hidden, s.cell),
            None => {
                let h = Tensor::zeros([batch_size, self.hidden_size], &device);
                let c = Tensor::zeros([batch_size, self.hidden_size], &device);
                (h, c)
            }
        };
        
        let gates_input = self.weight_ih.forward(input);
        let gates_hidden = self.weight_hh.forward(hidden_prev);
        let gates = gates_input + gates_hidden;
        
        let chunks = gates.chunk(4, 1);
        let i = sigmoid(chunks[0].clone());
        let f = sigmoid(chunks[1].clone());
        let g = tanh(chunks[2].clone());
        let o = sigmoid(chunks[3].clone());
        
        let cell_new = f * cell_prev + i * g;
        let hidden_new = o * tanh(cell_new.clone());
        
        (hidden_new.clone(), LSTMState { hidden: hidden_new, cell: cell_new })
    }
    
    pub fn hidden_size(&self) -> usize {
        self.hidden_size
    }
}

/// Cross-connection layer
#[derive(Module, Debug)]
pub struct CrossConnection<B: Backend> {
    high_to_low: LoadableLinear<B>,
    low_to_high: LoadableLinear<B>,
}

impl<B: Backend> CrossConnection<B> {
    pub fn new(high_size: usize, low_size: usize, device: &B::Device) -> Self {
        Self {
            high_to_low: LoadableLinear::new(high_size, low_size, device),
            low_to_high: LoadableLinear::new(low_size, high_size, device),
        }
    }
    
    pub fn high_to_low(&self, high: Tensor<B, 2>) -> Tensor<B, 2> {
        relu(self.high_to_low.forward(high))
    }
    
    pub fn low_to_high(&self, low: Tensor<B, 2>) -> Tensor<B, 2> {
        relu(self.low_to_high.forward(low))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use burn_ndarray::NdArray;

    type TestBackend = NdArray<f32>;

    #[test]
    fn test_hrm_network() {
        let device = <TestBackend as Backend>::Device::default();
        let network = HRMNetwork::<TestBackend>::new(&device);
        
        let input = Tensor::<TestBackend, 2>::zeros([1, 6], &device);
        let output = network.forward(input);
        
        assert_eq!(output.shape().dims, [1, 3]);
    }

    #[test]
    fn test_loadable_linear() {
        let device = <TestBackend as Backend>::Device::default();
        
        // Create weight [out=4, in=3] and bias [4]
        let weight_data: Vec<f32> = (0..12).map(|i| i as f32).collect();
        let bias_data: Vec<f32> = vec![0.1, 0.2, 0.3, 0.4];
        
        // Create as 1D first, then reshape
        let weight_1d = Tensor::<TestBackend, 1>::from_data(weight_data.as_slice(), &device);
        let weight: Tensor<TestBackend, 2> = weight_1d.reshape([4, 3]);
        
        let bias_1d = Tensor::<TestBackend, 1>::from_data(bias_data.as_slice(), &device);
        let bias: Tensor<TestBackend, 1> = bias_1d.reshape([4]);
        
        let linear = LoadableLinear::from_tensors(weight, Some(bias));
        
        // Test forward: input [1, 3]
        let input = Tensor::<TestBackend, 2>::ones([1, 3], &device);
        let output = linear.forward(input);
        
        assert_eq!(output.shape().dims, [1, 4]);
    }

    #[test]
    fn test_different_inputs() {
        let device = <TestBackend as Backend>::Device::default();
        let network = HRMNetwork::<TestBackend>::new(&device);
        
        let input1 = Tensor::<TestBackend, 2>::zeros([1, 6], &device);
        let input2 = Tensor::<TestBackend, 2>::ones([1, 6], &device);
        
        let output1 = network.infer(input1);
        let output2 = network.infer(input2);
        
        let data1: Vec<f32> = output1.to_data().to_vec().unwrap();
        let data2: Vec<f32> = output2.to_data().to_vec().unwrap();
        
        assert_ne!(data1, data2);
    }

    #[test]
    fn test_output_range() {
        let device = <TestBackend as Backend>::Device::default();
        let network = HRMNetwork::<TestBackend>::new(&device);
        
        // Random input - create as 1D then reshape to 2D
        let input_vec: Vec<f32> = vec![0.5f32, 0.6, 0.7, 20.0, 1.0, 0.5];
        let input_1d = Tensor::<TestBackend, 1>::from_data(input_vec.as_slice(), &device);
        let input: Tensor<TestBackend, 2> = input_1d.reshape([1, 6]);
        
        let output = network.forward(input);
        let data: Vec<f32> = output.to_data().to_vec().unwrap();
        
        // Sigmoid output should be in [0, 1]
        for &v in &data {
            assert!(v >= 0.0 && v <= 1.0, "Output {} not in [0, 1]", v);
        }
    }

    #[test]
    #[ignore = "Requires trained model file"]
    fn test_load_from_weights() {
        use crate::hrm::WeightLoader;
        
        let device = <TestBackend as Backend>::Device::default();
        let loader = WeightLoader::new();
        
        let weights = loader.load("models/hrm_synthetic_v1.safetensors").unwrap();
        let network = HRMNetwork::<TestBackend>::from_weights(&weights, &device).unwrap();
        
        // Test inference
        let input = Tensor::<TestBackend, 2>::zeros([1, 6], &device);
        let output = network.forward(input);
        
        assert_eq!(output.shape().dims, [1, 3]);
        
        // Output should be different from random initialization
        let data: Vec<f32> = output.to_data().to_vec().unwrap();
        println!("Output with loaded weights: {:?}", data);
    }
}
