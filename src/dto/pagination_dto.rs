use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_page() -> i64 {
    1
}

fn default_limit() -> i64 {
    10
}

impl PaginationParams {
    pub fn offset(&self) -> i64 {
        (self.page - 1) * self.limit
    }

    pub fn validate(&mut self) {
        if self.page < 1 {
            self.page = 1;
        }
        if self.limit < 1 {
            self.limit = 10;
        }
        if self.limit > 100 {
            self.limit = 100;
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub page: i64,
    pub limit: i64,
    pub total: i64,
    pub total_pages: i64,
    pub has_next: bool,
    pub has_prev: bool,
}

impl PaginationMeta {
    pub fn new(page: i64, limit: i64, total: i64) -> Self {
        let total_pages = (total as f64 / limit as f64).ceil() as i64;
        let has_next = page < total_pages;
        let has_prev = page > 1;

        Self {
            page,
            limit,
            total,
            total_pages,
            has_next,
            has_prev,
        }
    }
}
