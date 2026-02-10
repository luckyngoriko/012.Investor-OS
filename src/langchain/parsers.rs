//! Output parsers - структуриран output от LLM

use serde::de::DeserializeOwned;
use std::marker::PhantomData;

/// Trait за парсиране на LLM output
pub trait OutputParser: Send + Sync {
    fn parse(&self, output: &str) -> Result<serde_json::Value, super::ChainError>;
}

/// JSON parser - опитва да извлече JSON от output
pub struct JsonParser<T: DeserializeOwned> {
    _phantom: PhantomData<T>,
}

impl<T: DeserializeOwned> JsonParser<T> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<T: DeserializeOwned + Send + Sync> OutputParser for JsonParser<T> {
    fn parse(&self, output: &str) -> Result<serde_json::Value, super::ChainError> {
        // Търсим JSON блок в output (може да е обграден с ```json ... ```)
        let json_str = extract_json(output)
            .ok_or_else(|| super::ChainError::ParseError(
                format!("No JSON found in output: {}", output)
            ))?;
        
        let value: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| super::ChainError::ParseError(
                format!("JSON parse error: {}", e)
            ))?;
        
        // Валидираме че може да се десериализира до T
        let _: T = serde_json::from_value(value.clone())
            .map_err(|e| super::ChainError::ParseError(
                format!("Schema validation error: {}", e)
            ))?;
        
        Ok(value)
    }
}

impl<T: DeserializeOwned> Default for JsonParser<T> {
    fn default() -> Self {
        Self::new()
    }
}

fn extract_json(output: &str) -> Option<&str> {
    // Търсим ```json ... ```
    if let Some(start) = output.find("```json") {
        let start = start + 7;
        if let Some(end) = output[start..].find("```") {
            return Some(output[start..start + end].trim());
        }
    }
    
    // Търсим ``` ... ```
    if let Some(start) = output.find("```") {
        let start = start + 3;
        if let Some(end) = output[start..].find("```") {
            let candidate = output[start..start + end].trim();
            // Проверяваме дали започва с { или [
            if candidate.starts_with('{') || candidate.starts_with('[') {
                return Some(candidate);
            }
        }
    }
    
    // Търсим { ... } или [ ... ]
    if let Some(start) = output.find('{') {
        // Намираме съответстващата }
        let mut brace_count = 0;
        for (i, ch) in output[start..].chars().enumerate() {
            if ch == '{' {
                brace_count += 1;
            } else if ch == '}' {
                brace_count -= 1;
                if brace_count == 0 {
                    return Some(&output[start..start + i + 1]);
                }
            }
        }
    }
    
    None
}

/// Structured parser с JSON Schema валидация
pub struct StructuredParser {
    schema: serde_json::Value,
}

impl StructuredParser {
    pub fn new(schema: serde_json::Value) -> Self {
        Self { schema }
    }
}

impl OutputParser for StructuredParser {
    fn parse(&self, output: &str) -> Result<serde_json::Value, super::ChainError> {
        let json_str = extract_json(output)
            .ok_or_else(|| super::ChainError::ParseError(
                "No JSON found".to_string()
            ))?;
        
        let value: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| super::ChainError::ParseError(format!("JSON error: {}", e)))?;
        
        // TODO: JSON Schema валидация
        
        Ok(value)
    }
}

/// List parser - парсира списък от елементи
pub struct ListParser {
    separator: String,
}

impl ListParser {
    pub fn new() -> Self {
        Self {
            separator: "\n".to_string(),
        }
    }
    
    pub fn with_separator(mut self, sep: impl Into<String>) -> Self {
        self.separator = sep.into();
        self
    }
}

impl OutputParser for ListParser {
    fn parse(&self, output: &str) -> Result<serde_json::Value, super::ChainError> {
        let items: Vec<String> = output
            .split(&self.separator)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        Ok(serde_json::json!(items))
    }
}

impl Default for ListParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Comma-separated values parser
pub struct CsvParser {
    headers: Vec<String>,
}

impl CsvParser {
    pub fn new(headers: Vec<String>) -> Self {
        Self { headers }
    }
}

impl OutputParser for CsvParser {
    fn parse(&self, output: &str) -> Result<serde_json::Value, super::ChainError> {
        let lines: Vec<&str> = output.lines().collect();
        if lines.is_empty() {
            return Ok(serde_json::json!([]));
        }
        
        let mut result = vec![];
        
        for line in lines {
            let values: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            let mut obj = serde_json::Map::new();
            
            for (i, header) in self.headers.iter().enumerate() {
                let value = values.get(i).unwrap_or(&"");
                obj.insert(header.clone(), serde_json::json!(value));
            }
            
            result.push(serde_json::Value::Object(obj));
        }
        
        Ok(serde_json::json!(result))
    }
}

/// Boolean parser - yes/no, true/false
pub struct BooleanParser;

impl OutputParser for BooleanParser {
    fn parse(&self, output: &str) -> Result<serde_json::Value, super::ChainError> {
        let trimmed = output.trim().to_lowercase();
        let value = match trimmed.as_str() {
            "yes" | "true" | "1" | "y" => true,
            "no" | "false" | "0" | "n" => false,
            _ => {
                return Err(super::ChainError::ParseError(
                    format!("Cannot parse '{}' as boolean", output)
                ));
            }
        };
        
        Ok(serde_json::json!(value))
    }
}

/// Score parser - 0-1 или 0-100
pub struct ScoreParser;

impl OutputParser for ScoreParser {
    fn parse(&self, output: &str) -> Result<serde_json::Value, super::ChainError> {
        let trimmed = output.trim();
        
        // Опитваме да парснем като число
        if let Ok(num) = trimmed.parse::<f64>() {
            // Нормализираме към 0-1
            let normalized = if num > 1.0 {
                num / 100.0
            } else {
                num
            };
            return Ok(serde_json::json!(normalized.clamp(0.0, 1.0)));
        }
        
        // Опитваме да извлечем число от текста
        let re = regex::Regex::new(r"(\d+\.?\d*)").unwrap();
        if let Some(caps) = re.captures(trimmed) {
            if let Some(m) = caps.get(1) {
                if let Ok(num) = m.as_str().parse::<f64>() {
                    let normalized = if num > 1.0 { num / 100.0 } else { num };
                    return Ok(serde_json::json!(normalized.clamp(0.0, 1.0)));
                }
            }
        }
        
        Err(super::ChainError::ParseError(
            format!("Cannot parse '{}' as score", output)
        ))
    }
}

/// Auto-fix parser - опитва няколко стратегии
pub struct AutoFixParser<T: DeserializeOwned> {
    _phantom: PhantomData<T>,
}

impl<T: DeserializeOwned> Default for AutoFixParser<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: DeserializeOwned> AutoFixParser<T> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<T: DeserializeOwned + Send + Sync> OutputParser for AutoFixParser<T> {
    fn parse(&self, output: &str) -> Result<serde_json::Value, super::ChainError> {
        // Стратегия 1: Директен JSON
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(output) {
            if serde_json::from_value::<T>(v.clone()).is_ok() {
                return Ok(v);
            }
        }
        
        // Стратегия 2: Extract JSON block
        if let Some(json_str) = extract_json(output) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
                if serde_json::from_value::<T>(v.clone()).is_ok() {
                    return Ok(v);
                }
            }
        }
        
        // Стратегия 3: Fix common issues (trailing commas, etc.)
        let fixed = fix_common_json_issues(output);
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&fixed) {
            if serde_json::from_value::<T>(v.clone()).is_ok() {
                return Ok(v);
            }
        }
        
        Err(super::ChainError::ParseError(
            "All parsing strategies failed".to_string()
        ))
    }
}

fn fix_common_json_issues(input: &str) -> String {
    let mut result = input.to_string();
    
    // Премахваме trailing commas
    result = result.replace(",}", "}");
    result = result.replace(",]", "]");
    
    // Добавяме quotes около ключове ако липсват
    // (simplified)
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    
    #[derive(Debug, Deserialize)]
    struct TestData {
        name: String,
        score: f64,
    }
    
    #[test]
    fn test_extract_json() {
        let text = r#"Some text
```json
{"name": "test", "score": 0.95}
```
More text"#;
        
        let json = extract_json(text).unwrap();
        assert!(json.contains("name"));
    }
    
    #[test]
    fn test_json_parser() {
        let parser = JsonParser::<TestData>::new();
        let output = r#"{"name": "AAPL", "score": 0.85}"#;
        
        let result = parser.parse(output).unwrap();
        assert_eq!(result["name"], "AAPL");
    }
    
    #[test]
    fn test_score_parser() {
        let parser = ScoreParser;
        
        assert_eq!(parser.parse("0.85").unwrap(), 0.85);
        assert_eq!(parser.parse("85").unwrap(), 0.85);
        assert_eq!(parser.parse("Score: 92").unwrap(), 0.92);
    }
}
