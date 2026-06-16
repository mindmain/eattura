//! Shared query parameters for list endpoints (pagination + filtering).

/// Common list query parameters: `?page=&per_page=&status=&search=`.
#[derive(Debug, serde::Deserialize, Default)]
pub struct Pagination {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub status: Option<String>,
    pub search: Option<String>,
}

impl Pagination {
    /// Page size, defaulting to 20 and capped at 100.
    pub fn limit(&self) -> i64 {
        self.per_page.unwrap_or(20).clamp(1, 100)
    }

    /// Row offset derived from the 1-based page number.
    pub fn offset(&self) -> i64 {
        (self.page.unwrap_or(1).max(1) - 1) * self.limit()
    }
}

/// Build a `%escaped%` pattern for a case-insensitive LIKE/ILIKE match,
/// escaping the `%` and `_` wildcards in user input.
pub fn like_pattern(input: &str) -> String {
    format!("%{}%", input.replace('%', "\\%").replace('_', "\\_"))
}
