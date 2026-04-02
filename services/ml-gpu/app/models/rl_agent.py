"""Reinforcement Learning Trading Agent (Wave 3 Task 19).

Simple policy gradient (REINFORCE) agent for trading position sizing.
Uses a lightweight 2-layer neural network (PyTorch) with 9-feature state
and 5-action output (position sizes: -1, -0.5, 0, 0.5, 1).

No stable-baselines3 dependency -- pure PyTorch implementation.
"""

import logging
import os
from typing import List, Optional, Tuple

import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F
from torch.distributions import Categorical

logger = logging.getLogger(__name__)

# Discrete action space: position sizes
ACTION_SPACE = [-1.0, -0.5, 0.0, 0.5, 1.0]

# 9 input features (same as CatBoost pipeline):
# rsi, macd, atr, bb, obv, vol_change, pct5, pct20, consensus_confidence
STATE_DIM = 9
ACTION_DIM = len(ACTION_SPACE)


class PolicyNetwork(nn.Module):
    """2-layer feedforward policy network.

    Architecture: 9 -> 64 (ReLU) -> 5 (softmax)
    """

    def __init__(self, state_dim: int = STATE_DIM, action_dim: int = ACTION_DIM):
        super().__init__()
        self.fc1 = nn.Linear(state_dim, 64)
        self.fc2 = nn.Linear(64, action_dim)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        x = F.relu(self.fc1(x))
        return F.softmax(self.fc2(x), dim=-1)


class RLTradingAgent:
    """REINFORCE policy gradient trading agent.

    State: 9-dimensional feature vector
        [rsi, macd, atr, bb, obv, vol_change, pct5, pct20, consensus_confidence]

    Action: position size from {-1, -0.5, 0, 0.5, 1}

    Reward: realized_return - 0.5 * max_drawdown
    """

    def __init__(self, lr: float = 1e-3, gamma: float = 0.99, device: str = "auto"):
        if device == "auto":
            self.device = "cuda" if torch.cuda.is_available() else "cpu"
        else:
            self.device = device

        self.policy = PolicyNetwork().to(self.device)
        self.optimizer = torch.optim.Adam(self.policy.parameters(), lr=lr)
        self.gamma = gamma

        # Episode buffers for REINFORCE
        self._log_probs: List[torch.Tensor] = []
        self._rewards: List[float] = []

        logger.info(
            "RLTradingAgent initialised (device=%s, lr=%s, gamma=%s)",
            self.device, lr, gamma,
        )

    def predict(self, state: List[float]) -> float:
        """Select an action (position size) given a state vector.

        Returns the position size as a float in {-1, -0.5, 0, 0.5, 1}.
        Uses argmax of the policy (greedy, no exploration).
        """
        state_t = torch.FloatTensor(state).unsqueeze(0).to(self.device)
        with torch.no_grad():
            probs = self.policy(state_t)
        action_idx = torch.argmax(probs, dim=-1).item()
        return ACTION_SPACE[action_idx]

    def predict_with_probs(self, state: List[float]) -> Tuple[float, List[float]]:
        """Predict action and return action probabilities."""
        state_t = torch.FloatTensor(state).unsqueeze(0).to(self.device)
        with torch.no_grad():
            probs = self.policy(state_t)
        action_idx = torch.argmax(probs, dim=-1).item()
        return ACTION_SPACE[action_idx], probs.squeeze(0).cpu().tolist()

    def sample_action(self, state: List[float]) -> Tuple[float, int]:
        """Sample action from policy distribution (for training).

        Stores the log-probability for the REINFORCE update.
        Returns (position_size, action_index).
        """
        state_t = torch.FloatTensor(state).unsqueeze(0).to(self.device)
        probs = self.policy(state_t)
        dist = Categorical(probs)
        action_idx = dist.sample()
        self._log_probs.append(dist.log_prob(action_idx))
        return ACTION_SPACE[action_idx.item()], action_idx.item()

    def store_reward(self, reward: float):
        """Store reward for the current timestep."""
        self._rewards.append(reward)

    def train_step(self, state: List[float], action: int, reward: float) -> float:
        """Single REINFORCE training step.

        Args:
            state: 9-dim feature vector
            action: action index (0-4)
            reward: realized_return - 0.5 * max_drawdown

        Returns:
            The policy loss value.
        """
        state_t = torch.FloatTensor(state).unsqueeze(0).to(self.device)
        probs = self.policy(state_t)
        dist = Categorical(probs)
        action_t = torch.tensor([action], device=self.device)
        log_prob = dist.log_prob(action_t)

        # REINFORCE: loss = -log_prob * reward
        loss = -log_prob * reward

        self.optimizer.zero_grad()
        loss.backward()
        # Gradient clipping for stability
        torch.nn.utils.clip_grad_norm_(self.policy.parameters(), max_norm=1.0)
        self.optimizer.step()

        return loss.item()

    def train_episode(self) -> Optional[float]:
        """Train on a complete episode using stored log_probs and rewards.

        Uses discounted returns (REINFORCE with baseline subtraction).
        Clears the episode buffers afterwards.

        Returns the mean loss, or None if no data.
        """
        if not self._log_probs or not self._rewards:
            return None

        # Compute discounted returns
        returns = []
        R = 0.0
        for r in reversed(self._rewards):
            R = r + self.gamma * R
            returns.insert(0, R)

        returns_t = torch.FloatTensor(returns).to(self.device)

        # Normalise returns (baseline subtraction)
        if len(returns_t) > 1:
            returns_t = (returns_t - returns_t.mean()) / (returns_t.std() + 1e-8)

        # REINFORCE loss
        losses = []
        for log_prob, G in zip(self._log_probs, returns_t):
            losses.append(-log_prob * G)

        total_loss = torch.stack(losses).sum()

        self.optimizer.zero_grad()
        total_loss.backward()
        torch.nn.utils.clip_grad_norm_(self.policy.parameters(), max_norm=1.0)
        self.optimizer.step()

        mean_loss = total_loss.item() / len(losses)

        # Clear buffers
        self._log_probs.clear()
        self._rewards.clear()

        return mean_loss

    def save(self, path: str):
        """Save the policy network weights to disk."""
        os.makedirs(os.path.dirname(path) if os.path.dirname(path) else ".", exist_ok=True)
        torch.save({
            "policy_state_dict": self.policy.state_dict(),
            "optimizer_state_dict": self.optimizer.state_dict(),
        }, path)
        logger.info("RL agent saved to %s", path)

    def load(self, path: str) -> bool:
        """Load the policy network weights from disk.

        Returns True if loaded successfully, False otherwise.
        """
        if not os.path.exists(path):
            logger.warning("RL agent weights not found at %s", path)
            return False

        try:
            checkpoint = torch.load(path, map_location=self.device, weights_only=True)
            self.policy.load_state_dict(checkpoint["policy_state_dict"])
            self.optimizer.load_state_dict(checkpoint["optimizer_state_dict"])
            logger.info("RL agent loaded from %s", path)
            return True
        except Exception as e:
            logger.error("Failed to load RL agent from %s: %s", path, e)
            return False
