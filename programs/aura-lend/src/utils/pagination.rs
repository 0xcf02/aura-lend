use crate::error::LendingError;
use anchor_lang::prelude::*;

/// Pagination parameters for querying large datasets
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PaginationParams {
    /// Page number (0-based)
    pub page: u32,
    /// Number of items per page (max 100)
    pub page_size: u32,
}

impl PaginationParams {
    /// Maximum items per page to prevent excessive memory usage
    pub const MAX_PAGE_SIZE: u32 = 100;
    /// Default page size if not specified
    pub const DEFAULT_PAGE_SIZE: u32 = 20;

    /// Create new pagination parameters with validation
    pub fn new(page: u32, page_size: u32) -> Result<Self> {
        if page_size == 0 {
            return Err(LendingError::InvalidAmount.into());
        }

        if page_size > Self::MAX_PAGE_SIZE {
            return Err(LendingError::AmountTooLarge.into());
        }

        Ok(Self { page, page_size })
    }

    /// Create default pagination (page 0, 20 items)
    pub fn default() -> Self {
        Self {
            page: 0,
            page_size: Self::DEFAULT_PAGE_SIZE,
        }
    }

    /// Calculate the starting index for this page
    pub fn start_index(&self) -> u32 {
        self.page * self.page_size
    }

    /// Calculate the ending index (exclusive) for this page
    pub fn end_index(&self) -> u32 {
        (self.page + 1) * self.page_size
    }

    /// Check if given index should be included in this page
    pub fn includes_index(&self, index: u32) -> bool {
        let start = self.start_index();
        let end = self.end_index();
        index >= start && index < end
    }
}

/// Pagination result with metadata
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PaginationResult {
    /// Current page number
    pub page: u32,
    /// Items per page
    pub page_size: u32,
    /// Total number of items across all pages
    pub total_items: u32,
    /// Total number of pages
    pub total_pages: u32,
    /// Whether there are more pages after this one
    pub has_next_page: bool,
    /// Whether there are pages before this one
    pub has_previous_page: bool,
}

impl PaginationResult {
    /// Create pagination result from parameters and total count
    pub fn new(params: &PaginationParams, total_items: u32) -> Self {
        let total_pages = if total_items == 0 {
            0
        } else {
            (total_items + params.page_size - 1) / params.page_size
        };

        let has_next_page = params.page + 1 < total_pages;
        let has_previous_page = params.page > 0;

        Self {
            page: params.page,
            page_size: params.page_size,
            total_items,
            total_pages,
            has_next_page,
            has_previous_page,
        }
    }
}

/// Helper trait for paginating vectors
pub trait Paginate<T> {
    /// Apply pagination to a vector
    fn paginate(&self, params: &PaginationParams) -> Vec<&T>;

    /// Get pagination result metadata
    fn pagination_result(&self, params: &PaginationParams) -> PaginationResult;
}

impl<T> Paginate<T> for Vec<T> {
    fn paginate(&self, params: &PaginationParams) -> Vec<&T> {
        let start = params.start_index() as usize;
        let end = std::cmp::min(params.end_index() as usize, self.len());

        if start >= self.len() {
            return vec![];
        }

        self[start..end].iter().collect()
    }

    fn pagination_result(&self, params: &PaginationParams) -> PaginationResult {
        PaginationResult::new(params, self.len() as u32)
    }
}

/// Paginated query for reserves
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ReserveQueryParams {
    /// Pagination parameters
    pub pagination: PaginationParams,
    /// Optional filter by liquidity mint
    pub liquidity_mint: Option<Pubkey>,
    /// Optional filter by minimum liquidity amount
    pub min_liquidity: Option<u64>,
    /// Sort by available liquidity (ascending if true)
    pub sort_by_liquidity: Option<bool>,
}

impl Default for ReserveQueryParams {
    fn default() -> Self {
        Self {
            pagination: PaginationParams::default(),
            liquidity_mint: None,
            min_liquidity: None,
            sort_by_liquidity: None,
        }
    }
}

/// Paginated query for obligations
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ObligationQueryParams {
    /// Pagination parameters
    pub pagination: PaginationParams,
    /// Optional filter by owner
    pub owner: Option<Pubkey>,
    /// Optional filter by health factor below threshold (for liquidation)
    pub max_health_factor: Option<u64>, // In basis points (10000 = 100%)
    /// Optional filter by minimum borrowed value
    pub min_borrowed_value: Option<u64>,
    /// Sort by health factor (ascending if true)
    pub sort_by_health: Option<bool>,
}

impl Default for ObligationQueryParams {
    fn default() -> Self {
        Self {
            pagination: PaginationParams::default(),
            owner: None,
            max_health_factor: None,
            min_borrowed_value: None,
            sort_by_health: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_params() {
        let params = PaginationParams::new(0, 20).unwrap();
        assert_eq!(params.start_index(), 0);
        assert_eq!(params.end_index(), 20);
        assert!(params.includes_index(10));
        assert!(!params.includes_index(25));

        let params2 = PaginationParams::new(2, 10).unwrap();
        assert_eq!(params2.start_index(), 20);
        assert_eq!(params2.end_index(), 30);
    }

    #[test]
    fn test_pagination_validation() {
        assert!(PaginationParams::new(0, 0).is_err());
        assert!(PaginationParams::new(0, 101).is_err());
        assert!(PaginationParams::new(0, 50).is_ok());
    }

    #[test]
    fn test_vector_pagination() {
        let data: Vec<u32> = (0..100).collect();
        let params = PaginationParams::new(2, 20).unwrap();

        let page_data = data.paginate(&params);
        assert_eq!(page_data.len(), 20);
        assert_eq!(*page_data[0], 40);
        assert_eq!(*page_data[19], 59);

        let result = data.pagination_result(&params);
        assert_eq!(result.total_items, 100);
        assert_eq!(result.total_pages, 5);
        assert!(result.has_next_page);
        assert!(result.has_previous_page);
    }
}
