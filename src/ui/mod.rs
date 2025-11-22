use crate::memory::{
    clean_memory, get_detailed_system_memory_info, CleaningResults, 
    format_bytes, MemoryMonitor, MemoryTrend,
};
use crate::theme::Theme;
use crate::ui::app::CleanRamApp;
use eframe::egui::{self, Layout, RichText, ProgressBar, Color32};
use poll_promise::Promise;

// Add these to CleanRamApp:
// pub memory_monitor: MemoryMonitor,
// pub show_memory_details: bool,

fn bytes_to_gb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0 / 1024.0
}

pub fn draw_memory_tab(app: &mut CleanRamApp, ui: &mut egui::Ui, _theme: &Theme) {
    let mem_info = get_detailed_system_memory_info();

    // Update RAM usage and monitor
    if app.cleaning_promise.is_none() {
        app.ram_usage = mem_info.used_physical_percent();
        // app.memory_monitor.record_snapshot(); // If implemented
    }

    ui.vertical_centered(|ui| {
        ui.add_space(10.0);
        ui.heading("💾 Memory Optimization");
        ui.add_space(10.0);
    });
    
    ui.separator();
    ui.add_space(10.0);

    // === MAIN MEMORY STATUS ===
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading("Physical RAM");
                ui.add_space(5.0);

                let used_gb = bytes_to_gb(mem_info.used_physical());
                let total_gb = bytes_to_gb(mem_info.total_physical);
                let usage_percent = mem_info.used_physical_percent() / 100.0;
                
                ui.label(format!("Usage: {:.2} GB / {:.2} GB", used_gb, total_gb));

                // Color-coded progress bar
                let bar_color = if usage_percent > 0.9 {
                    Color32::from_rgb(220, 38, 38) // Red
                } else if usage_percent > 0.75 {
                    Color32::from_rgb(251, 191, 36) // Yellow
                } else {
                    Color32::from_rgb(34, 197, 94) // Green
                };

                let progress_bar = ProgressBar::new(usage_percent)
                    .show_percentage()
                    .text(format!("{:.1}%", usage_percent * 100.0));
                
                ui.add(progress_bar);
                
                // Available memory highlight
                ui.horizontal(|ui| {
                    ui.colored_label(
                        Color32::from_rgb(34, 197, 94),
                        format!("Available: {:.2} GB", bytes_to_gb(mem_info.avail_physical))
                    );
                });
            });

            ui.separator();

            // === VIRTUAL MEMORY (PAGEFILE) ===
            ui.vertical(|ui| {
                ui.heading("Virtual Memory");
                ui.add_space(5.0);

                let used_pf = mem_info.used_pagefile();
                let total_pf = mem_info.total_pagefile;
                let pf_usage = mem_info.pagefile_usage_percent() / 100.0;

                ui.label(format!("Usage: {:.2} GB / {:.2} GB", 
                    bytes_to_gb(used_pf), 
                    bytes_to_gb(total_pf)
                ));

                let pf_bar = ProgressBar::new(pf_usage)
                    .show_percentage()
                    .text(format!("{:.1}%", pf_usage * 100.0));
                
                ui.add(pf_bar);
            });
        });
    });

    ui.add_space(15.0);

    // === QUICK STATS ===
    ui.horizontal(|ui| {
        ui.group(|ui| {
            ui.label("📊 Status");
            let status_color = if mem_info.used_physical_percent() > 85.0 {
                Color32::RED
            } else if mem_info.used_physical_percent() > 70.0 {
                Color32::YELLOW
            } else {
                Color32::GREEN
            };
            
            let status_text = if mem_info.used_physical_percent() > 85.0 {
                "⚠️ HIGH"
            } else if mem_info.used_physical_percent() > 70.0 {
                "⚡ MEDIUM"
            } else {
                "✅ GOOD"
            };
            
            ui.colored_label(status_color, status_text);
        });

        // Memory trend (if monitor is available)
        /*
        ui.group(|ui| {
            ui.label("📈 Trend");
            let trend = app.memory_monitor.trend();
            let (trend_text, trend_color) = match trend {
                MemoryTrend::Increasing => ("↗️ Increasing", Color32::RED),
                MemoryTrend::Decreasing => ("↘️ Decreasing", Color32::GREEN),
                MemoryTrend::Stable => ("→ Stable", Color32::GRAY),
            };
            ui.colored_label(trend_color, trend_text);
        });
        */
    });

    ui.add_space(20.0);

    // === CLEAN BUTTON ===
    ui.with_layout(Layout::top_down(egui::Align::Center), |ui| {
        let button_size = egui::vec2(250.0, 50.0);
        let is_cleaning = app.cleaning_promise.is_some();
        
        let button_text = if is_cleaning {
            "🔄 Cleaning..."
        } else {
            "🧹 Clean Memory Cache"
        };
        
        let clean_button = egui::Button::new(RichText::new(button_text).size(16.0))
            .min_size(button_size);

        ui.add_enabled(!is_cleaning, clean_button)
            .on_hover_text("Optimize memory by cleaning process working sets")
            .clicked()
            .then(|| {
                let promise = Promise::spawn_thread("memory_clean", || {
                    match clean_memory() {
                        Ok(results) => results,
                        Err(e) => {
                            let mut error_results = CleaningResults::new();
                            error_results.has_error = true;
                            error_results.error_message = format!("Cleaning error: {}", e);
                            error_results.is_completed = true;
                            error_results.end_time = Some(chrono::Local::now());
                            error_results
                        }
                    }
                });
                app.cleaning_promise = Some(promise);
            });

        if is_cleaning {
            ui.add_space(10.0);
            ui.spinner();
            ui.label("Optimizing memory usage...");
            ui.ctx().request_repaint();
        }
    });

    // === HANDLE PROMISE ===
    if let Some(promise) = &app.cleaning_promise {
        if let Some(results) = promise.ready() {
            app.last_cleaned_results = Some(results.clone());
            app.cleaning_promise = None;
        }
    }
    
    // === CLEANING RESULTS ===
    if let Some(results) = &app.last_cleaned_results {
        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);
        
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("📋 Cleaning Results").size(18.0).strong());
        });

        ui.add_space(10.0);

        if results.has_error {
            ui.colored_label(Color32::RED, &results.error_message);
        } else {
            // === SUMMARY ===
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Memory Freed:");
                        ui.label(RichText::new(format_bytes(results.total_freed()))
                            .size(24.0)
                            .color(Color32::from_rgb(34, 197, 94))
                            .strong());
                    });

                    ui.separator();

                    ui.vertical(|ui| {
                        ui.label("Processes:");
                        ui.label(RichText::new(format!("{} / {}", 
                            results.processes_succeeded, 
                            results.processes_attempted
                        )).size(20.0).strong());
                        ui.label(format!("Success: {:.1}%", results.success_rate()));
                    });

                    ui.separator();

                    ui.vertical(|ui| {
                        ui.label("Duration:");
                        if let Some(ms) = results.duration_ms {
                            ui.label(RichText::new(format!("{:.2}s", ms as f64 / 1000.0))
                                .size(20.0)
                                .strong());
                        }
                    });
                });
            });

            // === DETAILED PROCESS LIST ===
            if !results.processes.is_empty() {
                ui.add_space(15.0);
                
                ui.collapsing(
                    format!("📊 Detailed Report ({} processes)", results.processes.len()), 
                    |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(300.0)
                            .show(ui, |ui| {
                                egui::Grid::new("process_grid")
                                    .striped(true)
                                    .spacing([10.0, 8.0])
                                    .show(ui, |ui| {
                                        // Header
                                        ui.label(RichText::new("Process").strong());
                                        ui.label(RichText::new("PID").strong());
                                        ui.label(RichText::new("Before").strong());
                                        ui.label(RichText::new("After").strong());
                                        ui.label(RichText::new("Freed").strong());
                                        ui.end_row();

                                        // Data rows
                                        for process in &results.processes {
                                            // Process name
                                            ui.label(&process.name);
                                            
                                            // PID
                                            ui.label(process.pid.to_string());
                                            
                                            // Before
                                            ui.label(format_bytes(process.memory_before));
                                            
                                            // After
                                            ui.label(format_bytes(process.memory_after));
                                            
                                            // Freed (highlighted)
                                            ui.colored_label(
                                                Color32::from_rgb(34, 197, 94),
                                                format_bytes(process.memory_freed)
                                            );
                                            
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                );
            }

            // === TOP PROCESSES ===
            if results.processes.len() > 3 {
                ui.add_space(10.0);
                ui.label(RichText::new("🏆 Top Memory Freed:").strong());
                
                for (i, process) in results.processes.iter().take(3).enumerate() {
                    ui.horizontal(|ui| {
                        let medal = match i {
                            0 => "🥇",
                            1 => "🥈",
                            2 => "🥉",
                            _ => "•",
                        };
                        ui.label(medal);
                        ui.label(&process.name);
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.colored_label(
                                Color32::from_rgb(34, 197, 94),
                                format_bytes(process.memory_freed)
                            );
                        });
                    });
                }
            }
        }
    }

    // === HELP SECTION ===
    ui.add_space(20.0);
    ui.collapsing("ℹ️ How It Works", |ui| {
        ui.label("Memory cleaning works by:");
        ui.label("• Emptying the working set of processes");
        ui.label("• Releasing unused memory pages back to the system");
        ui.label("• NOT terminating any processes");
        ui.label("• Keeping critical system processes safe");
        ui.add_space(5.0);
        ui.colored_label(Color32::YELLOW, "⚠️ Note: Memory will be reused by processes as needed");
    });
}