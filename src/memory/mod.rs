// Optimized memory management module with advanced features
use anyhow::{Result, Context};
use chrono::{DateTime, Local, Duration};
use serde::{Deserialize, Serialize};
use sysinfo::System;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};

#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, BOOL, MAX_PATH, HANDLE};
#[cfg(windows)]
use windows_sys::Win32::System::ProcessStatus::{
    EmptyWorkingSet, EnumProcesses, GetModuleBaseNameW, K32GetProcessMemoryInfo,
    PROCESS_MEMORY_COUNTERS,
};
#[cfg(windows)]
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
#[cfg(windows)]
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_SET_QUOTA, PROCESS_VM_READ,
};

// ============================================================================
// STRUCTURES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessCleaned {
    pub pid: u32,
    pub name: String,
    pub memory_freed: usize,
    pub memory_before: usize,
    pub memory_after: usize,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleaningResults {
    pub start_time: DateTime<Local>,
    pub end_time: Option<DateTime<Local>>,
    pub processes: Vec<ProcessCleaned>,
    pub total_memory_before: usize,
    pub total_memory_after: usize,
    pub processes_attempted: usize,
    pub processes_succeeded: usize,
    pub has_error: bool,
    pub error_message: String,
    pub is_completed: bool,
    pub duration_ms: Option<u64>,
}

impl CleaningResults {
    pub fn new() -> Self {
        Self {
            start_time: Local::now(),
            end_time: None,
            processes: Vec::new(),
            total_memory_before: 0,
            total_memory_after: 0,
            processes_attempted: 0,
            processes_succeeded: 0,
            has_error: false,
            error_message: String::new(),
            is_completed: false,
            duration_ms: None,
        }
    }

    pub fn total_freed(&self) -> usize {
        if self.total_memory_before > self.total_memory_after {
            self.total_memory_before - self.total_memory_after
        } else {
            0
        }
    }

    pub fn complete(&mut self) {
        let end = Local::now();
        self.end_time = Some(end);
        self.is_completed = true;
        
        if let Ok(duration) = (end - self.start_time).to_std() {
            self.duration_ms = Some(duration.as_millis() as u64);
        }
    }

    pub fn success_rate(&self) -> f32 {
        if self.processes_attempted == 0 {
            0.0
        } else {
            (self.processes_succeeded as f32 / self.processes_attempted as f32) * 100.0
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SystemMemoryInfo {
    pub total_physical: u64,
    pub avail_physical: u64,
    pub total_pagefile: u64,
    pub avail_pagefile: u64,
}

impl SystemMemoryInfo {
    pub fn used_physical(&self) -> u64 {
        self.total_physical.saturating_sub(self.avail_physical)
    }

    pub fn used_physical_percent(&self) -> f32 {
        if self.total_physical == 0 {
            0.0
        } else {
            (self.used_physical() as f32 / self.total_physical as f32) * 100.0
        }
    }

    pub fn available_percent(&self) -> f32 {
        if self.total_physical == 0 {
            0.0
        } else {
            (self.avail_physical as f32 / self.total_physical as f32) * 100.0
        }
    }

    pub fn used_pagefile(&self) -> u64 {
        self.total_pagefile.saturating_sub(self.avail_pagefile)
    }

    pub fn pagefile_usage_percent(&self) -> f32 {
        if self.total_pagefile == 0 {
            0.0
        } else {
            (self.used_pagefile() as f32 / self.total_pagefile as f32) * 100.0
        }
    }
}

// ============================================================================
// ADVANCED MEMORY CLEANER WITH OPTIMIZATION
// ============================================================================

pub struct AdvancedMemoryCleaner {
    blacklisted_processes: Vec<String>,
    min_memory_threshold: usize, // Don't clean processes using less than this
    dry_run: bool,
    cancel_flag: Arc<AtomicBool>,
}

impl Default for AdvancedMemoryCleaner {
    fn default() -> Self {
        Self {
            blacklisted_processes: vec![
                // Critical Windows processes
                "System".to_string(),
                "Registry".to_string(),
                "smss.exe".to_string(),
                "csrss.exe".to_string(),
                "wininit.exe".to_string(),
                "winlogon.exe".to_string(),
                "services.exe".to_string(),
                "lsass.exe".to_string(),
                "svchost.exe".to_string(),
                "dwm.exe".to_string(),
                "explorer.exe".to_string(),
                // Critical drivers
                "ntoskrnl.exe".to_string(),
                "hal.dll".to_string(),
            ],
            min_memory_threshold: 10 * 1024 * 1024, // 10 MB minimum
            dry_run: false,
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl AdvancedMemoryCleaner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_blacklist(mut self, blacklist: Vec<String>) -> Self {
        self.blacklisted_processes = blacklist;
        self
    }

    pub fn with_threshold(mut self, threshold_mb: usize) -> Self {
        self.min_memory_threshold = threshold_mb * 1024 * 1024;
        self
    }

    pub fn dry_run(mut self, enabled: bool) -> Self {
        self.dry_run = enabled;
        self
    }

    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }

    fn is_process_blacklisted(&self, name: &str) -> bool {
        let lower_name = name.to_lowercase();
        self.blacklisted_processes.iter().any(|bl| {
            lower_name.contains(&bl.to_lowercase())
        })
    }
}

// ============================================================================
// WINDOWS IMPLEMENTATION
// ============================================================================

#[cfg(windows)]
pub fn clean_memory() -> Result<CleaningResults> {
    let cleaner = AdvancedMemoryCleaner::new();
    clean_memory_advanced(cleaner)
}

#[cfg(windows)]
pub fn clean_memory_advanced(cleaner: AdvancedMemoryCleaner) -> Result<CleaningResults> {
    let mut results = CleaningResults::new();
    
    tracing::info!("🧹 Starting advanced memory cleaning...");
    
    // Get all PIDs
    let pids = enumerate_processes()
        .context("Failed to enumerate processes")?;
    
    tracing::info!("📊 Found {} processes to scan", pids.len());
    
    // Clean current process first
    clean_current_process(&mut results)?;
    
    // Clean other processes
    for pid in pids {
        if cleaner.is_cancelled() {
            tracing::warn!("❌ Cleaning cancelled by user");
            results.error_message = "Operation cancelled by user".to_string();
            break;
        }
        
        if pid == 0 || pid == 4 {
            continue; // Skip System Idle and System
        }
        
        results.processes_attempted += 1;
        
        match clean_process(pid, &cleaner) {
            Ok(Some(process_result)) => {
                if process_result.success {
                    results.processes_succeeded += 1;
                }
                
                results.total_memory_before += process_result.memory_before;
                results.total_memory_after += process_result.memory_after;
                
                if process_result.memory_freed > 0 {
                    results.processes.push(process_result);
                }
            }
            Ok(None) => {
                // Process skipped (blacklisted or too small)
            }
            Err(e) => {
                tracing::debug!("Failed to clean PID {}: {}", pid, e);
                // Don't stop on individual process failures
            }
        }
    }
    
    // Sort by memory freed
    results.processes.sort_by(|a, b| b.memory_freed.cmp(&a.memory_freed));
    
    results.complete();
    
    tracing::info!(
        "✅ Cleaning completed: {} MB freed from {} processes ({:.1}% success rate)",
        results.total_freed() / 1024 / 1024,
        results.processes_succeeded,
        results.success_rate()
    );
    
    Ok(results)
}

#[cfg(windows)]
fn enumerate_processes() -> Result<Vec<u32>> {
    let mut pids = vec![0u32; 4096]; // Increased buffer
    let mut bytes_returned = 0;

    unsafe {
        if EnumProcesses(
            pids.as_mut_ptr(),
            (std::mem::size_of::<u32>() * pids.len()) as u32,
            &mut bytes_returned,
        ) == 0 {
            return Err(anyhow::anyhow!("EnumProcesses failed"));
        }
    }

    let count = bytes_returned as usize / std::mem::size_of::<u32>();
    pids.truncate(count);
    
    Ok(pids)
}

#[cfg(windows)]
fn clean_current_process(results: &mut CleaningResults) -> Result<()> {
    unsafe {
        let handle = GetCurrentProcess();
        if EmptyWorkingSet(handle) != 0 {
            tracing::debug!("✅ Cleaned current process memory");
        }
    }
    Ok(())
}

#[cfg(windows)]
fn clean_process(pid: u32, cleaner: &AdvancedMemoryCleaner) -> Result<Option<ProcessCleaned>> {
    unsafe {
        let handle = open_process_for_cleaning(pid)?;
        
        // Get process name
        let name = get_process_name(handle, pid);
        
        // Check blacklist
        if cleaner.is_process_blacklisted(&name) {
            CloseHandle(handle);
            return Ok(None);
        }
        
        // Get memory info before
        let mem_before = get_process_memory(handle)?;
        
        // Skip if below threshold
        if mem_before < cleaner.min_memory_threshold {
            CloseHandle(handle);
            return Ok(None);
        }
        
        // Clean working set (unless dry run)
        let success = if !cleaner.dry_run {
            EmptyWorkingSet(handle) != 0
        } else {
            true // Simulate success in dry run
        };
        
        // Get memory info after
        let mem_after = if !cleaner.dry_run {
            // Small delay to let the system update
            std::thread::sleep(std::time::Duration::from_millis(5));
            get_process_memory(handle).unwrap_or(mem_before)
        } else {
            mem_before // No change in dry run
        };
        
        CloseHandle(handle);
        
        let freed = mem_before.saturating_sub(mem_after);
        
        Ok(Some(ProcessCleaned {
            pid,
            name,
            memory_freed: freed,
            memory_before: mem_before,
            memory_after: mem_after,
            success,
        }))
    }
}

#[cfg(windows)]
fn open_process_for_cleaning(pid: u32) -> Result<HANDLE> {
    unsafe {
        let handle = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ | PROCESS_SET_QUOTA,
            BOOL::from(false),
            pid,
        );
        
        if handle.is_null() {
            return Err(anyhow::anyhow!("Failed to open process {}", pid));
        }
        
        Ok(handle)
    }
}

#[cfg(windows)]
fn get_process_name(handle: HANDLE, pid: u32) -> String {
    unsafe {
        let mut name_buffer = [0u16; MAX_PATH as usize];
        let name_len = GetModuleBaseNameW(
            handle,
            std::ptr::null_mut(),
            name_buffer.as_mut_ptr(),
            MAX_PATH,
        );

        if name_len > 0 {
            String::from_utf16_lossy(&name_buffer[..name_len as usize])
        } else {
            format!("PID:{}", pid)
        }
    }
}

#[cfg(windows)]
fn get_process_memory(handle: HANDLE) -> Result<usize> {
    unsafe {
        let mut counters = PROCESS_MEMORY_COUNTERS {
            cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            PageFaultCount: 0,
            PeakWorkingSetSize: 0,
            WorkingSetSize: 0,
            QuotaPeakPagedPoolUsage: 0,
            QuotaPagedPoolUsage: 0,
            QuotaPeakNonPagedPoolUsage: 0,
            QuotaNonPagedPoolUsage: 0,
            PagefileUsage: 0,
            PeakPagefileUsage: 0,
        };

        if K32GetProcessMemoryInfo(
            handle,
            &mut counters,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        ) == 0 {
            return Err(anyhow::anyhow!("Failed to get process memory info"));
        }

        Ok(counters.WorkingSetSize)
    }
}

// ============================================================================
// LINUX IMPLEMENTATION
// ============================================================================

#[cfg(not(windows))]
pub fn clean_memory() -> Result<CleaningResults> {
    use crate::utils;
    
    let mut results = CleaningResults::new();
    let mut sys = System::new_all();
    sys.refresh_memory();
    
    results.total_memory_before = (sys.total_memory() - sys.available_memory()) as usize;

    if utils::is_elevated() {
        tracing::info!("🧹 Cleaning system caches with root privileges...");
        
        // Sync first
        if let Err(e) = std::process::Command::new("sync").output() {
            results.has_error = true;
            results.error_message = format!("Sync failed: {}", e);
            results.complete();
            return Ok(results);
        }
        
        // Drop caches (3 = pagecache + dentries + inodes)
        match std::process::Command::new("sh")
            .arg("-c")
            .arg("echo 3 > /proc/sys/vm/drop_caches")
            .output() 
        {
            Ok(output) => {
                if output.status.success() {
                    results.error_message = "System caches cleared successfully".to_string();
                } else {
                    results.has_error = true;
                    results.error_message = "Failed to clear caches".to_string();
                }
            }
            Err(e) => {
                results.has_error = true;
                results.error_message = format!("Cache clear command failed: {}", e);
            }
        }
    } else {
        results.has_error = true;
        results.error_message = "Root privileges required for cache clearing on Linux".to_string();
    }

    sys.refresh_memory();
    results.total_memory_after = (sys.total_memory() - sys.available_memory()) as usize;
    results.complete();
    
    tracing::info!(
        "✅ Linux cache cleaning: {} MB freed",
        results.total_freed() / 1024 / 1024
    );
    
    Ok(results)
}

// ============================================================================
// MEMORY INFO FUNCTIONS
// ============================================================================

#[cfg(windows)]
pub fn get_system_memory_info() -> (u64, u64) {
    let info = get_detailed_system_memory_info();
    (info.total_physical, info.used_physical())
}

#[cfg(not(windows))]
pub fn get_system_memory_info() -> (u64, u64) {
    let mut sys = System::new_all();
    sys.refresh_memory();
    (sys.total_memory(), sys.total_memory() - sys.available_memory())
}

#[cfg(windows)]
pub fn get_detailed_system_memory_info() -> SystemMemoryInfo {
    unsafe {
        let mut mem_info: MEMORYSTATUSEX = std::mem::zeroed();
        mem_info.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        
        if GlobalMemoryStatusEx(&mut mem_info) != 0 {
            SystemMemoryInfo {
                total_physical: mem_info.ullTotalPhys,
                avail_physical: mem_info.ullAvailPhys,
                total_pagefile: mem_info.ullTotalPageFile,
                avail_pagefile: mem_info.ullAvailPageFile,
            }
        } else {
            tracing::error!("Failed to get memory info");
            SystemMemoryInfo {
                total_physical: 0,
                avail_physical: 0,
                total_pagefile: 0,
                avail_pagefile: 0,
            }
        }
    }
}

#[cfg(not(windows))]
pub fn get_detailed_system_memory_info() -> SystemMemoryInfo {
    let mut sys = System::new_all();
    sys.refresh_memory();

    SystemMemoryInfo {
        total_physical: sys.total_memory(),
        avail_physical: sys.available_memory(),
        total_pagefile: sys.total_swap(),
        avail_pagefile: sys.free_swap(),
    }
}

// ============================================================================
// MEMORY MONITORING
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub timestamp: DateTime<Local>,
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub usage_percent: f32,
}

pub struct MemoryMonitor {
    history: Vec<MemorySnapshot>,
    max_history: usize,
}

impl MemoryMonitor {
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Vec::with_capacity(max_history),
            max_history,
        }
    }

    pub fn record_snapshot(&mut self) {
        let info = get_detailed_system_memory_info();
        
        let snapshot = MemorySnapshot {
            timestamp: Local::now(),
            total: info.total_physical,
            used: info.used_physical(),
            available: info.avail_physical,
            usage_percent: info.used_physical_percent(),
        };

        self.history.push(snapshot);
        
        // Keep only recent snapshots
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    pub fn get_history(&self) -> &[MemorySnapshot] {
        &self.history
    }

    pub fn average_usage(&self) -> f32 {
        if self.history.is_empty() {
            return 0.0;
        }
        
        let sum: f32 = self.history.iter().map(|s| s.usage_percent).sum();
        sum / self.history.len() as f32
    }

    pub fn peak_usage(&self) -> Option<&MemorySnapshot> {
        self.history.iter().max_by(|a, b| {
            a.usage_percent.partial_cmp(&b.usage_percent).unwrap()
        })
    }

    pub fn trend(&self) -> MemoryTrend {
        if self.history.len() < 2 {
            return MemoryTrend::Stable;
        }

        let recent = &self.history[self.history.len() - 5..];
        let avg_recent: f32 = recent.iter().map(|s| s.usage_percent).sum::<f32>() / recent.len() as f32;
        
        let older = &self.history[..self.history.len() - 5];
        let avg_older: f32 = older.iter().map(|s| s.usage_percent).sum::<f32>() / older.len() as f32;
        
        let diff = avg_recent - avg_older;
        
        if diff > 5.0 {
            MemoryTrend::Increasing
        } else if diff < -5.0 {
            MemoryTrend::Decreasing
        } else {
            MemoryTrend::Stable
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryTrend {
    Increasing,
    Decreasing,
    Stable,
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

pub fn format_bytes(bytes: usize) -> String {
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

pub fn should_clean_memory(threshold_percent: f32) -> bool {
    let info = get_detailed_system_memory_info();
    info.used_physical_percent() >= threshold_percent
}