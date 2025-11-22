// CPU management module for process priority and affinity control
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use sysinfo::{System, Pid, ProcessExt};

#[cfg(target_os = "windows")]
use windows_sys::Win32::{
    Foundation::{HANDLE, CloseHandle},
    System::Threading::{
        OpenProcess, SetPriorityClass, GetPriorityClass, SetProcessAffinityMask,
        GetProcessAffinityMask, PROCESS_SET_INFORMATION, PROCESS_QUERY_INFORMATION,
        IDLE_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS,
        ABOVE_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS, REALTIME_PRIORITY_CLASS,
    },
};

/// CPU priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CpuPriority {
    Idle,
    BelowNormal,
    Normal,
    AboveNormal,
    High,
    Realtime, // Use with caution!
}

impl CpuPriority {
    #[cfg(target_os = "windows")]
    fn to_windows_priority(&self) -> u32 {
        match self {
            CpuPriority::Idle => IDLE_PRIORITY_CLASS,
            CpuPriority::BelowNormal => BELOW_NORMAL_PRIORITY_CLASS,
            CpuPriority::Normal => NORMAL_PRIORITY_CLASS,
            CpuPriority::AboveNormal => ABOVE_NORMAL_PRIORITY_CLASS,
            CpuPriority::High => HIGH_PRIORITY_CLASS,
            CpuPriority::Realtime => REALTIME_PRIORITY_CLASS,
        }
    }

    #[cfg(target_os = "windows")]
    fn from_windows_priority(priority: u32) -> Self {
        match priority {
            IDLE_PRIORITY_CLASS => CpuPriority::Idle,
            BELOW_NORMAL_PRIORITY_CLASS => CpuPriority::BelowNormal,
            ABOVE_NORMAL_PRIORITY_CLASS => CpuPriority::AboveNormal,
            HIGH_PRIORITY_CLASS => CpuPriority::High,
            REALTIME_PRIORITY_CLASS => CpuPriority::Realtime,
            _ => CpuPriority::Normal,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            CpuPriority::Idle => "Idle - Lowest priority, runs only when system is idle",
            CpuPriority::BelowNormal => "Below Normal - Lower than normal priority",
            CpuPriority::Normal => "Normal - Standard priority",
            CpuPriority::AboveNormal => "Above Normal - Higher than normal priority",
            CpuPriority::High => "High - High priority, may affect system responsiveness",
            CpuPriority::Realtime => "Realtime - Highest priority, DANGEROUS if misused!",
        }
    }
}

/// Information about a process CPU usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessCpuInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32, // Percentage
    pub priority: CpuPriority,
    pub cpu_affinity: Option<usize>, // CPU core mask
    pub thread_count: usize,
}

/// CPU Limiter for managing process priorities and affinity
pub struct CpuLimiter {
    system: System,
    processes: HashMap<u32, ProcessCpuInfo>,
    modified_processes: HashMap<u32, (CpuPriority, Option<usize>)>, // Original values
}

impl CpuLimiter {
    pub fn new() -> Result<Self> {
        Ok(Self {
            system: System::new_all(),
            processes: HashMap::new(),
            modified_processes: HashMap::new(),
        })
    }

    /// Scan all processes and get their CPU information
    pub fn scan_processes(&mut self) -> Result<()> {
        self.system.refresh_all();
        self.processes.clear();

        for (pid, process) in self.system.processes() {
            let pid_u32 = pid.as_u32();

            // Skip system processes
            if pid_u32 <= 4 {
                continue;
            }

            let cpu_usage = process.cpu_usage();
            let name = process.name().to_string();

            // Get current priority
            let priority = self.get_process_priority(pid_u32).unwrap_or(CpuPriority::Normal);

            // Get affinity
            let affinity = self.get_process_affinity(pid_u32).ok();

            let info = ProcessCpuInfo {
                pid: pid_u32,
                name,
                cpu_usage,
                priority,
                cpu_affinity: affinity,
                thread_count: 0, // Could be enhanced
            };

            self.processes.insert(pid_u32, info);
        }

        Ok(())
    }

    /// Set process priority
    #[cfg(target_os = "windows")]
    pub fn set_process_priority(&mut self, pid: u32, priority: CpuPriority) -> Result<()> {
        // Save original priority if not already saved
        if !self.modified_processes.contains_key(&pid) {
            let original_priority = self.get_process_priority(pid)?;
            let original_affinity = self.get_process_affinity(pid).ok();
            self.modified_processes.insert(pid, (original_priority, original_affinity));
        }

        unsafe {
            let handle = OpenProcess(PROCESS_SET_INFORMATION, 0, pid);
            if handle == 0 {
                return Err(anyhow!("Failed to open process {}", pid));
            }

            let result = SetPriorityClass(handle, priority.to_windows_priority());
            CloseHandle(handle);

            if result == 0 {
                return Err(anyhow!("Failed to set priority for process {}", pid));
            }
        }

        // Update cached info
        if let Some(info) = self.processes.get_mut(&pid) {
            info.priority = priority;
        }

        tracing::info!("✅ Set priority {:?} for PID {}", priority, pid);
        Ok(())
    }

    /// Get process priority
    #[cfg(target_os = "windows")]
    pub fn get_process_priority(&self, pid: u32) -> Result<CpuPriority> {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_INFORMATION, 0, pid);
            if handle == 0 {
                return Err(anyhow!("Failed to open process {}", pid));
            }

            let priority_class = GetPriorityClass(handle);
            CloseHandle(handle);

            if priority_class == 0 {
                return Err(anyhow!("Failed to get priority for process {}", pid));
            }

            Ok(CpuPriority::from_windows_priority(priority_class))
        }
    }

    /// Set CPU affinity (which cores the process can use)
    #[cfg(target_os = "windows")]
    pub fn set_process_affinity(&mut self, pid: u32, core_mask: usize) -> Result<()> {
        // Save original if not saved
        if !self.modified_processes.contains_key(&pid) {
            let original_priority = self.get_process_priority(pid)?;
            let original_affinity = self.get_process_affinity(pid).ok();
            self.modified_processes.insert(pid, (original_priority, original_affinity));
        }

        unsafe {
            let handle = OpenProcess(PROCESS_SET_INFORMATION, 0, pid);
            if handle == 0 {
                return Err(anyhow!("Failed to open process {}", pid));
            }

            let result = SetProcessAffinityMask(handle, core_mask);
            CloseHandle(handle);

            if result == 0 {
                return Err(anyhow!("Failed to set affinity for process {}", pid));
            }
        }

        if let Some(info) = self.processes.get_mut(&pid) {
            info.cpu_affinity = Some(core_mask);
        }

        tracing::info!("✅ Set CPU affinity 0x{:X} for PID {}", core_mask, pid);
        Ok(())
    }

    /// Get CPU affinity
    #[cfg(target_os = "windows")]
    pub fn get_process_affinity(&self, pid: u32) -> Result<usize> {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_INFORMATION, 0, pid);
            if handle == 0 {
                return Err(anyhow!("Failed to open process {}", pid));
            }

            let mut process_mask: usize = 0;
            let mut system_mask: usize = 0;

            let result = GetProcessAffinityMask(handle, &mut process_mask, &mut system_mask);
            CloseHandle(handle);

            if result == 0 {
                return Err(anyhow!("Failed to get affinity for process {}", pid));
            }

            Ok(process_mask)
        }
    }

    /// Restore original priority and affinity for a process
    #[cfg(target_os = "windows")]
    pub fn restore_process(&mut self, pid: u32) -> Result<()> {
        if let Some((original_priority, original_affinity)) = self.modified_processes.remove(&pid) {
            // Restore priority
            self.set_process_priority(pid, original_priority)?;

            // Restore affinity if it was saved
            if let Some(affinity) = original_affinity {
                self.set_process_affinity(pid, affinity)?;
            }

            tracing::info!("✅ Restored original settings for PID {}", pid);
        }

        Ok(())
    }

    /// Restore all modified processes
    pub fn restore_all(&mut self) -> Result<()> {
        let pids: Vec<u32> = self.modified_processes.keys().copied().collect();
        
        for pid in pids {
            if let Err(e) = self.restore_process(pid) {
                tracing::warn!("Failed to restore PID {}: {}", pid, e);
            }
        }

        Ok(())
    }

    /// Get all processes
    pub fn get_processes(&self) -> Vec<&ProcessCpuInfo> {
        self.processes.values().collect()
    }

    /// Get system CPU count
    pub fn get_cpu_count(&self) -> usize {
        num_cpus::get()
    }

    /// Create CPU affinity mask for specific cores
    /// Example: cores = vec![0, 2, 4] -> mask for cores 0, 2, 4
    pub fn create_affinity_mask(cores: &[usize]) -> usize {
        let mut mask = 0usize;
        for &core in cores {
            mask |= 1 << core;
        }
        mask
    }

    /// Parse affinity mask to core list
    pub fn parse_affinity_mask(mask: usize) -> Vec<usize> {
        let mut cores = Vec::new();
        for i in 0..64 {
            if (mask & (1 << i)) != 0 {
                cores.push(i);
            }
        }
        cores
    }
}

#[cfg(not(target_os = "windows"))]
impl CpuLimiter {
    pub fn set_process_priority(&mut self, _pid: u32, _priority: CpuPriority) -> Result<()> {
        Err(anyhow!("CPU priority control is only supported on Windows"))
    }

    pub fn get_process_priority(&self, _pid: u32) -> Result<CpuPriority> {
        Err(anyhow!("CPU priority control is only supported on Windows"))
    }

    pub fn set_process_affinity(&mut self, _pid: u32, _core_mask: usize) -> Result<()> {
        Err(anyhow!("CPU affinity control is only supported on Windows"))
    }

    pub fn get_process_affinity(&self, _pid: u32) -> Result<usize> {
        Err(anyhow!("CPU affinity control is only supported on Windows"))
    }

    pub fn restore_process(&mut self, _pid: u32) -> Result<()> {
        Ok(())
    }
}

// Helper function to format CPU usage
pub fn format_cpu_usage(usage: f32) -> String {
    format!("{:.1}%", usage)
}