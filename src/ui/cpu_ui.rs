use egui::{Ui, Color32};
use crate::cpu::{CpuLimiter, CpuPriority, ProcessCpuInfo};
use crate::ui::app::CleanRamApp;

// Add to CleanRamApp struct:
// pub cpu_limiter: Option<CpuLimiter>,
// pub cpu_search_text: String,
// pub cpu_selected_processes: HashSet<u32>,
// pub cpu_selected_priority: CpuPriority,
// pub cpu_selected_cores: Vec<bool>, // One per CPU core

pub fn draw_cpu_tab(app: &mut CleanRamApp, ui: &mut Ui) {
    ui.add_space(10.0);
    
    // Header
    ui.horizontal(|ui| {
        ui.label("⚙️");
        ui.heading("CPU Process Manager");
    });
    
    ui.separator();
    
    // Information panel
    ui.colored_label(Color32::from_rgb(33, 150, 243), "✅ CPU CONTROL ACTIVE");
    ui.label("• 🎯 Process priority management (Idle → Realtime)");
    ui.label("• 🔧 CPU core affinity control");
    ui.label("• 📊 Real-time CPU usage monitoring");
    ui.label("• 🔄 Automatic restoration of original settings");
    ui.separator();

    // Collect data first
    let (cpu_count, all_processes, has_limiter) = if let Some(ref limiter) = app.cpu_limiter {
        let cpu_count = limiter.get_cpu_count();
        let processes: Vec<_> = limiter.get_processes().iter().map(|p| (*p).clone()).collect();
        (cpu_count, processes, true)
    } else {
        (0, Vec::new(), false)
    };

    // System info
    if has_limiter {
        ui.horizontal(|ui| {
            ui.group(|ui| {
                ui.label(format!("💻 CPU Cores: {}", cpu_count));
            });
            
            ui.group(|ui| {
                ui.label(format!("📊 Processes: {}", all_processes.len()));
            });
            
            ui.group(|ui| {
                let selected_count = app.cpu_selected_processes.len();
                ui.label(format!("✅ Selected: {}", selected_count));
            });
        });
        
        ui.separator();
    }

    // Control buttons
    let mut scan_clicked = false;
    let mut restore_all_clicked = false;
    
    ui.horizontal(|ui| {
        if ui.button("🔄 Scan Processes").clicked() {
            scan_clicked = true;
        }
        
        if ui.button("🔓 Restore All").clicked() {
            restore_all_clicked = true;
        }
    });

    ui.separator();

    // Search
    ui.label("🔍 Search process:");
    ui.text_edit_singleline(&mut app.cpu_search_text);
    ui.add_space(5.0);

    // Filter processes
    let filtered_processes: Vec<_> = all_processes
        .iter()
        .filter(|process| {
            if app.cpu_search_text.is_empty() {
                true
            } else {
                process.name.to_lowercase().contains(&app.cpu_search_text.to_lowercase())
            }
        })
        .cloned()
        .collect();

    // Quick controls
    ui.horizontal(|ui| {
        ui.label("⚡ Quick priority:");
        
        let mut apply_priority_clicked = None;
        
        if ui.button("💤 Idle").clicked() {
            apply_priority_clicked = Some(CpuPriority::Idle);
        }
        if ui.button("📉 Below Normal").clicked() {
            apply_priority_clicked = Some(CpuPriority::BelowNormal);
        }
        if ui.button("⚖️ Normal").clicked() {
            apply_priority_clicked = Some(CpuPriority::Normal);
        }
        if ui.button("📈 Above Normal").clicked() {
            apply_priority_clicked = Some(CpuPriority::AboveNormal);
        }
        if ui.button("🔥 High").clicked() {
            apply_priority_clicked = Some(CpuPriority::High);
        }
        
        if let Some(priority) = apply_priority_clicked {
            if let Some(ref mut limiter) = app.cpu_limiter {
                for &pid in &app.cpu_selected_processes {
                    if let Err(e) = limiter.set_process_priority(pid, priority) {
                        tracing::error!("Failed to set priority for PID {}: {}", pid, e);
                    }
                }
            }
        }
    });

    ui.separator();

    // CPU Affinity control
    if has_limiter && !app.cpu_selected_processes.is_empty() {
        ui.collapsing("🔧 CPU Affinity Settings", |ui| {
            ui.label(format!("Select CPU cores for {} selected process(es):", app.cpu_selected_processes.len()));
            
            // Ensure vector is correct size
            while app.cpu_selected_cores.len() < cpu_count {
                app.cpu_selected_cores.push(true);
            }
            app.cpu_selected_cores.truncate(cpu_count);
            
            ui.horizontal_wrapped(|ui| {
                for core_idx in 0..cpu_count {
                    ui.checkbox(&mut app.cpu_selected_cores[core_idx], format!("Core {}", core_idx));
                }
            });
            
            if ui.button("Apply Affinity").clicked() {
                let selected_cores: Vec<usize> = app.cpu_selected_cores
                    .iter()
                    .enumerate()
                    .filter(|(_, &selected)| selected)
                    .map(|(idx, _)| idx)
                    .collect();
                
                if !selected_cores.is_empty() {
                    let mask = CpuLimiter::create_affinity_mask(&selected_cores);
                    
                    if let Some(ref mut limiter) = app.cpu_limiter {
                        for &pid in &app.cpu_selected_processes {
                            if let Err(e) = limiter.set_process_affinity(pid, mask) {
                                tracing::error!("Failed to set affinity for PID {}: {}", pid, e);
                            }
                        }
                    }
                } else {
                    tracing::warn!("No cores selected for affinity");
                }
            }
        });
        
        ui.separator();
    }

    // Selection controls
    ui.horizontal(|ui| {
        if ui.button("✅ Select All").clicked() {
            for process in &filtered_processes {
                app.cpu_selected_processes.insert(process.pid);
            }
        }
        if ui.button("❌ Deselect All").clicked() {
            app.cpu_selected_processes.clear();
        }
    });

    ui.separator();

    // Process list
    if !has_limiter {
        ui.colored_label(Color32::RED, "❌ CPU Limiter not initialized");
    } else if filtered_processes.is_empty() && app.cpu_search_text.is_empty() {
        ui.colored_label(Color32::YELLOW, "⚠️ No processes found. Click 'Scan Processes'");
    } else if filtered_processes.is_empty() {
        ui.colored_label(Color32::YELLOW, "🔍 No processes match your search");
    } else {
        ui.label("📊 Processes:");
        
        let mut actions: Vec<(u32, Option<CpuPriority>)> = Vec::new();
        
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                for process in &filtered_processes {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            // Selection checkbox
                            let mut selected = app.cpu_selected_processes.contains(&process.pid);
                            if ui.checkbox(&mut selected, "").changed() {
                                if selected {
                                    app.cpu_selected_processes.insert(process.pid);
                                } else {
                                    app.cpu_selected_processes.remove(&process.pid);
                                }
                            }
                            
                            // Process info
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(format!("📋 {} (PID: {})", process.name, process.pid));
                                    
                                    // Priority badge
                                    let priority_color = match process.priority {
                                        CpuPriority::Realtime => Color32::RED,
                                        CpuPriority::High => Color32::from_rgb(255, 140, 0),
                                        CpuPriority::AboveNormal => Color32::from_rgb(255, 215, 0),
                                        CpuPriority::Normal => Color32::GREEN,
                                        CpuPriority::BelowNormal => Color32::LIGHT_BLUE,
                                        CpuPriority::Idle => Color32::GRAY,
                                    };
                                    
                                    ui.colored_label(priority_color, format!("{:?}", process.priority));
                                });
                                
                                // CPU usage
                                ui.horizontal(|ui| {
                                    ui.label("💻 CPU:");
                                    let usage_color = if process.cpu_usage > 50.0 {
                                        Color32::RED
                                    } else if process.cpu_usage > 20.0 {
                                        Color32::YELLOW
                                    } else {
                                        Color32::GREEN
                                    };
                                    ui.colored_label(usage_color, format!("{:.1}%", process.cpu_usage));
                                    
                                    // Affinity
                                    if let Some(mask) = process.cpu_affinity {
                                        let cores = CpuLimiter::parse_affinity_mask(mask);
                                        ui.label(format!("🔧 Cores: {:?}", cores));
                                    }
                                });
                            });
                            
                            // Action buttons
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("🔄 Restore").clicked() {
                                    actions.push((process.pid, None));
                                }
                                
                                // Priority selector
                                egui::ComboBox::from_id_source(format!("priority_{}", process.pid))
                                    .selected_text("Set Priority")
                                    .show_ui(ui, |ui| {
                                        if ui.selectable_label(false, "💤 Idle").clicked() {
                                            actions.push((process.pid, Some(CpuPriority::Idle)));
                                        }
                                        if ui.selectable_label(false, "📉 Below Normal").clicked() {
                                            actions.push((process.pid, Some(CpuPriority::BelowNormal)));
                                        }
                                        if ui.selectable_label(false, "⚖️ Normal").clicked() {
                                            actions.push((process.pid, Some(CpuPriority::Normal)));
                                        }
                                        if ui.selectable_label(false, "📈 Above Normal").clicked() {
                                            actions.push((process.pid, Some(CpuPriority::AboveNormal)));
                                        }
                                        if ui.selectable_label(false, "🔥 High").clicked() {
                                            actions.push((process.pid, Some(CpuPriority::High)));
                                        }
                                        if ui.selectable_label(false, "⚠️ Realtime").clicked() {
                                            actions.push((process.pid, Some(CpuPriority::Realtime)));
                                        }
                                    });
                            });
                        });
                    });
                    ui.add_space(5.0);
                }
            });
        
        // Execute actions
        if let Some(ref mut limiter) = app.cpu_limiter {
            for (pid, priority_opt) in actions {
                if let Some(priority) = priority_opt {
                    if let Err(e) = limiter.set_process_priority(pid, priority) {
                        tracing::error!("Failed to set priority: {}", e);
                    }
                } else {
                    if let Err(e) = limiter.restore_process(pid) {
                        tracing::error!("Failed to restore: {}", e);
                    }
                }
            }
        }
    }

    ui.separator();
    ui.label("⚠️ Warning: Realtime priority can freeze your system if misused!");

    // Execute collected actions
    if scan_clicked {
        if let Some(ref mut limiter) = app.cpu_limiter {
            if let Err(e) = limiter.scan_processes() {
                tracing::error!("Failed to scan processes: {}", e);
            }
        }
    }
    
    if restore_all_clicked {
        if let Some(ref mut limiter) = app.cpu_limiter {
            if let Err(e) = limiter.restore_all() {
                tracing::error!("Failed to restore all: {}", e);
            }
        }
    }
}