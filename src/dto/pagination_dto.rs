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
        self.page.saturating_sub(1).saturating_mul(self.limit)
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

#[cfg(test)]
mod tests {
    use super::PaginationParams;

    #[test]
    fn offset_never_overflows_for_untrusted_query_values() {
        let params = PaginationParams {
            page: i64::MAX,
            limit: 100,
        };

        assert_eq!(params.offset(), i64::MAX);
    }

    #[test]
    fn validation_normalizes_invalid_page_bounds() {
        let mut params = PaginationParams { page: 0, limit: 101 };
        params.validate();

        assert_eq!(params.page, 1);
        assert_eq!(params.limit, 100);
    }
}

#[derive(Debug, Serialize, Deserialize)]
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
