use anchor_lang::prelude::*;
use crate::error::LendingError;
use crate::utils::math::Decimal;
use std::mem::{size_of, align_of};

/// Memory-optimized data structures with cache-friendly layouts
/// Focuses on minimizing cache misses and improving memory access patterns

/// Cache-aligned obligation structure for better memory access
#[repr(C, align(64))] // Align to CPU cache line size
#[account]
pub struct ObligationCacheOptimized {
    // Hot data - frequently accessed together (cache line 1)
    pub version: u8,
    pub market: Pubkey,
    pub owner: Pubkey,
    pub last_update_slot: u64,
    
    // Financial data - grouped for cache efficiency (cache line 2)
    pub deposited_value_usd: Decimal,      // 16 bytes
    pub borrowed_value_usd: Decimal,       // 16 bytes  
    pub liquidation_snapshot_health_factor: Option<Decimal>, // 16 bytes + 1 tag
    pub last_update_timestamp: u64,       // 8 bytes
    // Total: ~57 bytes, fits in cache line
    
    // Cold data - less frequently accessed
    pub deposit_count: u8,
    pub borrow_count: u8,
    
    // Variable length data stored separately to avoid fragmentation
    pub deposits_ptr: u64,      // Pointer to deposits array
    pub borrows_ptr: u64,       // Pointer to borrows array
    
    // Performance metrics grouped together
    pub lookup_count: u32,
    pub cache_hits: u32,
    pub last_health_calculation: u64,
    
    // Reserved space aligned to cache boundary
    pub reserved: [u8; 32],
}

/// Memory pool for efficient allocation of similar objects
pub struct MemoryPool<T> {
    /// Pre-allocated chunks of memory
    chunks: Vec<Box<[T]>>,
    /// Free list for O(1) allocation/deallocation
    free_list: Vec<usize>,
    /// Chunk size for cache efficiency
    chunk_size: usize,
    /// Current allocation statistics
    stats: PoolStats,
}

#[derive(Debug, Default)]
pub struct PoolStats {
    pub allocations: u64,
    pub deallocations: u64,
    pub cache_misses: u64,
    pub fragmentation_ratio: f64,
}

impl<T: Default + Clone> MemoryPool<T> {
    /// Create new memory pool with specified chunk size
    /// Chunk size should be chosen based on cache line size and usage patterns
    pub fn new(chunk_size: usize) -> Self {
        let initial_chunk = vec![T::default(); chunk_size].into_boxed_slice();
        let free_list: Vec<usize> = (0..chunk_size).collect();
        
        Self {
            chunks: vec![initial_chunk],
            free_list,
            chunk_size,
            stats: PoolStats::default(),
        }
    }

    /// Allocate object with O(1) complexity
    pub fn allocate(&mut self) -> Result<(usize, &mut T)> {
        if let Some(index) = self.free_list.pop() {
            self.stats.allocations += 1;
            let chunk_id = index / self.chunk_size;
            let item_id = index % self.chunk_size;
            
            if let Some(chunk) = self.chunks.get_mut(chunk_id) {
                return Ok((index, &mut chunk[item_id]));
            }
        }
        
        // Allocate new chunk if needed
        self.allocate_new_chunk()
    }

    /// Deallocate object with O(1) complexity
    pub fn deallocate(&mut self, index: usize) {
        self.free_list.push(index);
        self.stats.deallocations += 1;
    }

    /// Allocate new chunk when pool is exhausted
    fn allocate_new_chunk(&mut self) -> Result<(usize, &mut T)> {
        let new_chunk = vec![T::default(); self.chunk_size].into_boxed_slice();
        let chunk_id = self.chunks.len();
        self.chunks.push(new_chunk);
        
        // Add new free indices
        let start_index = chunk_id * self.chunk_size;
        for i in (start_index + 1)..(start_index + self.chunk_size) {
            self.free_list.push(i);
        }
        
        self.stats.allocations += 1;
        let chunk = self.chunks.get_mut(chunk_id).unwrap();
        Ok((start_index, &mut chunk[0]))
    }

    /// Get pool statistics
    pub fn get_stats(&self) -> &PoolStats {
        &self.stats
    }

    /// Compact memory by defragmenting free space
    pub fn compact(&mut self) {
        // Sort free list to improve locality
        self.free_list.sort();
        
        // Calculate fragmentation ratio
        let total_slots = self.chunks.len() * self.chunk_size;
        let free_slots = self.free_list.len();
        self.stats.fragmentation_ratio = (free_slots as f64) / (total_slots as f64);
    }
}

/// Structure-of-Arrays layout for better cache utilization
/// Instead of Array-of-Structures, use separate arrays for each field
pub struct CollateralArrays {
    /// Separate arrays for each field - better for vectorized operations
    pub reserve_keys: Vec<Pubkey>,
    pub deposited_amounts: Vec<u64>,
    pub market_values_usd: Vec<u64>, // Stored as scaled integers for better packing
    pub liquidation_thresholds: Vec<u16>, // Basis points fit in u16
    pub loan_to_value_ratios: Vec<u16>,   // Basis points fit in u16
    
    /// Index mapping for O(1) lookup by reserve key
    pub reserve_to_index: std::collections::HashMap<Pubkey, usize>,
    
    /// Length tracking
    pub length: usize,
}

impl CollateralArrays {
    pub fn new() -> Self {
        Self {
            reserve_keys: Vec::new(),
            deposited_amounts: Vec::new(),
            market_values_usd: Vec::new(),
            liquidation_thresholds: Vec::new(),
            loan_to_value_ratios: Vec::new(),
            reserve_to_index: std::collections::HashMap::new(),
            length: 0,
        }
    }

    /// Add collateral with cache-friendly operations
    pub fn add_collateral(
        &mut self,
        reserve: Pubkey,
        amount: u64,
        market_value: Decimal,
        liquidation_threshold_bps: u16,
        ltv_bps: u16,
    ) -> Result<()> {
        if self.length >= crate::constants::MAX_OBLIGATION_RESERVES {
            return Err(LendingError::ObligationDepositsMaxed.into());
        }

        let index = self.length;
        
        // Add to parallel arrays
        self.reserve_keys.push(reserve);
        self.deposited_amounts.push(amount);
        self.market_values_usd.push(market_value.try_floor_u64()?);
        self.liquidation_thresholds.push(liquidation_threshold_bps);
        self.loan_to_value_ratios.push(ltv_bps);
        
        // Update index
        self.reserve_to_index.insert(reserve, index);
        self.length += 1;
        
        Ok(())
    }

    /// Get collateral by reserve with O(1) lookup
    pub fn get_collateral(&self, reserve: &Pubkey) -> Option<CollateralView> {
        self.reserve_to_index.get(reserve).map(|&index| {
            CollateralView {
                reserve: self.reserve_keys[index],
                deposited_amount: self.deposited_amounts[index],
                market_value_usd: self.market_values_usd[index],
                liquidation_threshold_bps: self.liquidation_thresholds[index],
                loan_to_value_bps: self.loan_to_value_ratios[index],
            }
        })
    }

    /// Vectorized calculation of total value - cache-friendly
    pub fn calculate_total_value(&self) -> u64 {
        // Single pass through market_values_usd array - excellent cache locality
        self.market_values_usd.iter().sum()
    }

    /// Vectorized calculation with SIMD potential
    pub fn calculate_weighted_ltv(&self) -> Result<u64> {
        if self.length == 0 {
            return Ok(0);
        }

        let mut total_value = 0u128;
        let mut weighted_ltv = 0u128;
        
        // Parallel iteration over arrays - compiler can optimize with SIMD
        for i in 0..self.length {
            let value = self.market_values_usd[i] as u128;
            let ltv = self.loan_to_value_ratios[i] as u128;
            
            total_value += value;
            weighted_ltv += value * ltv;
        }
        
        if total_value == 0 {
            return Ok(0);
        }
        
        Ok((weighted_ltv / total_value) as u64)
    }

    /// Remove collateral efficiently
    pub fn remove_collateral(&mut self, reserve: &Pubkey) -> Result<()> {
        let index = self.reserve_to_index.remove(reserve)
            .ok_or(LendingError::ObligationReserveNotFound)?;
        
        // Use swap_remove for O(1) removal (trades order for performance)
        self.reserve_keys.swap_remove(index);
        self.deposited_amounts.swap_remove(index);
        self.market_values_usd.swap_remove(index);
        self.liquidation_thresholds.swap_remove(index);
        self.loan_to_value_ratios.swap_remove(index);
        
        self.length -= 1;
        
        // Update index map for swapped element
        if index < self.length {
            let swapped_reserve = self.reserve_keys[index];
            self.reserve_to_index.insert(swapped_reserve, index);
        }
        
        Ok(())
    }
}

/// View structure for collateral data
#[derive(Debug, Clone)]
pub struct CollateralView {
    pub reserve: Pubkey,
    pub deposited_amount: u64,
    pub market_value_usd: u64,
    pub liquidation_threshold_bps: u16,
    pub loan_to_value_bps: u16,
}

/// Cache-aware data prefetching utilities
pub mod prefetch {
    use super::*;

    /// Prefetch data to improve cache performance
    pub fn prefetch_obligations(obligation_keys: &[Pubkey], accounts: &[AccountInfo]) {
        // In a real implementation, this would use CPU prefetch instructions
        // For now, we simulate by accessing the first few bytes of each account
        for account in accounts.iter().take(8) { // Limit to prevent excessive work
            if !account.data.borrow().is_empty() {
                let _ = account.data.borrow()[0]; // Touch first byte to trigger cache load
            }
        }
    }

    /// Sequential access pattern for cache-friendly iteration
    pub fn sequential_health_factor_calculation(
        obligations: &[ObligationCacheOptimized]
    ) -> Vec<Option<Decimal>> {
        let mut results = Vec::with_capacity(obligations.len());
        
        // Process in sequential order for optimal cache usage
        for obligation in obligations {
            let health_factor = calculate_health_factor_cached(obligation);
            results.push(health_factor);
        }
        
        results
    }

    /// Calculate health factor using cached values when possible
    fn calculate_health_factor_cached(obligation: &ObligationCacheOptimized) -> Option<Decimal> {
        // Use cached values to avoid recomputation
        if obligation.borrowed_value_usd.is_zero() {
            return Some(Decimal::from_integer(u64::MAX).unwrap_or(Decimal::zero()));
        }
        
        if obligation.deposited_value_usd.is_zero() {
            return Some(Decimal::zero());
        }
        
        // Simple calculation using cached values
        obligation.deposited_value_usd.try_div(obligation.borrowed_value_usd).ok()
    }
}

/// Memory allocation strategies for different use cases
pub mod allocation_strategies {
    use super::*;

    /// Arena allocator for temporary calculations
    pub struct ArenaAllocator {
        buffer: Vec<u8>,
        offset: usize,
    }

    impl ArenaAllocator {
        pub fn new(size: usize) -> Self {
            Self {
                buffer: vec![0; size],
                offset: 0,
            }
        }

        /// Allocate aligned memory block
        pub fn allocate<T>(&mut self, count: usize) -> Result<&mut [T]> {
            let size = size_of::<T>() * count;
            let align = align_of::<T>();
            
            // Align offset
            let aligned_offset = (self.offset + align - 1) & !(align - 1);
            
            if aligned_offset + size > self.buffer.len() {
                return Err(LendingError::InsufficientMemory.into());
            }
            
            self.offset = aligned_offset + size;
            
            // Cast buffer slice to T slice
            let ptr = self.buffer[aligned_offset..].as_mut_ptr() as *mut T;
            Ok(unsafe { std::slice::from_raw_parts_mut(ptr, count) })
        }

        /// Reset arena for reuse
        pub fn reset(&mut self) {
            self.offset = 0;
        }

        /// Get utilization ratio
        pub fn utilization(&self) -> f64 {
            (self.offset as f64) / (self.buffer.len() as f64)
        }
    }

    /// Stack allocator for fixed-size temporary data
    pub struct StackAllocator<const SIZE: usize> {
        buffer: [u8; SIZE],
        top: usize,
    }

    impl<const SIZE: usize> StackAllocator<SIZE> {
        pub fn new() -> Self {
            Self {
                buffer: [0; SIZE],
                top: 0,
            }
        }

        /// Push data onto stack
        pub fn push<T>(&mut self, count: usize) -> Result<&mut [T]> {
            let size = size_of::<T>() * count;
            let align = align_of::<T>();
            
            let aligned_top = (self.top + align - 1) & !(align - 1);
            
            if aligned_top + size > SIZE {
                return Err(LendingError::StackOverflow.into());
            }
            
            let old_top = aligned_top;
            self.top = aligned_top + size;
            
            let ptr = self.buffer[old_top..].as_mut_ptr() as *mut T;
            Ok(unsafe { std::slice::from_raw_parts_mut(ptr, count) })
        }

        /// Pop data from stack
        pub fn pop<T>(&mut self, count: usize) {
            let size = size_of::<T>() * count;
            self.top = self.top.saturating_sub(size);
        }
    }
}

/// Cache-friendly algorithms for common operations
pub mod cache_algorithms {
    use super::*;

    /// Block-wise matrix operations for large datasets
    pub fn blocked_health_factor_batch(
        obligations: &[ObligationCacheOptimized],
        block_size: usize,
    ) -> Vec<Option<Decimal>> {
        let mut results = Vec::with_capacity(obligations.len());
        
        // Process in blocks to maintain cache locality
        for chunk in obligations.chunks(block_size) {
            let block_results = prefetch::sequential_health_factor_calculation(chunk);
            results.extend(block_results);
        }
        
        results
    }

    /// Cache-oblivious algorithm for sorting
    pub fn cache_oblivious_sort<T: Ord + Clone>(data: &mut [T]) {
        if data.len() <= 32 {
            // Use insertion sort for small arrays (cache-friendly)
            insertion_sort(data);
        } else {
            // Divide and conquer with cache-aware recursion
            let mid = data.len() / 2;
            cache_oblivious_sort(&mut data[..mid]);
            cache_oblivious_sort(&mut data[mid..]);
            merge_cache_friendly(data, mid);
        }
    }

    fn insertion_sort<T: Ord>(data: &mut [T]) {
        for i in 1..data.len() {
            let mut j = i;
            while j > 0 && data[j - 1] > data[j] {
                data.swap(j - 1, j);
                j -= 1;
            }
        }
    }

    fn merge_cache_friendly<T: Ord + Clone>(data: &mut [T], mid: usize) {
        let mut temp = Vec::with_capacity(data.len());
        let (left, right) = data.split_at(mid);
        
        let mut i = 0;
        let mut j = 0;
        
        // Merge with cache-friendly access patterns
        while i < left.len() && j < right.len() {
            if left[i] <= right[j] {
                temp.push(left[i].clone());
                i += 1;
            } else {
                temp.push(right[j].clone());
                j += 1;
            }
        }
        
        temp.extend_from_slice(&left[i..]);
        temp.extend_from_slice(&right[j..]);
        
        data.clone_from_slice(&temp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pool() {
        let mut pool: MemoryPool<u64> = MemoryPool::new(4);
        
        // Test allocation
        let (index1, value1) = pool.allocate().unwrap();
        *value1 = 42;
        
        let (index2, value2) = pool.allocate().unwrap();
        *value2 = 84;
        
        assert_ne!(index1, index2);
        
        // Test deallocation
        pool.deallocate(index1);
        
        // Test reallocation of deallocated slot
        let (index3, value3) = pool.allocate().unwrap();
        assert_eq!(index1, index3); // Should reuse deallocated slot
        
        let stats = pool.get_stats();
        assert_eq!(stats.allocations, 3);
        assert_eq!(stats.deallocations, 1);
    }

    #[test]
    fn test_collateral_arrays() {
        let mut arrays = CollateralArrays::new();
        
        let reserve = Pubkey::new_unique();
        arrays.add_collateral(
            reserve,
            1000,
            Decimal::from_integer(1000).unwrap(),
            8000,
            7500,
        ).unwrap();
        
        let collateral = arrays.get_collateral(&reserve).unwrap();
        assert_eq!(collateral.deposited_amount, 1000);
        assert_eq!(collateral.liquidation_threshold_bps, 8000);
        
        let total_value = arrays.calculate_total_value();
        assert_eq!(total_value, 1000);
    }

    #[test]
    fn test_arena_allocator() {
        let mut arena = allocation_strategies::ArenaAllocator::new(1024);
        
        let slice1: &mut [u64] = arena.allocate(10).unwrap();
        assert_eq!(slice1.len(), 10);
        
        let slice2: &mut [u32] = arena.allocate(5).unwrap();
        assert_eq!(slice2.len(), 5);
        
        assert!(arena.utilization() > 0.0);
        
        arena.reset();
        assert_eq!(arena.utilization(), 0.0);
    }

    #[test]
    fn test_cache_friendly_sort() {
        let mut data = vec![5, 2, 8, 1, 9, 3];
        cache_algorithms::cache_oblivious_sort(&mut data);
        assert_eq!(data, vec![1, 2, 3, 5, 8, 9]);
    }
}