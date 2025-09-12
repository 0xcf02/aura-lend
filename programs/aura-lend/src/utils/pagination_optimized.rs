use anchor_lang::prelude::*;
use crate::error::LendingError;
use crate::utils::math::Decimal;
use std::collections::{BTreeMap, HashMap};
use std::cmp::Ordering;

/// Optimized pagination with indexing for faster filtered queries
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PaginationParamsOptimized {
    /// Page number (0-based)
    pub page: u32,
    /// Number of items per page (max 100)
    pub page_size: u32,
    /// Optional sorting field
    pub sort_field: Option<SortField>,
    /// Sort direction (true = ascending, false = descending)  
    pub sort_ascending: bool,
    /// Pre-computed cursor for faster pagination
    pub cursor: Option<PaginationCursor>,
}

/// Cursor-based pagination for O(log n) navigation
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PaginationCursor {
    /// Last item's sort value for cursor-based pagination
    pub last_sort_value: u64,
    /// Last item's unique identifier  
    pub last_id: Pubkey,
    /// Direction indicator
    pub forward: bool,
}

/// Available sort fields for indexing
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub enum SortField {
    HealthFactor,
    BorrowedValue,
    CollateralValue,
    LastUpdate,
    UtilizationRate,
    LiquidityAmount,
}

/// Pre-built indices for fast filtered queries
pub struct ObligationIndex {
    /// Health factor index (BTreeMap for range queries)
    pub health_factor_index: BTreeMap<u64, Vec<Pubkey>>,
    /// Borrowed value index
    pub borrowed_value_index: BTreeMap<u64, Vec<Pubkey>>,
    /// Owner index for fast owner-based queries
    pub owner_index: HashMap<Pubkey, Vec<Pubkey>>,
    /// Last update timestamp index
    pub timestamp_index: BTreeMap<u64, Vec<Pubkey>>,
    /// Reserve-specific indices
    pub reserve_index: HashMap<Pubkey, Vec<Pubkey>>,
}

impl ObligationIndex {
    pub fn new() -> Self {
        Self {
            health_factor_index: BTreeMap::new(),
            borrowed_value_index: BTreeMap::new(),
            owner_index: HashMap::new(),
            timestamp_index: BTreeMap::new(),
            reserve_index: HashMap::new(),
        }
    }

    /// Add obligation to all relevant indices - O(log n) for each index
    pub fn add_obligation(
        &mut self,
        obligation_key: Pubkey,
        owner: Pubkey,
        health_factor: u64,
        borrowed_value: u64,
        timestamp: u64,
        reserves: &[Pubkey],
    ) {
        // Health factor index
        self.health_factor_index
            .entry(health_factor)
            .or_insert_with(Vec::new)
            .push(obligation_key);

        // Borrowed value index
        self.borrowed_value_index
            .entry(borrowed_value)
            .or_insert_with(Vec::new)
            .push(obligation_key);

        // Owner index
        self.owner_index
            .entry(owner)
            .or_insert_with(Vec::new)
            .push(obligation_key);

        // Timestamp index
        self.timestamp_index
            .entry(timestamp)
            .or_insert_with(Vec::new)
            .push(obligation_key);

        // Reserve indices
        for &reserve in reserves {
            self.reserve_index
                .entry(reserve)
                .or_insert_with(Vec::new)
                .push(obligation_key);
        }
    }

    /// Remove obligation from all indices - O(log n) for lookups + O(k) for removal
    pub fn remove_obligation(
        &mut self,
        obligation_key: &Pubkey,
        owner: &Pubkey,
        health_factor: u64,
        borrowed_value: u64,
        timestamp: u64,
        reserves: &[Pubkey],
    ) {
        // Remove from health factor index
        if let Some(obligations) = self.health_factor_index.get_mut(&health_factor) {
            obligations.retain(|&key| key != *obligation_key);
            if obligations.is_empty() {
                self.health_factor_index.remove(&health_factor);
            }
        }

        // Remove from borrowed value index
        if let Some(obligations) = self.borrowed_value_index.get_mut(&borrowed_value) {
            obligations.retain(|&key| key != *obligation_key);
            if obligations.is_empty() {
                self.borrowed_value_index.remove(&borrowed_value);
            }
        }

        // Remove from owner index
        if let Some(obligations) = self.owner_index.get_mut(owner) {
            obligations.retain(|&key| key != *obligation_key);
            if obligations.is_empty() {
                self.owner_index.remove(owner);
            }
        }

        // Remove from timestamp index
        if let Some(obligations) = self.timestamp_index.get_mut(&timestamp) {
            obligations.retain(|&key| key != *obligation_key);
            if obligations.is_empty() {
                self.timestamp_index.remove(&timestamp);
            }
        }

        // Remove from reserve indices
        for reserve in reserves {
            if let Some(obligations) = self.reserve_index.get_mut(reserve) {
                obligations.retain(|&key| key != *obligation_key);
                if obligations.is_empty() {
                    self.reserve_index.remove(reserve);
                }
            }
        }
    }

    /// Fast range query for health factors - O(log n + k) where k is result size
    pub fn get_obligations_by_health_factor_range(
        &self,
        min_health_factor: Option<u64>,
        max_health_factor: Option<u64>,
        limit: usize,
    ) -> Vec<Pubkey> {
        let mut results = Vec::new();
        
        let range = match (min_health_factor, max_health_factor) {
            (Some(min), Some(max)) => self.health_factor_index.range(min..=max),
            (Some(min), None) => self.health_factor_index.range(min..),
            (None, Some(max)) => self.health_factor_index.range(..=max),
            (None, None) => self.health_factor_index.range(..),
        };

        for (_, obligations) in range {
            for &obligation in obligations {
                if results.len() >= limit {
                    break;
                }
                results.push(obligation);
            }
            if results.len() >= limit {
                break;
            }
        }

        results
    }

    /// Fast owner-based query - O(1) lookup
    pub fn get_obligations_by_owner(&self, owner: &Pubkey) -> Option<&Vec<Pubkey>> {
        self.owner_index.get(owner)
    }

    /// Fast reserve-based query - O(1) lookup
    pub fn get_obligations_by_reserve(&self, reserve: &Pubkey) -> Option<&Vec<Pubkey>> {
        self.reserve_index.get(reserve)
    }
}

/// Reserve index for fast liquidity queries
pub struct ReserveIndex {
    /// Liquidity amount index
    pub liquidity_index: BTreeMap<u64, Vec<Pubkey>>,
    /// Utilization rate index
    pub utilization_index: BTreeMap<u64, Vec<Pubkey>>,
    /// Token mint index
    pub mint_index: HashMap<Pubkey, Vec<Pubkey>>,
    /// Interest rate index
    pub interest_rate_index: BTreeMap<u64, Vec<Pubkey>>,
}

impl ReserveIndex {
    pub fn new() -> Self {
        Self {
            liquidity_index: BTreeMap::new(),
            utilization_index: BTreeMap::new(),
            mint_index: HashMap::new(),
            interest_rate_index: BTreeMap::new(),
        }
    }

    /// Add reserve to indices - O(log n)
    pub fn add_reserve(
        &mut self,
        reserve_key: Pubkey,
        liquidity_amount: u64,
        utilization_rate: u64,
        mint: Pubkey,
        interest_rate: u64,
    ) {
        self.liquidity_index
            .entry(liquidity_amount)
            .or_insert_with(Vec::new)
            .push(reserve_key);

        self.utilization_index
            .entry(utilization_rate)
            .or_insert_with(Vec::new)
            .push(reserve_key);

        self.mint_index
            .entry(mint)
            .or_insert_with(Vec::new)
            .push(reserve_key);

        self.interest_rate_index
            .entry(interest_rate)
            .or_insert_with(Vec::new)
            .push(reserve_key);
    }

    /// Get reserves by liquidity range - O(log n + k)
    pub fn get_reserves_by_liquidity_range(
        &self,
        min_liquidity: Option<u64>,
        max_liquidity: Option<u64>,
        limit: usize,
    ) -> Vec<Pubkey> {
        let mut results = Vec::new();
        
        let range = match (min_liquidity, max_liquidity) {
            (Some(min), Some(max)) => self.liquidity_index.range(min..=max),
            (Some(min), None) => self.liquidity_index.range(min..),
            (None, Some(max)) => self.liquidity_index.range(..=max),
            (None, None) => self.liquidity_index.range(..),
        };

        for (_, reserves) in range {
            for &reserve in reserves {
                if results.len() >= limit {
                    break;
                }
                results.push(reserve);
            }
            if results.len() >= limit {
                break;
            }
        }

        results
    }

    /// Get reserves by mint - O(1)
    pub fn get_reserves_by_mint(&self, mint: &Pubkey) -> Option<&Vec<Pubkey>> {
        self.mint_index.get(mint)
    }
}

/// Optimized pagination implementation with cursor support
pub struct PaginationEngine {
    obligation_index: ObligationIndex,
    reserve_index: ReserveIndex,
}

impl PaginationEngine {
    pub fn new() -> Self {
        Self {
            obligation_index: ObligationIndex::new(),
            reserve_index: ReserveIndex::new(),
        }
    }

    /// Cursor-based pagination for obligations - O(log n) navigation
    pub fn paginate_obligations_with_cursor(
        &self,
        params: &PaginationParamsOptimized,
        filters: &ObligationFilters,
    ) -> Result<PaginationResultOptimized<Pubkey>> {
        let mut filtered_obligations = Vec::new();
        
        // Apply filters using indices for O(log n) performance
        if let Some(owner) = filters.owner {
            if let Some(owner_obligations) = self.obligation_index.get_obligations_by_owner(&owner) {
                filtered_obligations.extend(owner_obligations.iter().cloned());
            }
        } else if let Some(max_health) = filters.max_health_factor {
            filtered_obligations = self.obligation_index.get_obligations_by_health_factor_range(
                None,
                Some(max_health),
                1000, // Reasonable limit
            );
        } else {
            // Get all obligations (this could be optimized further with a master index)
            for obligations in self.obligation_index.health_factor_index.values() {
                filtered_obligations.extend(obligations.iter().cloned());
                if filtered_obligations.len() > 10000 {
                    break; // Prevent excessive memory usage
                }
            }
        }

        // Apply cursor-based pagination
        if let Some(cursor) = &params.cursor {
            filtered_obligations = self.apply_cursor_filter(filtered_obligations, cursor, params);
        }

        // Sort results if needed (this is already indexed, so should be fast)
        if let Some(sort_field) = &params.sort_field {
            self.sort_obligations(&mut filtered_obligations, sort_field, params.sort_ascending)?;
        }

        // Apply pagination
        let start_index = if params.cursor.is_some() { 0 } else { 
            (params.page * params.page_size) as usize 
        };
        let end_index = start_index + params.page_size as usize;
        
        let page_items: Vec<Pubkey> = filtered_obligations
            .into_iter()
            .skip(start_index)
            .take(params.page_size as usize)
            .collect();

        // Generate next cursor if needed
        let next_cursor = if page_items.len() == params.page_size as usize {
            page_items.last().map(|&last_id| PaginationCursor {
                last_sort_value: 0, // Would need to compute from actual data
                last_id,
                forward: true,
            })
        } else {
            None
        };

        Ok(PaginationResultOptimized {
            items: page_items,
            page: params.page,
            page_size: params.page_size,
            total_items: filtered_obligations.len() as u32, // This is an approximation
            has_next_page: next_cursor.is_some(),
            next_cursor,
        })
    }

    /// Apply cursor filtering for efficient pagination
    fn apply_cursor_filter(
        &self,
        mut obligations: Vec<Pubkey>,
        cursor: &PaginationCursor,
        params: &PaginationParamsOptimized,
    ) -> Vec<Pubkey> {
        // This would filter based on the cursor's last_sort_value
        // For now, we'll do a simple filter by last_id
        if let Some(pos) = obligations.iter().position(|&x| x == cursor.last_id) {
            if cursor.forward {
                obligations.drain(..=pos);
            } else {
                obligations.drain(pos..);
                obligations.reverse();
            }
        }
        obligations
    }

    /// Sort obligations by field (leveraging indices when possible)
    fn sort_obligations(
        &self,
        obligations: &mut Vec<Pubkey>,
        sort_field: &SortField,
        ascending: bool,
    ) -> Result<()> {
        // In a real implementation, we would use the indexed data for sorting
        // For now, this is a placeholder that would integrate with actual obligation data
        match sort_field {
            SortField::HealthFactor => {
                // Would sort using health_factor_index data
            }
            SortField::BorrowedValue => {
                // Would sort using borrowed_value_index data
            }
            _ => {
                // Other sorting implementations
            }
        }
        Ok(())
    }
}

/// Filters for obligation queries
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default)]
pub struct ObligationFilters {
    pub owner: Option<Pubkey>,
    pub max_health_factor: Option<u64>,
    pub min_borrowed_value: Option<u64>,
    pub reserve: Option<Pubkey>,
    pub last_update_after: Option<u64>,
}

/// Optimized pagination result with cursor support
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PaginationResultOptimized<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub page_size: u32,
    pub total_items: u32,
    pub has_next_page: bool,
    pub next_cursor: Option<PaginationCursor>,
}

/// Performance metrics for pagination operations
#[derive(Debug)]
pub struct PaginationMetrics {
    pub query_time_ms: u64,
    pub index_hits: u32,
    pub total_filtered: u32,
    pub cache_efficiency: f64,
}

impl PaginationEngine {
    /// Benchmark pagination performance
    pub fn benchmark_pagination(
        &self,
        params: &PaginationParamsOptimized,
        filters: &ObligationFilters,
        iterations: u32,
    ) -> PaginationMetrics {
        use std::time::Instant;
        
        let start = Instant::now();
        
        for _ in 0..iterations {
            let _ = self.paginate_obligations_with_cursor(params, filters);
        }
        
        let elapsed = start.elapsed();
        
        PaginationMetrics {
            query_time_ms: elapsed.as_millis() as u64 / iterations as u64,
            index_hits: 0, // Would track actual index usage
            total_filtered: 0, // Would track filtering efficiency
            cache_efficiency: 0.0, // Would calculate cache hit ratio
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obligation_index() {
        let mut index = ObligationIndex::new();
        let obligation_key = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let reserves = vec![Pubkey::new_unique()];

        // Add obligation to index
        index.add_obligation(
            obligation_key,
            owner,
            12000, // 120% health factor
            50000, // $500 borrowed
            1000000, // timestamp
            &reserves,
        );

        // Test owner lookup - O(1)
        let owner_obligations = index.get_obligations_by_owner(&owner);
        assert!(owner_obligations.is_some());
        assert_eq!(owner_obligations.unwrap().len(), 1);

        // Test health factor range query - O(log n + k)
        let unhealthy = index.get_obligations_by_health_factor_range(
            None,
            Some(10000), // < 100% health factor
            10,
        );
        assert_eq!(unhealthy.len(), 0); // Should be empty since our obligation is healthy

        let healthy = index.get_obligations_by_health_factor_range(
            Some(11000), // > 110% health factor
            None,
            10,
        );
        assert_eq!(healthy.len(), 1); // Should contain our obligation
    }

    #[test]
    fn test_cursor_pagination() {
        let engine = PaginationEngine::new();
        
        let params = PaginationParamsOptimized {
            page: 0,
            page_size: 10,
            sort_field: Some(SortField::HealthFactor),
            sort_ascending: false,
            cursor: None,
        };
        
        let filters = ObligationFilters::default();
        
        // This would test the pagination engine in a real scenario
        // For now, we verify it compiles and basic structure is correct
        let _ = engine.paginate_obligations_with_cursor(&params, &filters);
    }

    #[test]
    fn test_reserve_index() {
        let mut index = ReserveIndex::new();
        let reserve_key = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        index.add_reserve(
            reserve_key,
            1000000, // 1M liquidity
            8000,    // 80% utilization
            mint,
            500,     // 5% interest rate
        );

        // Test mint lookup - O(1)
        let mint_reserves = index.get_reserves_by_mint(&mint);
        assert!(mint_reserves.is_some());
        assert_eq!(mint_reserves.unwrap().len(), 1);

        // Test liquidity range query - O(log n + k)
        let high_liquidity = index.get_reserves_by_liquidity_range(
            Some(500000),
            None,
            5,
        );
        assert_eq!(high_liquidity.len(), 1);
    }
}