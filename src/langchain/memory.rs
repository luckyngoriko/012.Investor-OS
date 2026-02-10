//! Memory systems за LangChain

use async_trait::async_trait;
use std::collections::VecDeque;

use super::prompts::Message;

/// Conversation memory trait
#[async_trait]
pub trait ConversationMemory: Send + Sync {
    /// Зарежда историята на разговора
    async fn load(&self) -> Vec<Message>;
    
    /// Запазва съобщение
    async fn save(&self, human: &str, ai: &str);
    
    /// Изчиства паметта
    async fn clear(&self);
}

/// Проста буфер памет (in-memory)
pub struct BufferMemory {
    max_messages: usize,
    messages: std::sync::Mutex<VecDeque<Message>>,
}

impl BufferMemory {
    pub fn new(max_messages: usize) -> Self {
        Self {
            max_messages: max_messages * 2, // human + ai за всяка двойка
            messages: std::sync::Mutex::new(VecDeque::new()),
        }
    }
}

#[async_trait]
impl ConversationMemory for BufferMemory {
    async fn load(&self) -> Vec<Message> {
        let messages = self.messages.lock().unwrap();
        messages.iter().cloned().collect()
    }
    
    async fn save(&self, human: &str, ai: &str) {
        let mut messages = self.messages.lock().unwrap();
        
        messages.push_back(Message::user(human));
        messages.push_back(Message::assistant(ai));
        
        // Премахваме старите съобщения
        while messages.len() > self.max_messages {
            messages.pop_front();
        }
    }
    
    async fn clear(&self) {
        let mut messages = self.messages.lock().unwrap();
        messages.clear();
    }
}

/// Vector store memory - използва RAG за retrieval
pub struct VectorStoreMemory {
    session_id: String,
    retriever: Arc<dyn VectorRetriever>,
    max_retrieved: usize,
}

#[async_trait]
pub trait VectorRetriever: Send + Sync {
    async fn add(&self, session_id: &str, message: &Message);
    async fn search(&self, session_id: &str, query: &str, limit: usize) -> Vec<Message>;
}

use std::sync::Arc;

impl VectorStoreMemory {
    pub fn new(
        session_id: impl Into<String>,
        retriever: Arc<dyn VectorRetriever>,
        max_retrieved: usize,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            retriever,
            max_retrieved,
        }
    }
}

#[async_trait]
impl ConversationMemory for VectorStoreMemory {
    async fn load(&self) -> Vec<Message> {
        // Връщаме последните релевантни съобщения
        // За сега просто връщаме празен вектор - трябва query за релевантност
        vec![]
    }
    
    async fn save(&self, human: &str, ai: &str) {
        self.retriever.add(&self.session_id, &Message::user(human)).await;
        self.retriever.add(&self.session_id, &Message::assistant(ai)).await;
    }
    
    async fn clear(&self) {
        // TODO: implement clear
    }
}

/// Summary memory - обобщава старата история
pub struct SummaryMemory {
    llm: super::chains::LLMChain, // За генериране на summary
    recent_messages: BufferMemory,
    summary: std::sync::Mutex<String>,
}

impl SummaryMemory {
    pub fn new(llm: super::chains::LLMChain, recent_to_keep: usize) -> Self {
        Self {
            llm,
            recent_messages: BufferMemory::new(recent_to_keep),
            summary: std::sync::Mutex::new(String::new()),
        }
    }
}

#[async_trait]
impl ConversationMemory for SummaryMemory {
    async fn load(&self) -> Vec<Message> {
        let summary = self.summary.lock().unwrap().clone();
        let recent = self.recent_messages.load().await;
        
        let mut messages = vec![];
        if !summary.is_empty() {
            messages.push(Message::system(format!(
                "Summary of previous conversation:\n{}",
                summary
            )));
        }
        messages.extend(recent);
        
        messages
    }
    
    async fn save(&self, human: &str, ai: &str) {
        self.recent_messages.save(human, ai).await;
        
        // TODO: Периодично обновяваме summary с LLM
    }
    
    async fn clear(&self) {
        self.recent_messages.clear().await;
        *self.summary.lock().unwrap() = String::new();
    }
}

/// Entity memory - помни конкретни ентитети
pub struct EntityMemory {
    entities: std::sync::Mutex<HashMap<String, EntityInfo>>,
}

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct EntityInfo {
    pub name: String,
    pub entity_type: String,
    pub observations: Vec<String>,
}

impl Default for EntityMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityMemory {
    pub fn new() -> Self {
        Self {
            entities: std::sync::Mutex::new(HashMap::new()),
        }
    }
    
    pub fn add_observation(&self, entity: &str, observation: &str) {
        let mut entities = self.entities.lock().unwrap();
        entities.entry(entity.to_string())
            .or_insert_with(|| EntityInfo {
                name: entity.to_string(),
                entity_type: "unknown".to_string(),
                observations: vec![],
            })
            .observations.push(observation.to_string());
    }
}

#[async_trait]
impl ConversationMemory for EntityMemory {
    async fn load(&self) -> Vec<Message> {
        let entities = self.entities.lock().unwrap();
        if entities.is_empty() {
            return vec![];
        }
        
        let context = entities.values()
            .map(|e| format!("{}: {}", e.name, e.observations.join("; ")))
            .collect::<Vec<_>>()
            .join("\n");
        
        vec![Message::system(format!(
            "Known entities:\n{}",
            context
        ))]
    }
    
    async fn save(&self, _human: &str, _ai: &str) {
        // Entity extraction би се случило тук с LLM
    }
    
    async fn clear(&self) {
        self.entities.lock().unwrap().clear();
    }
}

/// Комбинирана памет
pub struct CombinedMemory {
    memories: Vec<Box<dyn ConversationMemory>>,
}

impl CombinedMemory {
    pub fn new(memories: Vec<Box<dyn ConversationMemory>>) -> Self {
        Self { memories }
    }
}

#[async_trait]
impl ConversationMemory for CombinedMemory {
    async fn load(&self) -> Vec<Message> {
        let mut all_messages = vec![];
        for memory in &self.memories {
            all_messages.extend(memory.load().await);
        }
        all_messages
    }
    
    async fn save(&self, human: &str, ai: &str) {
        for memory in &self.memories {
            memory.save(human, ai).await;
        }
    }
    
    async fn clear(&self) {
        for memory in &self.memories {
            memory.clear().await;
        }
    }
}
