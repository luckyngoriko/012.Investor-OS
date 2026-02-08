//! S5-GP-06: Embedding generation produces normalized vectors

use investor_os::rag::embeddings::{cosine_similarity, EmbeddingGenerator};

#[tokio::test]
async fn test_embedding_generation() {
    let generator = EmbeddingGenerator::new().await
        .expect("Should create embedding generator");

    let text = "Apple Inc. reported strong earnings this quarter.";
    let embedding = generator.generate(text).await
        .expect("Should generate embedding");

    // Should produce a vector of the correct dimension
    assert_eq!(embedding.len(), generator.dimension(), 
        "Embedding dimension should match generator dimension");
    
    // Should be normalized (magnitude ≈ 1.0)
    let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((magnitude - 1.0).abs() < 0.001, 
        "Embedding should be normalized, got magnitude {}", magnitude);
}

#[tokio::test]
async fn test_embedding_determinism() {
    let generator = EmbeddingGenerator::new().await
        .expect("Should create embedding generator");

    let text = "Test sentence for embedding generation.";
    
    let embedding1 = generator.generate(text).await
        .expect("Should generate embedding");
    let embedding2 = generator.generate(text).await
        .expect("Should generate embedding");

    // Same text should produce same embedding (with mock provider)
    assert_eq!(embedding1, embedding2, 
        "Same text should produce identical embeddings");
}

#[tokio::test]
async fn test_different_texts_different_embeddings() {
    let generator = EmbeddingGenerator::new().await
        .expect("Should create embedding generator");

    let text1 = "Apple is a technology company.";
    let text2 = "The weather is nice today.";

    let embedding1 = generator.generate(text1).await
        .expect("Should generate embedding");
    let embedding2 = generator.generate(text2).await
        .expect("Should generate embedding");

    // Different texts should produce different embeddings
    assert_ne!(embedding1, embedding2, 
        "Different texts should produce different embeddings");
}

#[tokio::test]
async fn test_cosine_similarity_range() {
    let generator = EmbeddingGenerator::new().await
        .expect("Should create embedding generator");

    let text1 = "The company revenue increased.";
    let text2 = "Revenue grew significantly.";

    let embedding1 = generator.generate(text1).await
        .expect("Should generate embedding");
    let embedding2 = generator.generate(text2).await
        .expect("Should generate embedding");

    let similarity = cosine_similarity(&embedding1, &embedding2);

    // Similarity should be in range [0, 1] (for positive vectors)
    // Actually for normalized vectors, range is [-1, 1]
    assert!(similarity >= -1.0 && similarity <= 1.0, 
        "Cosine similarity should be in range [-1, 1]");

    // Similar texts should have higher similarity
    assert!(similarity > 0.0, 
        "Similar texts should have positive cosine similarity");
}

#[tokio::test]
async fn test_batch_embedding_generation() {
    let generator = EmbeddingGenerator::new().await
        .expect("Should create embedding generator");

    let texts = vec![
        "First document about Apple.".to_string(),
        "Second document about Google.".to_string(),
        "Third document about Microsoft.".to_string(),
    ];

    let embeddings = generator.generate_batch(&texts).await
        .expect("Should generate batch embeddings");

    assert_eq!(embeddings.len(), texts.len(), 
        "Should generate one embedding per text");

    // All embeddings should have same dimension
    for embedding in &embeddings {
        assert_eq!(embedding.len(), generator.dimension());
    }
}

#[test]
fn test_cosine_similarity_identical() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![1.0, 0.0, 0.0];

    let similarity = cosine_similarity(&a, &b);
    assert!((similarity - 1.0).abs() < 0.001, 
        "Identical vectors should have similarity 1.0");
}

#[test]
fn test_cosine_similarity_orthogonal() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![0.0, 1.0, 0.0];

    let similarity = cosine_similarity(&a, &b);
    assert!(similarity.abs() < 0.001, 
        "Orthogonal vectors should have similarity 0.0");
}

#[test]
fn test_cosine_similarity_opposite() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![-1.0, 0.0, 0.0];

    let similarity = cosine_similarity(&a, &b);
    assert!((similarity - (-1.0)).abs() < 0.001, 
        "Opposite vectors should have similarity -1.0");
}
