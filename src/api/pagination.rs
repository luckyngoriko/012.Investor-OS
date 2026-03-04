//! Cursor-based pagination (Sprint 103).
//!
//! All list endpoints accept `?cursor=<uuid>&limit=<n>` (max 100).

use serde::{Deserialize, Serialize};

/// Query parameters for paginated endpoints.
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    /// Cursor — opaque string (typically a UUID or timestamp).
    /// If `None`, returns from the beginning.
    pub cursor: Option<String>,
    /// Number of items per page (clamped to 1..=100).
    pub limit: Option<u32>,
}

impl PaginationParams {
    /// Effective limit, clamped to [1, 100]. Defaults to 20.
    pub fn effective_limit(&self) -> u32 {
        self.limit.unwrap_or(20).clamp(1, 100)
    }
}

/// Paginated response wrapper.
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub success: bool,
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
}

#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    /// Cursor to fetch the next page. `None` when there are no more items.
    pub next_cursor: Option<String>,
    /// Number of items returned in this page.
    pub count: usize,
    /// The limit that was applied.
    pub limit: u32,
}

impl<T: Serialize> PaginatedResponse<T> {
    /// Build a paginated response from a data vec.
    /// If `data.len() > limit`, the last element is the cursor marker.
    pub fn from_vec(mut data: Vec<T>, limit: u32, cursor_fn: impl Fn(&T) -> String) -> Self {
        let has_next = data.len() > limit as usize;
        if has_next {
            data.truncate(limit as usize);
        }
        let next_cursor = if has_next {
            data.last().map(&cursor_fn)
        } else {
            None
        };
        let count = data.len();
        Self {
            success: true,
            data,
            pagination: PaginationMeta {
                next_cursor,
                count,
                limit,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_limit_defaults_to_20() {
        let p = PaginationParams {
            cursor: None,
            limit: None,
        };
        assert_eq!(p.effective_limit(), 20);
    }

    #[test]
    fn effective_limit_clamps_to_100() {
        let p = PaginationParams {
            cursor: None,
            limit: Some(500),
        };
        assert_eq!(p.effective_limit(), 100);
    }

    #[test]
    fn effective_limit_clamps_to_1() {
        let p = PaginationParams {
            cursor: None,
            limit: Some(0),
        };
        assert_eq!(p.effective_limit(), 1);
    }

    #[test]
    fn paginated_response_no_next_cursor_when_under_limit() {
        let data = vec!["a".to_string(), "b".to_string()];
        let resp = PaginatedResponse::from_vec(data, 10, |s| s.clone());
        assert!(resp.pagination.next_cursor.is_none());
        assert_eq!(resp.pagination.count, 2);
    }

    #[test]
    fn paginated_response_has_next_cursor_when_over_limit() {
        // Simulate: limit=2, but we fetched 3 (one extra to detect next page)
        let data = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let resp = PaginatedResponse::from_vec(data, 2, |s| s.clone());
        assert_eq!(resp.pagination.next_cursor, Some("b".to_string()));
        assert_eq!(resp.pagination.count, 2);
        assert_eq!(resp.data.len(), 2);
    }
}
