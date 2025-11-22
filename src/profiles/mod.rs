// Gaming optimization profiles for popular games
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;
use crate::disk::DiskCleaningOptions;
use crate::cpu::CpuPriority;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamingProfile {
    pub id: String,
    pub name: String,
    pub game_executable: String, // e.g., "valorant.exe"
    pub description: String,
    pub icon: String, // Emoji
    
    // Memory settings
    pub auto_clean_ram: bool,
    pub ram_threshold: u8, // Percentage
    
    // Disk settings
    pub clean_disk_before_launch: bool,
    pub disk_options: DiskCleaningOptions,
    
    // CPU settings
    pub game_priority: CpuPriority,
    pub game_affinity: Option<Vec<usize>>, // Specific CPU cores
    
    // Background process settings
    pub limit_background_apps: bool,
    pub background_priority: CpuPriority,
    pub background_cpu_limit: Option<u32>, // Percentage
    
    // Network settings
    pub network_priority: bool,
    pub limit_background_network: bool,
    pub background_network_limit: Option<u32>, // KB/s
    
    // Services
    pub disable_defender: bool,
    pub disable_windows_update: bool,
    pub disable_superfetch: bool,
    
    // Display
    pub enable_game_mode: bool, // Windows Game Mode
    pub disable_fullscreen_optimizations: bool,
}

impl Default for GamingProfile {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            name: "Default Gaming Profile".to_string(),
            game_executable: "game.exe".to_string(),
            description: "Balanced gaming optimization".to_string(),
            icon: "🎮".to_string(),
            
            auto_clean_ram: true,
            ram_threshold: 85,
            
            clean_disk_before_launch: false,
            disk_options: DiskCleaningOptions::default(),
            
            game_priority: CpuPriority::High,
            game_affinity: None,
            
            limit_background_apps: true,
            background_priority: CpuPriority::BelowNormal,
            background_cpu_limit: Some(10),
            
            network_priority: true,
            limit_background_network: true,
            background_network_limit: Some(512), // 512 KB/s
            
            disable_defender: false,
            disable_windows_update: true,
            disable_superfetch: true,
            
            enable_game_mode: true,
            disable_fullscreen_optimizations: false,
        }
    }
}

impl GamingProfile {
    // Pre-configured profiles for popular games
    
    pub fn valorant() -> Self {
        Self {
            id: "valorant".to_string(),
            name: "Valorant".to_string(),
            game_executable: "VALORANT-Win64-Shipping.exe".to_string(),
            description: "Optimized for Valorant - Focus on CPU and network".to_string(),
            icon: "🎯".to_string(),
            
            auto_clean_ram: true,
            ram_threshold: 80,
            
            game_priority: CpuPriority::High,
            limit_background_apps: true,
            background_priority: CpuPriority::Idle,
            
            network_priority: true,
            limit_background_network: true,
            background_network_limit: Some(256),
            
            disable_windows_update: true,
            disable_superfetch: true,
            
            enable_game_mode: true,
            ..Default::default()
        }
    }
    
    pub fn league_of_legends() -> Self {
        Self {
            id: "league".to_string(),
            name: "League of Legends".to_string(),
            game_executable: "League of Legends.exe".to_string(),
            description: "Optimized for League of Legends".to_string(),
            icon: "⚔️".to_string(),
            
            auto_clean_ram: true,
            ram_threshold: 75,
            
            game_priority: CpuPriority::High,
            
            network_priority: true,
            limit_background_network: true,
            background_network_limit: Some(512),
            
            enable_game_mode: true,
            ..Default::default()
        }
    }
    
    pub fn cs2() -> Self {
        Self {
            id: "cs2".to_string(),
            name: "Counter-Strike 2".to_string(),
            game_executable: "cs2.exe".to_string(),
            description: "Optimized for CS2 - Maximum performance".to_string(),
            icon: "🔫".to_string(),
            
            auto_clean_ram: true,
            ram_threshold: 85,
            
            game_priority: CpuPriority::High,
            game_affinity: Some(vec![0, 1, 2, 3]), // First 4 cores
            
            limit_background_apps: true,
            background_priority: CpuPriority::Idle,
            background_cpu_limit: Some(5),
            
            network_priority: true,
            limit_background_network: true,
            background_network_limit: Some(128),
            
            disable_windows_update: true,
            disable_superfetch: true,
            
            enable_game_mode: true,
            disable_fullscreen_optimizations: true,
            ..Default::default()
        }
    }
    
    pub fn apex_legends() -> Self {
        Self {
            id: "apex".to_string(),
            name: "Apex Legends".to_string(),
            game_executable: "r5apex.exe".to_string(),
            description: "Optimized for Apex Legends".to_string(),
            icon: "🏆".to_string(),
            
            auto_clean_ram: true,
            ram_threshold: 80,
            
            game_priority: CpuPriority::High,
            
            limit_background_apps: true,
            background_priority: CpuPriority::BelowNormal,
            
            network_priority: true,
            limit_background_network: true,
            background_network_limit: Some(256),
            
            enable_game_mode: true,
            ..Default::default()
        }
    }
    
    pub fn fortnite() -> Self {
        Self {
            id: "fortnite".to_string(),
            name: "Fortnite".to_string(),
            game_executable: "FortniteClient-Win64-Shipping.exe".to_string(),
            description: "Optimized for Fortnite".to_string(),
            icon: "🏗️".to_string(),
            
            auto_clean_ram: true,
            ram_threshold: 75,
            
            game_priority: CpuPriority::High,
            
            limit_background_apps: true,
            background_priority: CpuPriority::BelowNormal,
            
            network_priority: true,
            
            enable_game_mode: true,
            ..Default::default()
        }
    }
    
    pub fn overwatch2() -> Self {
        Self {
            id: "overwatch2".to_string(),
            name: "Overwatch 2".to_string(),
            game_executable: "Overwatch.exe".to_string(),
            description: "Optimized for Overwatch 2".to_string(),
            icon: "🎮".to_string(),
            
            auto_clean_ram: true,
            ram_threshold: 80,
            
            game_priority: CpuPriority::High,
            
            network_priority: true,
            limit_background_network: true,
            background_network_limit: Some(256),
            
            enable_game_mode: true,
            ..Default::default()
        }
    }
    
    pub fn get_all_presets() -> Vec<Self> {
        vec![
            Self::valorant(),
            Self::league_of_legends(),
            Self::cs2(),
            Self::apex_legends(),
            Self::fortnite(),
            Self::overwatch2(),
        ]
    }
}

pub struct ProfileManager {
    profiles: HashMap<String, GamingProfile>,
    active_profile: Option<String>,
}

impl ProfileManager {
    pub fn new() -> Self {
        let mut profiles = HashMap::new();
        
        // Load presets
        for preset in GamingProfile::get_all_presets() {
            profiles.insert(preset.id.clone(), preset);
        }
        
        Self {
            profiles,
            active_profile: None,
        }
    }
    
    pub fn add_profile(&mut self, profile: GamingProfile) {
        self.profiles.insert(profile.id.clone(), profile);
    }
    
    pub fn get_profile(&self, id: &str) -> Option<&GamingProfile> {
        self.profiles.get(id)
    }
    
    pub fn get_all_profiles(&self) -> Vec<&GamingProfile> {
        self.profiles.values().collect()
    }
    
    pub fn activate_profile(&mut self, id: &str) -> Result<()> {
        if !self.profiles.contains_key(id) {
            return Err(anyhow::anyhow!("Profile not found: {}", id));
        }
        
        self.active_profile = Some(id.to_string());
        tracing::info!("✅ Activated gaming profile: {}", id);
        Ok(())
    }
    
    pub fn deactivate_profile(&mut self) {
        self.active_profile = None;
        tracing::info!("🔄 Deactivated gaming profile");
    }
    
    pub fn get_active_profile(&self) -> Option<&GamingProfile> {
        if let Some(ref id) = self.active_profile {
            self.profiles.get(id)
        } else {
            None
        }
    }
    
    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.profiles)?;
        std::fs::write(path, json)?;
        Ok(())
    }
    
    pub fn load_from_file(&mut self, path: &str) -> Result<()> {
        if std::path::Path::new(path).exists() {
            let json = std::fs::read_to_string(path)?;
            let loaded: HashMap<String, GamingProfile> = serde_json::from_str(&json)?;
            
            // Merge with existing profiles (presets + custom)
            for (id, profile) in loaded {
                self.profiles.insert(id, profile);
            }
        }
        Ok(())
    }
}