//! Consensus Decision Making System
//!
//! Implements voting mechanisms for multi-agent decisions:
//! - Simple majority (>50%)
//! - Super majority (≥67%)
//! - Unanimous consent (100%)
//! - Weighted voting by agent expertise

use super::{AgentError, AgentId, VoteChoice};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Unique proposal identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProposalId(pub String);

impl ProposalId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for ProposalId {
    fn default() -> Self {
        Self::new()
    }
}

/// Trading decision for voting
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TradingDecision {
    Buy { symbol: String, quantity: Decimal, max_price: Option<Decimal> },
    Sell { symbol: String, quantity: Decimal, min_price: Option<Decimal> },
    Hold { symbol: String },
    Hedge { symbol: String, hedge_ratio: f64 },
    NoTrade,
}

impl TradingDecision {
    /// Get the symbol associated with this decision
    pub fn symbol(&self) -> Option<&str> {
        match self {
            TradingDecision::Buy { symbol, .. } => Some(symbol),
            TradingDecision::Sell { symbol, .. } => Some(symbol),
            TradingDecision::Hold { symbol } => Some(symbol),
            TradingDecision::Hedge { symbol, .. } => Some(symbol),
            TradingDecision::NoTrade => None,
        }
    }
    
    /// Check if decisions are compatible (same action type)
    pub fn is_compatible(&self, other: &TradingDecision) -> bool {
        matches!(
            (self, other),
            (TradingDecision::Buy { .. }, TradingDecision::Buy { .. })
                | (TradingDecision::Sell { .. }, TradingDecision::Sell { .. })
                | (TradingDecision::Hold { .. }, TradingDecision::Hold { .. })
                | (TradingDecision::Hedge { .. }, TradingDecision::Hedge { .. })
                | (TradingDecision::NoTrade, TradingDecision::NoTrade)
        )
    }
}

/// Proposal for consensus voting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: ProposalId,
    pub title: String,
    pub description: String,
    pub proposed_decision: TradingDecision,
    pub created_at: DateTime<Utc>,
    pub deadline: DateTime<Utc>,
    pub min_participation: f64, // Minimum participation rate (0.0 - 1.0)
    pub threshold: ConsensusThreshold,
}

impl Proposal {
    pub fn new(
        title: impl Into<String>,
        description: impl Into<String>,
        decision: TradingDecision,
        voting_duration: Duration,
        threshold: ConsensusThreshold,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: ProposalId::new(),
            title: title.into(),
            description: description.into(),
            proposed_decision: decision,
            created_at: now,
            deadline: now + chrono::Duration::from_std(voting_duration).unwrap_or_else(|_| chrono::Duration::minutes(5)),
            min_participation: 0.67, // Default: require 2/3 participation
            threshold,
        }
    }
    
    pub fn with_min_participation(mut self, rate: f64) -> Self {
        self.min_participation = rate.clamp(0.0, 1.0);
        self
    }
    
    /// Check if voting is still open
    pub fn is_open(&self) -> bool {
        Utc::now() < self.deadline
    }
}

/// Consensus threshold types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ConsensusThreshold {
    /// Simple majority (>50%)
    SimpleMajority,
    /// Super majority (≥67%)
    SuperMajority,
    /// Unanimous consent (100%)
    Unanimous,
    /// Custom threshold (0.0 - 1.0)
    Custom(f64),
}

impl ConsensusThreshold {
    /// Get the required approval ratio
    pub fn required_ratio(&self) -> f64 {
        match self {
            ConsensusThreshold::SimpleMajority => 0.5,
            ConsensusThreshold::SuperMajority => 0.67,
            ConsensusThreshold::Unanimous => 1.0,
            ConsensusThreshold::Custom(ratio) => ratio.clamp(0.0, 1.0),
        }
    }
    
    /// Check if a ratio meets the threshold
    pub fn is_met(&self, approval_ratio: f64) -> bool {
        approval_ratio >= self.required_ratio()
    }
}

/// Vote with agent weight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightedVote {
    pub agent_id: AgentId,
    pub vote: VoteChoice,
    pub weight: f64,
    pub confidence: f64,
    pub rationale: String,
    pub timestamp: DateTime<Utc>,
}

impl WeightedVote {
    pub fn new(
        agent_id: AgentId,
        vote: VoteChoice,
        weight: f64,
        confidence: f64,
        rationale: impl Into<String>,
    ) -> Self {
        Self {
            agent_id,
            vote,
            weight,
            confidence,
            rationale: rationale.into(),
            timestamp: Utc::now(),
        }
    }
}

/// Consensus voting result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConsensusResult {
    /// Consensus reached
    Approved(ApprovedDecision),
    /// Consensus not reached
    Rejected(RejectedDecision),
    /// Not enough participation
    InsufficientParticipation {
        participation_rate: f64,
        required_rate: f64,
    },
    /// Voting timeout
    Timeout,
    /// Tie (for simple majority)
    Tie,
}

/// Approved decision details
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApprovedDecision {
    pub proposal_id: ProposalId,
    pub decision: TradingDecision,
    pub agreement_level: f64,
    pub votes_for: Vec<AgentId>,
    pub votes_against: Vec<AgentId>,
    pub abstentions: Vec<AgentId>,
    pub total_weight: f64,
    pub for_weight: f64,
}

/// Rejected decision details
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RejectedDecision {
    pub proposal_id: ProposalId,
    pub proposed_decision: TradingDecision,
    pub rejection_reason: RejectionReason,
    pub votes_for: Vec<AgentId>,
    pub votes_against: Vec<AgentId>,
    pub abstentions: Vec<AgentId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RejectionReason {
    MajorityAgainst,
    InsufficientParticipation,
    Timeout,
    Tie,
}

/// Consensus engine for multi-agent voting
pub struct ConsensusEngine {
    /// Active proposals
    proposals: HashMap<ProposalId, ProposalState>,
    /// Vote history
    vote_history: Vec<(ProposalId, WeightedVote)>,
}

/// State of an active proposal
#[derive(Debug, Clone)]
struct ProposalState {
    proposal: Proposal,
    votes: HashMap<AgentId, WeightedVote>,
    eligible_voters: Vec<AgentId>,
}

impl ConsensusEngine {
    pub fn new() -> Self {
        Self {
            proposals: HashMap::new(),
            vote_history: Vec::new(),
        }
    }
    
    /// Create a new proposal for voting
    pub fn create_proposal(
        &mut self,
        proposal: Proposal,
        eligible_voters: Vec<AgentId>,
    ) -> ProposalId {
        let id = proposal.id.clone();
        let state = ProposalState {
            proposal,
            votes: HashMap::new(),
            eligible_voters,
        };
        
        self.proposals.insert(id.clone(), state);
        info!("Created proposal {} for consensus voting", id.0);
        
        id
    }
    
    /// Cast a vote
    pub fn vote(
        &mut self,
        proposal_id: &ProposalId,
        vote: WeightedVote,
    ) -> Result<(), AgentError> {
        let voter_id = vote.agent_id.clone(); // Clone before moving
        
        let state = self.proposals.get_mut(proposal_id)
            .ok_or_else(|| AgentError::AgentNotFound(AgentId::from_string("proposal")))?;
        
        // Check if agent is eligible
        if !state.eligible_voters.contains(&voter_id) {
            return Err(AgentError::AgentError(
                format!("Agent {} is not eligible to vote", voter_id)
            ));
        }
        
        // Check if voting is still open
        if !state.proposal.is_open() {
            return Err(AgentError::AgentError("Voting period has closed".to_string()));
        }
        
        // Record vote
        state.votes.insert(voter_id.clone(), vote.clone());
        self.vote_history.push((proposal_id.clone(), vote));
        
        debug!("Recorded vote from {} for proposal {}", voter_id, proposal_id.0);
        Ok(())
    }
    
    /// Check if consensus has been reached
    pub fn check_consensus(&self, proposal_id: &ProposalId) -> Option<ConsensusResult> {
        let state = self.proposals.get(proposal_id)?;
        let proposal = &state.proposal;
        
        // Check if deadline passed
        if !proposal.is_open() {
            return Some(self.calculate_result(state));
        }
        
        // Check if we have enough votes for a decision
        let participation_rate = state.votes.len() as f64 / state.eligible_voters.len() as f64;
        if participation_rate >= proposal.min_participation {
            let result = self.calculate_result(state);
            // Only return if consensus is definitive
            match &result {
                ConsensusResult::Approved(_) | ConsensusResult::Tie => return Some(result),
                _ => {}
            }
        }
        
        None // Still waiting for more votes
    }
    
    /// Finalize voting and get result
    pub fn finalize(&mut self, proposal_id: &ProposalId) -> ConsensusResult {
        let state = match self.proposals.remove(proposal_id) {
            Some(s) => s,
            None => return ConsensusResult::Timeout,
        };
        
        self.calculate_result(&state)
    }
    
    /// Calculate consensus result
    fn calculate_result(&self, state: &ProposalState) -> ConsensusResult {
        let proposal = &state.proposal;
        let total_eligible = state.eligible_voters.len() as f64;
        let participation = state.votes.len() as f64 / total_eligible;
        
        // Check minimum participation
        if participation < proposal.min_participation {
            return ConsensusResult::InsufficientParticipation {
                participation_rate: participation,
                required_rate: proposal.min_participation,
            };
        }
        
        // Calculate weighted votes
        let mut total_weight = 0.0;
        let mut for_weight = 0.0;
        let mut against_weight = 0.0;
        
        let mut votes_for = Vec::new();
        let mut votes_against = Vec::new();
        let mut abstentions = Vec::new();
        
        for (agent_id, vote) in &state.votes {
            total_weight += vote.weight;
            
            match vote.vote {
                VoteChoice::For => {
                    for_weight += vote.weight;
                    votes_for.push(agent_id.clone());
                }
                VoteChoice::Against => {
                    against_weight += vote.weight;
                    votes_against.push(agent_id.clone());
                }
                VoteChoice::Abstain => {
                    abstentions.push(agent_id.clone());
                }
            }
        }
        
        // Check for tie (only relevant for simple majority)
        if for_weight == against_weight && matches!(proposal.threshold, ConsensusThreshold::SimpleMajority) {
            return ConsensusResult::Tie;
        }
        
        let approval_ratio = if total_weight > 0.0 {
            for_weight / total_weight
        } else {
            0.0
        };
        
        // Check threshold
        if proposal.threshold.is_met(approval_ratio) {
            ConsensusResult::Approved(ApprovedDecision {
                proposal_id: proposal.id.clone(),
                decision: proposal.proposed_decision.clone(),
                agreement_level: approval_ratio,
                votes_for,
                votes_against,
                abstentions,
                total_weight,
                for_weight,
            })
        } else {
            ConsensusResult::Rejected(RejectedDecision {
                proposal_id: proposal.id.clone(),
                proposed_decision: proposal.proposed_decision.clone(),
                rejection_reason: if for_weight == against_weight {
                    RejectionReason::Tie
                } else {
                    RejectionReason::MajorityAgainst
                },
                votes_for,
                votes_against,
                abstentions,
            })
        }
    }
    
    /// Get proposal status
    pub fn get_status(&self, proposal_id: &ProposalId) -> Option<ProposalStatus> {
        let state = self.proposals.get(proposal_id)?;
        
        Some(ProposalStatus {
            proposal_id: proposal_id.clone(),
            total_votes: state.votes.len(),
            eligible_voters: state.eligible_voters.len(),
            participation_rate: state.votes.len() as f64 / state.eligible_voters.len() as f64,
            is_open: state.proposal.is_open(),
            deadline: state.proposal.deadline,
        })
    }
    
    /// Get all active proposals
    pub fn get_active_proposals(&self) -> Vec<ProposalId> {
        self.proposals
            .iter()
            .filter(|(_, state)| state.proposal.is_open())
            .map(|(id, _)| id.clone())
            .collect()
    }
    
    /// Get vote history for an agent
    pub fn get_agent_votes(&self, agent_id: &AgentId) -> Vec<(ProposalId, WeightedVote)> {
        self.vote_history
            .iter()
            .filter(|(_, vote)| &vote.agent_id == agent_id)
            .cloned()
            .collect()
    }
}

impl Default for ConsensusEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Proposal status summary
#[derive(Debug, Clone)]
pub struct ProposalStatus {
    pub proposal_id: ProposalId,
    pub total_votes: usize,
    pub eligible_voters: usize,
    pub participation_rate: f64,
    pub is_open: bool,
    pub deadline: DateTime<Utc>,
}

/// Quick consensus for simple decisions
pub async fn quick_consensus(
    engine: &mut ConsensusEngine,
    proposal: Proposal,
    eligible_voters: Vec<AgentId>,
    votes: Vec<WeightedVote>,
) -> ConsensusResult {
    let proposal_id = engine.create_proposal(proposal, eligible_voters);
    
    // Collect all votes
    for vote in votes {
        if let Err(e) = engine.vote(&proposal_id, vote) {
            warn!("Failed to record vote: {}", e);
        }
    }
    
    // Finalize immediately
    engine.finalize(&proposal_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_thresholds() {
        assert!(ConsensusThreshold::SimpleMajority.is_met(0.51));
        assert!(!ConsensusThreshold::SimpleMajority.is_met(0.49));
        
        assert!(ConsensusThreshold::SuperMajority.is_met(0.67));
        assert!(!ConsensusThreshold::SuperMajority.is_met(0.66));
        
        assert!(ConsensusThreshold::Unanimous.is_met(1.0));
        assert!(!ConsensusThreshold::Unanimous.is_met(0.99));
    }

    #[test]
    fn test_simple_majority_consensus() {
        let mut engine = ConsensusEngine::new();
        
        let voters = vec![
            AgentId::from_string("agent1"),
            AgentId::from_string("agent2"),
            AgentId::from_string("agent3"),
        ];
        
        let proposal = Proposal::new(
            "Test Proposal",
            "Description",
            TradingDecision::Buy { symbol: "AAPL".to_string(), quantity: Decimal::from(100), max_price: None },
            Duration::from_secs(60),
            ConsensusThreshold::SimpleMajority,
        ).with_min_participation(0.5);
        
        let proposal_id = engine.create_proposal(proposal, voters.clone());
        
        // 2 out of 3 vote for
        engine.vote(&proposal_id, WeightedVote::new(
            voters[0].clone(), VoteChoice::For, 1.0, 0.9, "Good opportunity"
        )).unwrap();
        
        engine.vote(&proposal_id, WeightedVote::new(
            voters[1].clone(), VoteChoice::For, 1.0, 0.8, "Agree"
        )).unwrap();
        
        engine.vote(&proposal_id, WeightedVote::new(
            voters[2].clone(), VoteChoice::Against, 1.0, 0.6, "Too risky"
        )).unwrap();
        
        let result = engine.finalize(&proposal_id);
        
        match result {
            ConsensusResult::Approved(decision) => {
                assert_eq!(decision.agreement_level, 2.0 / 3.0);
                assert_eq!(decision.votes_for.len(), 2);
                assert_eq!(decision.votes_against.len(), 1);
            }
            _ => panic!("Expected approval"),
        }
    }

    #[test]
    fn test_unanimous_consensus_fails() {
        let mut engine = ConsensusEngine::new();
        
        let voters = vec![
            AgentId::from_string("agent1"),
            AgentId::from_string("agent2"),
            AgentId::from_string("agent3"),
        ];
        
        let proposal = Proposal::new(
            "Test Proposal",
            "Description",
            TradingDecision::Hold { symbol: "AAPL".to_string() },
            Duration::from_secs(60),
            ConsensusThreshold::Unanimous,
        );
        
        let proposal_id = engine.create_proposal(proposal, voters.clone());
        
        // Not unanimous
        engine.vote(&proposal_id, WeightedVote::new(
            voters[0].clone(), VoteChoice::For, 1.0, 1.0, "Agree"
        )).unwrap();
        
        engine.vote(&proposal_id, WeightedVote::new(
            voters[1].clone(), VoteChoice::For, 1.0, 1.0, "Agree"
        )).unwrap();
        
        engine.vote(&proposal_id, WeightedVote::new(
            voters[2].clone(), VoteChoice::Against, 1.0, 1.0, "Disagree"
        )).unwrap();
        
        let result = engine.finalize(&proposal_id);
        
        match result {
            ConsensusResult::Rejected(_) => {}
            _ => panic!("Expected rejection"),
        }
    }

    #[test]
    fn test_weighted_voting() {
        let mut engine = ConsensusEngine::new();
        
        let voters = vec![
            AgentId::from_string("expert"),
            AgentId::from_string("novice1"),
            AgentId::from_string("novice2"),
        ];
        
        let proposal = Proposal::new(
            "Weighted Vote",
            "Test",
            TradingDecision::Sell { symbol: "TSLA".to_string(), quantity: Decimal::from(50), min_price: None },
            Duration::from_secs(60),
            ConsensusThreshold::SimpleMajority,
        );
        
        let proposal_id = engine.create_proposal(proposal, voters.clone());
        
        // Expert votes against with high weight
        engine.vote(&proposal_id, WeightedVote::new(
            voters[0].clone(), VoteChoice::Against, 3.0, 0.95, "Strong signal"
        )).unwrap();
        
        // Novices vote for with low weight
        engine.vote(&proposal_id, WeightedVote::new(
            voters[1].clone(), VoteChoice::For, 1.0, 0.5, "Maybe"
        )).unwrap();
        
        engine.vote(&proposal_id, WeightedVote::new(
            voters[2].clone(), VoteChoice::For, 1.0, 0.5, "Maybe"
        )).unwrap();
        
        let result = engine.finalize(&proposal_id);
        
        // Should be rejected: 3.0 against vs 2.0 for
        match result {
            ConsensusResult::Rejected(_) => {}
            _ => panic!("Expected rejection due to expert weight"),
        }
    }

    #[test]
    fn test_insufficient_participation() {
        let mut engine = ConsensusEngine::new();
        
        let voters = vec![
            AgentId::from_string("agent1"),
            AgentId::from_string("agent2"),
            AgentId::from_string("agent3"),
            AgentId::from_string("agent4"),
        ];
        
        let proposal = Proposal::new(
            "Test",
            "Description",
            TradingDecision::NoTrade,
            Duration::from_secs(60),
            ConsensusThreshold::SimpleMajority,
        ).with_min_participation(0.75); // Need 3 out of 4
        
        let proposal_id = engine.create_proposal(proposal, voters.clone());
        
        // Only 2 vote (50%)
        engine.vote(&proposal_id, WeightedVote::new(
            voters[0].clone(), VoteChoice::For, 1.0, 0.9, "Yes"
        )).unwrap();
        
        engine.vote(&proposal_id, WeightedVote::new(
            voters[1].clone(), VoteChoice::For, 1.0, 0.9, "Yes"
        )).unwrap();
        
        let result = engine.finalize(&proposal_id);
        
        match result {
            ConsensusResult::InsufficientParticipation { participation_rate, .. } => {
                assert_eq!(participation_rate, 0.5);
            }
            _ => panic!("Expected insufficient participation"),
        }
    }

    #[test]
    fn test_tie_breaking() {
        let mut engine = ConsensusEngine::new();
        
        let voters = vec![
            AgentId::from_string("agent1"),
            AgentId::from_string("agent2"),
        ];
        
        let proposal = Proposal::new(
            "Tie Test",
            "Description",
            TradingDecision::Hold { symbol: "BTC".to_string() },
            Duration::from_secs(60),
            ConsensusThreshold::SimpleMajority,
        );
        
        let proposal_id = engine.create_proposal(proposal, voters.clone());
        
        // Perfect tie
        engine.vote(&proposal_id, WeightedVote::new(
            voters[0].clone(), VoteChoice::For, 1.0, 0.5, "Yes"
        )).unwrap();
        
        engine.vote(&proposal_id, WeightedVote::new(
            voters[1].clone(), VoteChoice::Against, 1.0, 0.5, "No"
        )).unwrap();
        
        let result = engine.finalize(&proposal_id);
        
        match result {
            ConsensusResult::Tie => {}
            _ => panic!("Expected tie"),
        }
    }
}
