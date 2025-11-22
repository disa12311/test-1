#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_info_calculation() {
        let info = SystemMemoryInfo {
            total_physical: 16_000_000_000, // 16 GB
            avail_physical: 4_000_000_000,  // 4 GB
            total_pagefile: 24_000_000_000,
            avail_pagefile: 12_000_000_000,
        };

        assert_eq!(info.used_physical(), 12_000_000_000);
        assert_eq!(info.used_physical_percent(), 75.0);
        assert_eq!(info.available_percent(), 25.0);
    }

    #[test]
    fn test_cleaning_results_calculation() {
        let mut results = CleaningResults::new();
        results.total_memory_before = 8_000_000_000;
        results.total_memory_after = 6_000_000_000;
        results.processes_attempted = 100;
        results.processes_succeeded = 85;

        assert_eq!(results.total_freed(), 2_000_000_000);
        assert_eq!(results.success_rate(), 85.0);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(1024), "1 KB");
        assert_eq!(format_bytes(1024 * 1024), "1 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_bytes(1536 * 1024 * 1024), "1.50 GB");
    }

    #[test]
    fn test_blacklist() {
        let cleaner = AdvancedMemoryCleaner::new();
        
        assert!(cleaner.is_process_blacklisted("System"));
        assert!(cleaner.is_process_blacklisted("csrss.exe"));
        assert!(cleaner.is_process_blacklisted("explorer.exe"));
        assert!(!cleaner.is_process_blacklisted("chrome.exe"));
    }

    #[test]
    fn test_memory_monitor() {
        let mut monitor = MemoryMonitor::new(10);
        
        // Simulate some snapshots
        for _ in 0..5 {
            monitor.record_snapshot();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        assert_eq!(monitor.get_history().len(), 5);
        assert!(monitor.average_usage() > 0.0);
    }

    #[test]
    fn test_should_clean_memory() {
        // This will use actual system memory
        let should_clean_low = should_clean_memory(99.0);
        let should_clean_high = should_clean_memory(1.0);
        
        assert!(!should_clean_low); // Unlikely to be 99% full
        assert!(should_clean_high);  // Should always be > 1%
    }

    #[test]
    fn test_dry_run() {
        let cleaner = AdvancedMemoryCleaner::new().dry_run(true);
        assert!(cleaner.dry_run);
        
        // Dry run should not actually clean anything
        // (would need mock or integration test)
    }

    #[test]
    fn test_process_cleaned_struct() {
        let process = ProcessCleaned {
            pid: 1234,
            name: "test.exe".to_string(),
            memory_freed: 1024 * 1024,
            memory_before: 10 * 1024 * 1024,
            memory_after: 9 * 1024 * 1024,
            success: true,
        };

        assert_eq!(process.memory_freed, 1024 * 1024);
        assert!(process.success);
    }
}

#[cfg(test)]
mod benchmark_tests {
    use super::*;

    #[test]
    #[ignore] // Run with: cargo test --release -- --ignored
    fn benchmark_memory_info() {
        let start = std::time::Instant::now();
        
        for _ in 0..1000 {
            let _ = get_detailed_system_memory_info();
        }
        
        let duration = start.elapsed();
        println!("1000 memory info calls: {:?}", duration);
        assert!(duration.as_millis() < 1000); // Should be fast
    }

    #[test]
    #[ignore]
    fn benchmark_format_bytes() {
        let start = std::time::Instant::now();
        
        for i in 0..100000 {
            let _ = format_bytes(i * 1024);
        }
        
        let duration = start.elapsed();
        println!("100k format operations: {:?}", duration);
    }
}