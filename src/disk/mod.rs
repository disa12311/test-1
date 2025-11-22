// Advanced disk cleaning functionality with parallel processing
pub mod temp_files;
pub mod browser_cache;
pub mod thumbnails;
pub mod recycle_bin;
pub mod registry_cache;
pub mod windows_logs;

use anyhow::{Result, Context};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::path::PathBuf;

// ============================================================================
// CONFIGURATION
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiskCleaningOptions {
    pub clean_temp_files: bool,
    pub clean_browser_cache: bool,
    pub clean_thumbnails: bool,
    pub clean_recycle_bin: bool,
    pub clean_system_cache: bool,
    pub clean_windows_logs: bool,
    pub clean_downloads: bool,
    
    // Advanced options
    pub size_threshold_mb: Option<u64>, // Only clean if total > threshold
    pub parallel_processing: bool,
    pub dry_run: bool,
    pub skip_files_in_use: bool,
    pub preserve_recent_days: Option<u32>, // Keep files newer than N days
}

impl Default for DiskCleaningOptions {
    fn default() -> Self {
        Self {
            clean_temp_files: true,
            clean_browser_cache: true,
            clean_thumbnails: true,
            clean_recycle_bin: false,
            clean_system_cache: false,
            clean_windows_logs: false,
            clean_downloads: false,
            
            size_threshold_mb: None,
            parallel_processing: true,
            dry_run: false,
            skip_files_in_use: true,
            preserve_recent_days: None,
        }
    }
}

// ============================================================================
// RESULTS STRUCTURES
// ============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiskScanResult {
    pub start_time: DateTime<Local>,
    pub end_time: Option<DateTime<Local>>,
    pub total_space_to_free: u64,
    pub temp_files_size: u64,
    pub cache_size: u64,
    pub thumbnails_size: u64,
    pub recycle_bin_size: u64,
    pub windows_logs_size: u64,
    pub downloads_size: u64,
    pub file_count: usize,
    pub folder_count: usize,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub duration: Option<std::time::Duration>,
}

impl DiskScanResult {
    pub fn new() -> Self {
        Self {
            start_time: Local::now(),
            ..Default::default()
        }
    }

    pub fn complete(&mut self) {
        self.end_time = Some(Local::now());
        if let Some(end) = self.end_time {
            self.duration = Some(std::time::Duration::from_millis(
                (end.timestamp_millis() - self.start_time.timestamp_millis()) as u64
            ));
        }
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiskCleanResult {
    pub start_time: DateTime<Local>,
    pub end_time: Option<DateTime<Local>>,
    pub total_space_freed: u64,
    pub temp_files_freed: u64,
    pub cache_freed: u64,
    pub thumbnails_freed: u64,
    pub recycle_bin_freed: u64,
    pub windows_logs_freed: u64,
    pub downloads_freed: u64,
    pub files_deleted: usize,
    pub folders_deleted: usize,
    pub files_failed: usize,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub duration: Option<std::time::Duration>,
    pub was_cancelled: bool,
}

impl DiskCleanResult {
    pub fn new() -> Self {
        Self {
            start_time: Local::now(),
            ..Default::default()
        }
    }

    pub fn complete(&mut self) {
        self.end_time = Some(Local::now());
        if let Some(end) = self.end_time {
            self.duration = Some(std::time::Duration::from_millis(
                (end.timestamp_millis() - self.start_time.timestamp_millis()) as u64
            ));
        }
    }

    pub fn success_rate(&self) -> f32 {
        let total = self.files_deleted + self.files_failed;
        if total == 0 {
            0.0
        } else {
            (self.files_deleted as f32 / total as f32) * 100.0
        }
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

// ============================================================================
// ADVANCED DISK CLEANER
// ============================================================================

pub struct AdvancedDiskCleaner {
    options: DiskCleaningOptions,
    cancel_flag: Arc<AtomicBool>,
    bytes_processed: Arc<AtomicU64>,
    files_processed: Arc<AtomicU64>,
}

impl AdvancedDiskCleaner {
    pub fn new(options: DiskCleaningOptions) -> Self {
        Self {
            options,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            bytes_processed: Arc::new(AtomicU64::new(0)),
            files_processed: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }

    pub fn bytes_processed(&self) -> u64 {
        self.bytes_processed.load(Ordering::Relaxed)
    }

    pub fn files_processed(&self) -> u64 {
        self.files_processed.load(Ordering::Relaxed)
    }

    pub async fn scan(&self) -> Result<DiskScanResult> {
        let mut result = DiskScanResult::new();
        
        tracing::info!("🔍 Starting disk scan with options: {:?}", self.options);

        // Parallel scanning if enabled
        if self.options.parallel_processing {
            self.scan_parallel(&mut result).await?;
        } else {
            self.scan_sequential(&mut result).await?;
        }

        result.complete();
        
        tracing::info!(
            "✅ Scan completed: {} MB found in {} files ({:.2}s)",
            result.total_space_to_free / 1024 / 1024,
            result.file_count,
            result.duration.map(|d| d.as_secs_f64()).unwrap_or(0.0)
        );

        Ok(result)
    }

    async fn scan_parallel(&self, result: &mut DiskScanResult) -> Result<()> {
        use tokio::task;

        let mut handles = vec![];

        if self.options.clean_temp_files {
            let handle = task::spawn(async move {
                temp_files::get_temp_file_size_async().await
            });
            handles.push(("temp", handle));
        }

        if self.options.clean_browser_cache {
            let handle = task::spawn(async move {
                browser_cache::get_browser_cache_size_async().await
            });
            handles.push(("cache", handle));
        }

        if self.options.clean_thumbnails {
            let handle = task::spawn(async move {
                thumbnails::get_thumbnails_size_async().await
            });
            handles.push(("thumbnails", handle));
        }

        if self.options.clean_recycle_bin {
            let handle = task::spawn(async move {
                recycle_bin::get_recycle_bin_size_sync()
            });
            handles.push(("recycle", handle));
        }

        // Collect results
        for (name, handle) in handles {
            match handle.await {
                Ok(Ok(size)) => {
                    match name {
                        "temp" => result.temp_files_size = size,
                        "cache" => result.cache_size = size,
                        "thumbnails" => result.thumbnails_size = size,
                        "recycle" => result.recycle_bin_size = size,
                        _ => {}
                    }
                    result.total_space_to_free += size;
                }
                Ok(Err(e)) => {
                    result.add_error(format!("{} scan failed: {}", name, e));
                }
                Err(e) => {
                    result.add_error(format!("{} task failed: {}", name, e));
                }
            }
        }

        Ok(())
    }

    async fn scan_sequential(&self, result: &mut DiskScanResult) -> Result<()> {
        if self.options.clean_temp_files {
            match temp_files::get_temp_file_size_async().await {
                Ok(size) => {
                    result.temp_files_size = size;
                    result.total_space_to_free += size;
                }
                Err(e) => result.add_error(format!("Temp files scan error: {}", e)),
            }
        }

        if self.options.clean_browser_cache {
            match browser_cache::get_browser_cache_size_async().await {
                Ok(size) => {
                    result.cache_size = size;
                    result.total_space_to_free += size;
                }
                Err(e) => result.add_error(format!("Cache scan error: {}", e)),
            }
        }

        if self.options.clean_thumbnails {
            match thumbnails::get_thumbnails_size_async().await {
                Ok(size) => {
                    result.thumbnails_size = size;
                    result.total_space_to_free += size;
                }
                Err(e) => result.add_error(format!("Thumbnails scan error: {}", e)),
            }
        }

        if self.options.clean_recycle_bin {
            match recycle_bin::get_recycle_bin_size_sync() {
                Ok(size) => {
                    result.recycle_bin_size = size;
                    result.total_space_to_free += size;
                }
                Err(e) => result.add_error(format!("Recycle bin scan error: {}", e)),
            }
        }

        Ok(())
    }

    pub async fn clean(&self) -> Result<DiskCleanResult> {
        let mut result = DiskCleanResult::new();

        // Check threshold
        if let Some(threshold_mb) = self.options.size_threshold_mb {
            let scan = self.scan().await?;
            let threshold_bytes = threshold_mb * 1024 * 1024;
            
            if scan.total_space_to_free < threshold_bytes {
                tracing::info!(
                    "⏭️ Skipping cleaning: {} MB < {} MB threshold",
                    scan.total_space_to_free / 1024 / 1024,
                    threshold_mb
                );
                result.complete();
                return Ok(result);
            }
        }

        tracing::info!("🧹 Starting disk cleaning...");

        // Parallel cleaning if enabled
        if self.options.parallel_processing && !self.options.dry_run {
            self.clean_parallel(&mut result).await?;
        } else {
            self.clean_sequential(&mut result).await?;
        }

        result.complete();

        tracing::info!(
            "✅ Cleaning completed: {} MB freed, {} files deleted ({:.1}% success)",
            result.total_space_freed / 1024 / 1024,
            result.files_deleted,
            result.success_rate()
        );

        Ok(result)
    }

    async fn clean_parallel(&self, result: &mut DiskCleanResult) -> Result<()> {
        use tokio::task;

        let mut handles = vec![];

        if self.options.clean_temp_files {
            let handle = task::spawn(async move {
                temp_files::clean_temp_files_sync()
            });
            handles.push(("temp", handle));
        }

        if self.options.clean_browser_cache {
            let handle = task::spawn(async move {
                browser_cache::clean_browser_cache_sync()
            });
            handles.push(("cache", handle));
        }

        if self.options.clean_thumbnails {
            let handle = task::spawn(async move {
                thumbnails::clean_thumbnails_sync()
            });
            handles.push(("thumbnails", handle));
        }

        if self.options.clean_recycle_bin {
            let handle = task::spawn(async move {
                recycle_bin::empty_recycle_bin_sync()
            });
            handles.push(("recycle", handle));
        }

        // Collect results
        for (name, handle) in handles {
            if self.is_cancelled() {
                result.was_cancelled = true;
                break;
            }

            match handle.await {
                Ok(Ok(freed)) => {
                    match name {
                        "temp" => result.temp_files_freed = freed,
                        "cache" => result.cache_freed = freed,
                        "thumbnails" => result.thumbnails_freed = freed,
                        "recycle" => result.recycle_bin_freed = freed,
                        _ => {}
                    }
                    result.total_space_freed += freed;
                }
                Ok(Err(e)) => {
                    result.add_error(format!("{} cleaning failed: {}", name, e));
                }
                Err(e) => {
                    result.add_error(format!("{} task failed: {}", name, e));
                }
            }
        }

        Ok(())
    }

    async fn clean_sequential(&self, result: &mut DiskCleanResult) -> Result<()> {
        if self.options.clean_temp_files && !self.is_cancelled() {
            match temp_files::clean_temp_files_sync() {
                Ok(freed) => {
                    result.temp_files_freed = freed;
                    result.total_space_freed += freed;
                }
                Err(e) => result.add_error(format!("Temp files error: {}", e)),
            }
        }

        if self.options.clean_browser_cache && !self.is_cancelled() {
            match browser_cache::clean_browser_cache_sync() {
                Ok(freed) => {
                    result.cache_freed = freed;
                    result.total_space_freed += freed;
                }
                Err(e) => result.add_error(format!("Cache error: {}", e)),
            }
        }

        if self.options.clean_thumbnails && !self.is_cancelled() {
            match thumbnails::clean_thumbnails_sync() {
                Ok(freed) => {
                    result.thumbnails_freed = freed;
                    result.total_space_freed += freed;
                }
                Err(e) => result.add_error(format!("Thumbnails error: {}", e)),
            }
        }

        if self.options.clean_recycle_bin && !self.is_cancelled() {
            match recycle_bin::empty_recycle_bin_sync() {
                Ok(freed) => {
                    result.recycle_bin_freed = freed;
                    result.total_space_freed += freed;
                }
                Err(e) => result.add_error(format!("Recycle bin error: {}", e)),
            }
        }

        if self.is_cancelled() {
            result.was_cancelled = true;
        }

        Ok(())
    }
}

// ============================================================================
// COMPATIBILITY FUNCTIONS (for existing code)
// ============================================================================

pub async fn clean_disk_with_options(options: DiskCleaningOptions) -> Result<DiskCleanResult> {
    let cleaner = AdvancedDiskCleaner::new(options);
    cleaner.clean().await
}

pub async fn scan_disk_with_options(options: DiskCleaningOptions) -> Result<DiskScanResult> {
    let cleaner = AdvancedDiskCleaner::new(options);
    cleaner.scan().await
}

pub fn scan_disk_with_options_sync(options: DiskCleaningOptions) -> Result<DiskScanResult> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(scan_disk_with_options(options))
}

pub fn clean_disk_with_options_sync(options: DiskCleaningOptions) -> Result<DiskCleanResult> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(clean_disk_with_options(options))
}

pub fn clean_disk_with_options_threaded(options: DiskCleaningOptions) -> Result<DiskCleanResult> {
    clean_disk_with_options_sync(options)
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

pub fn format_bytes(bytes: u64) -> String {
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const KB: f64 = 1024.0;
    
    let bytes_f = bytes as f64;
    
    if bytes_f >= GB {
        format!("{:.2} GB", bytes_f / GB)
    } else if bytes_f >= MB {
        format!("{:.0} MB", bytes_f / MB)
    } else if bytes_f >= KB {
        format!("{:.0} KB", bytes_f / KB)
    } else {
        format!("{} B", bytes)
    }
}

pub fn should_preserve_file(path: &PathBuf, preserve_days: Option<u32>) -> bool {
    if let Some(days) = preserve_days {
        if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(age) = modified.elapsed() {
                    let age_days = age.as_secs() / 86400;
                    return age_days < days as u64;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(1024), "1 KB");
        assert_eq!(format_bytes(1024 * 1024), "1 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_default_options() {
        let opts = DiskCleaningOptions::default();
        assert!(opts.clean_temp_files);
        assert!(opts.clean_browser_cache);
        assert!(!opts.clean_recycle_bin);
        assert!(opts.parallel_processing);
    }

    #[test]
    fn test_clean_result_success_rate() {
        let mut result = DiskCleanResult::new();
        result.files_deleted = 80;
        result.files_failed = 20;
        assert_eq!(result.success_rate(), 80.0);
    }
}