#![warn(clippy::all)]
#![windows_subsystem = "windows"]

use bytesize::ByteSize;
use dirs;
use diskspace_insight;
use diskspace_insight::{DirInfo, Directory, File};
use eframe::egui::Color32;
// use egui::paint::color::Srgba;
// use egui::{paint::PaintCmd, Button, Checkbox, Label, Slider, Style, TextStyle, Ui, Window, Stroke};
// use egui_glium::storage::FileStorage;
use env_logger;
use log::*;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::path::Path;

use eframe::{
    egui::{self, Ui, Button, Checkbox, Label, Slider, Style, TextStyle, Window, Stroke, color::Rgba, paint::Shape},
    epi,
};

#[cfg(test)]
mod tests;

struct MyApp {
    scan_path: String,
    max_types: i32,
    max_files: i32,
    max_dirs: i32,
    info: DirInfo,
    allow_delete: bool,
    filter_chain: Vec<Filter>,
    dirinfo_receiver: Receiver<DirInfo>,
    dirinfo_sender: Sender<DirInfo>,
    ready_receiver: Receiver<bool>,
    ready_sender: Sender<bool>,
    del_receiver: Receiver<PathBuf>,
    del_sender: Sender<PathBuf>,

    ready: bool,
}

impl Default for MyApp {
    fn default() -> MyApp {
        let (s, r): (Sender<DirInfo>, Receiver<DirInfo>) = channel();
        let (bs, br): (Sender<bool>, Receiver<bool>) = channel();
        let (ds, dr): (Sender<PathBuf>, Receiver<PathBuf>) = channel();
        MyApp {
            scan_path: String::default(),
            max_types: 10,
            max_files: 10,
            max_dirs: 10,
            info: DirInfo::new(),
            allow_delete: false,
            filter_chain: vec![],
            dirinfo_receiver: r,
            dirinfo_sender: s,
            ready_receiver: br,
            ready_sender: bs,
            del_receiver: dr,
            del_sender: ds,
            ready: true,
        }
    }
}

#[derive(Debug, PartialEq)]
enum Filter {
    MinAge(i32),
    MaxAge(i32),
    MinSize(i32),
    MaxResults(i32),
}

fn draw_file(ui: &mut Ui, file: &File, allow_delete: bool, del_sender: Sender<PathBuf>) {
    ui.horizontal(|ui| {
        // ui.label(format!("{:<10}MB", file.size / 1024 / 1024));
        ui.add(Label::new(format!("{}", ByteSize(file.size))).text_style(TextStyle::Monospace));
        // ui.expand_to_size(egui::math::Vec2::new(100.,10.));
        if allow_delete {
            if ui.button("Del").clicked() {
                let _ = std::fs::remove_file(&file.path);
                let _ = del_sender.send(file.path.to_path_buf());
            }
        }
        ui.label(format!("{}", file.path.display()));
    });
}

fn draw_dir(
    ui: &mut Ui,
    dir: &Directory,
    info: &DirInfo,
    allow_delete: bool,
    accent_color: Color32,
    del_sender: Sender<PathBuf>,
) {
    let scale = dir.combined_size as f32 / info.combined_size as f32;

    paint_size_bar_before_next(ui, scale, accent_color);

    // Sort subdirs
    ui.collapsing(
        format!(
            "{} | {} | {}%",
            dir.path
                .file_name()
                .map(|d| d.to_string_lossy().to_string())
                .unwrap_or_default(),
            ByteSize(dir.combined_size),
            (scale * 100.) as u8
        ),
        |ui| {
            if allow_delete {
                if ui
                    .button(&format!(
                        "Del {}",
                        dir.path
                            .file_name()
                            .map(|d| d.to_string_lossy().to_string())
                            .unwrap_or_default(),
                    ))
                    .clicked()
                {
                    let _ = std::fs::remove_dir_all(&dir.path);
                    let _ = del_sender.send(dir.path.to_path_buf());
                }
            }
            for subdir in &dir.sorted_subdirs(info) {
                draw_dir(
                    ui,
                    subdir,
                    info,
                    allow_delete,
                    accent_color,
                    del_sender.clone(),
                );
            }

            for (i, file) in dir.sorted_files().iter().enumerate() {
                if i as i32 > 10 {
                    break;
                }
                draw_file(ui, file, allow_delete, del_sender.clone());
            }
        },
    );
}

fn gen_light_style() -> Style {
    let mut style = Style::default();
    style.visuals.window_corner_radius = 0.;
    style
}

fn paint_size_bar_before_next(ui: &mut Ui, scale: f32, color: Color32) {
    // ask for available space - this is just to get the cursor
    // let mut paint_rect = ui.available();
    let mut paint_rect = ui.available_rect_before_wrap();
    paint_rect.max.y = paint_rect.min.y + ui.style().spacing.interact_size.y + 2.;
    paint_rect.max.x = paint_rect.min.x + ui.available_rect_before_wrap().size().x * scale;

    ui.painter().add(Shape::Rect {
        rect: paint_rect,
        corner_radius: 2.,
        fill: color,
        stroke: Stroke::default(),
    });
}

fn get_dirinfo(path: &String, sender: Sender<DirInfo>, ready: Sender<bool>) {
    let s = sender.clone();
    let r = ready.clone();
    let p = path.clone();

    thread::spawn(move || {
        let timer = std::time::Instant::now();
        if Path::new(&p).is_dir() {
            let final_info = diskspace_insight::scan_callback(
                &p,
                |d| {
                    let _ = s.send(d.clone());
                },
                2000,
            );
    
            // let final_info = diskspace_insight::scan(&p);
    
            let _ = s.send(final_info);
            let _ = r.send(true);


        } else {
            let final_info = diskspace_insight::scan_archive(
                &p
            );
    
    
            let _ = s.send(final_info);
            let _ = r.send(true);
        }

        println!("Done scanning in {} s", timer.elapsed().as_secs_f32());
    });
}

impl epi::App for MyApp {

    fn name(&self) -> &str {
        "egui template"
    }

    /// This function will be called whenever the Ui needs to be shown,
    /// which may be many times per second.
    fn update(
        &mut self,
        // ui: &mut egui::Ui,
        ctx: &egui::CtxRef,
        frame: &mut epi::Frame<'_>,
        // _info: &egui::app::BackendInfo,
        // _tex_allocator: Option<&mut dyn egui::app::TextureAllocator>,
    ) {
        let accent_color = Color32::from_rgb(120, 50, 200);

        // ui.style_mut().visuals.ui(ui);

        // ui.style_mut().visuals.dark_bg_color

        let MyApp {
            scan_path,
            max_types,
            max_files,
            max_dirs,
            info,
            allow_delete,
            filter_chain,
            dirinfo_receiver,
            dirinfo_sender,
            ready_receiver,
            ready_sender,
            del_receiver,
            del_sender,
            ready,
        } = self;

        egui::CentralPanel::default().show(ctx, |ui| {
            if !*ready {
                ui.ctx().request_repaint();
            }

            // ui.ctx().request_repaint();

            while let Ok(r_info) = dirinfo_receiver.try_recv() {
                *info = r_info;
                // ui.ctx().request_repaint();
            }

            while let Ok(_) = ready_receiver.try_recv() {
                // dbg!("Got RDY");
                *ready = true;
            }

            while let Ok(path) = del_receiver.try_recv() {
                dbg!("Got del", &path);
                info.tree.remove_entry(&path);
                info.files.retain(|x| x.path != path);
                info.dirs_by_size = info.dirs_by_size();
                info.files_by_size = info.files_by_size();
            }

            ui.set_style(gen_light_style());
            ui.style_mut().visuals.window_corner_radius = 1.;
            ui.style_mut().visuals.faint_bg_color = Color32::new(100, 0, 100, 255);
            ui.style_mut().visuals.widgets.active.corner_radius = 0.;
            //ui.style_mut().interact(ui.ctx().response());
            Window::new("Setup").show(ui.ctx(), |ui| {
                // ui.ctx().settings_ui(ui);

                ui.horizontal(|ui| {
                    if ui.button("Home").clicked() {
                        if let Some(dir) = dirs::home_dir() {
                            *scan_path = dir.to_string_lossy().to_string();
                        }
                    }
                    if ui.button("Downloads").clicked() {
                        if let Some(dir) = dirs::download_dir() {
                            *scan_path = dir.to_string_lossy().to_string();
                        }
                    }
                    if ui.button("Videos").clicked() {
                        if let Some(dir) = dirs::video_dir() {
                            *scan_path = dir.to_string_lossy().to_string();
                        }
                    }
                    if ui.button("Cache").clicked() {
                        if let Some(dir) = dirs::cache_dir() {
                            *scan_path = dir.to_string_lossy().to_string();
                        }
                    }
                    if ui.button("Temp").clicked() {
                        *scan_path = std::env::temp_dir().to_string_lossy().to_string();
                    }
                });

                ui.text_edit(scan_path);

                // ui.checkbox("Allow deletion", allow_delete);
                // ui.checkbox(allow_delete, allow_delete);
                ui.add(Checkbox::new(allow_delete, "Allow deletion"));

                if *ready {
                    if ui.button("Scan").clicked() {
                        *ready = false;
                        let s = dirinfo_sender.clone();
                        let r = ready_sender.clone();
                        get_dirinfo(&scan_path, s, r);
                        *info = DirInfo::new();
                        // The update loop only happens on repaint, so we need to
                        // make sure we do one next frame
                        ui.ctx().request_repaint();
                    }
                } else {
                    ui.label(format!("Scanned {} files...", info.files.len()));
                }
            });

            Window::new("Filetypes").scroll(true).show(ui.ctx(), |ui| {
                ui.label(format!("Files by type, largest first"));
                ui.add(Slider::i32(max_types, 1..=100).text("max results"));
                //ui.painter().rect_filled(Rect::from_min_max(pos2(0., 0.), pos2(100., 100.)), 2., Srgba::new(255,0,255, 255));
                // let visuals = ui.style().interact(&response);

                if !*ready {
                    ui.label(format!("Please wait for scan"));
                }
                for (i, filetype) in info.types_by_size.iter().enumerate() {
                    if i as i32 >= *max_types {
                        break;
                    }

                    let scale = filetype.size as f32 / info.combined_size as f32;
                    paint_size_bar_before_next(ui, scale, accent_color);

                    ui.collapsing(
                        format!(
                            "{} | {} | {}% | {} files",
                            filetype.ext,
                            ByteSize(filetype.size),
                            (scale * 100.) as u8,
                            filetype.files.len()
                        ),
                        |ui| {
                            for file in &filetype.files {
                                let s = del_sender.clone();
                                draw_file(ui, file, *allow_delete, s);
                            }
                        },
                    );
                }
            });

            Window::new("Files").scroll(true).show(ui.ctx(), |ui| {
                ui.label(format!("Files by size, largest first"));
                ui.add(Slider::i32(max_files, 1..=100).text("max results"));

                for (i, file) in info.files_by_size.iter().enumerate() {
                    if i as i32 >= *max_files {
                        break;
                    }
                    let s = del_sender.clone();

                    draw_file(ui, file, *allow_delete, s);
                }
            });

            Window::new("Largest directories")
                .scroll(true)
                .show(ui.ctx(), |ui| {
                    ui.label(format!("Largest individual directories"));
                    ui.add(Slider::i32(max_dirs, 1..=100).text("max results"));

                    for (i, dir) in info.dirs_by_size.iter().enumerate() {
                        if i as i32 > *max_dirs {
                            break;
                        }

                        let scale = dir.size as f32 / info.combined_size as f32;

                        paint_size_bar_before_next(ui, scale, accent_color);

                        // ui.label(format!("{:?} {}", dir.path, dir.size / 1024 / 1024));
                        ui.collapsing(
                            format!(
                                "{} | {} | {}%",
                                dir.path
                                    .file_name()
                                    .map(|d| d.to_string_lossy().to_string())
                                    .unwrap_or_default(),
                                ByteSize(dir.size),
                                (scale * 100.) as u8
                            ),
                            |ui| {
                                for (i, file) in dir.sorted_files().iter().enumerate() {
                                    if i as i32 > *max_dirs {
                                        break;
                                    }
                                    let s = del_sender.clone();

                                    draw_file(ui, file, *allow_delete, s);
                                }
                            },
                        );
                    }
                });

            Window::new("Directories")
                .scroll(true)
                .show(ui.ctx(), |ui| {
                    ui.label(format!("Directories"));

                    let root_dir = PathBuf::from(scan_path.clone());
                    if let Some(d) = info.tree.get(&root_dir) {
                        let sender = del_sender.clone();
                        draw_dir(ui, d, info, allow_delete.clone(), accent_color, sender)
                    }
                });

            Window::new("Filter builder")
                .scroll(true)
                .show(ui.ctx(), |ui| {
                    ui.label(format!("Filtered files"));

                    if ui.button("Add min size").clicked() {
                        filter_chain.push(Filter::MinSize(5));
                    }
                    if ui.button("Add min age").clicked() {
                        filter_chain.push(Filter::MinAge(1));
                    }
                    if ui.button("Add max age").clicked() {
                        filter_chain.push(Filter::MaxAge(30));
                    }
                    if ui.button("Add max results").clicked() {
                        filter_chain.push(Filter::MaxResults(50));
                    }

                    // Edit filters
                    for filter in filter_chain.iter_mut() {
                        match filter {
                            Filter::MinSize(size) => {
                                ui.add(
                                    Slider::i32(size, 1..=1000)
                                        .text("hide files smaller than this (MB)"),
                                );
                                if ui.button("X").clicked() {
                                    //filter_chain.retain(|x| *x != *filter);
                                    drop(filter);
                                }
                            }
                            Filter::MinAge(age) => {
                                ui.add(
                                    Slider::i32(age, 1..=500).text("hide files newer than (days)"),
                                );
                            }
                            Filter::MaxAge(age) => {
                                ui.add(
                                    Slider::i32(age, 1..=500).text("hide files older than (days)"),
                                );
                            }
                            Filter::MaxResults(max) => {
                                ui.add(Slider::i32(max, 1..=100).text("max results"));
                            } // _ => (),
                        }
                    }

                    if !filter_chain.is_empty() {
                        let mut i = 0;
                        'filter: for file in &info.files_by_size {
                            for filter in filter_chain.iter() {
                                match filter {
                                    Filter::MinSize(minsize) => {
                                        // println!("min{:?} / {:?}", minsize * 1024 * 1024, file.size);
                                        if *minsize * 1024 * 1024 > file.size as i32 {
                                            continue 'filter;
                                        }
                                    }
                                    Filter::MinAge(age) => {
                                        let discard_duration = std::time::Duration::from_secs(
                                            (*age as u64) * 24 * 3600,
                                        );
                                        if let Ok(elapsed) = file.modified.elapsed() {
                                            if elapsed < discard_duration {
                                                continue 'filter;
                                            }
                                        }
                                        // if file.modified.elapsed() {}
                                    }
                                    Filter::MaxAge(age) => {
                                        let discard_duration = std::time::Duration::from_secs(
                                            (*age as u64) * 24 * 3600,
                                        );
                                        if let Ok(elapsed) = file.modified.elapsed() {
                                            if elapsed > discard_duration {
                                                continue 'filter;
                                            }
                                        }
                                        // if file.modified.elapsed() {}
                                    }
                                    Filter::MaxResults(max) => {
                                        if i as i32 >= *max {
                                            break 'filter;
                                        }
                                    } // _ => (),
                                }
                            }
                            let s = del_sender.clone();

                            draw_file(ui, file, *allow_delete, s);

                            i += 1;
                        }
                    }
                });
            // Default::default()
        });
    }

    // fn on_exit(&mut self, storage: &mut dyn egui::app::Storage) {
    //     // egui::app::set_value(storage, egui::app::APP_KEY, self);
    // }
}

fn main() {
    // let i = diskspace_insight::scan("/home/woelper/Downloads");
    std::env::set_var("RUST_LOG", "info");
    let _ = env_logger::try_init();

    let title = "birdseye";
    // let storage = FileStorage::from_path(".birdseye.json".into());
    let mut app: MyApp = MyApp::default();
    app.scan_path = dirs::home_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let native_options = eframe::NativeOptions::default();

    eframe::run_native(Box::new(app), native_options);
}
