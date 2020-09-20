#![warn(clippy::all)]

use dirs;
use diskspace_insight;
use diskspace_insight::{DirInfo, File};
use egui::math::Vec2;
use egui::paint::color::Srgba;
use egui::{paint::PaintCmd, Label, Rect, Slider, Style, TextStyle, Ui, Window};
use egui_glium::storage::FileStorage;
use std::sync::mpsc::channel;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
// #[derive(Default, serde::Deserialize, serde::Serialize)]
#[derive(Default)]
struct MyApp {
    my_string: String,
    max_types: i32,
    max_files: i32,
    max_dirs: i32,
    info: Option<DirInfo>,
    allow_delete: bool,
    filter_chain: Vec<Filter>,
}

#[derive(Debug, PartialEq)]
enum Filter {
    MinAge(i32),
    MaxAge(i32),
    MinSize(i32),
    MaxResults(i32),
}

fn draw_file(ui: &mut Ui, file: &File, allow_delete: bool) {
    ui.horizontal(|ui| {
        // ui.label(format!("{:<10}MB", file.size / 1024 / 1024));
        ui.add(
            Label::new(format!("{:8} MB", file.size / 1024 / 1024))
                .text_style(TextStyle::Monospace),
        );
        // ui.expand_to_size(egui::math::Vec2::new(100.,10.));
        if allow_delete {
            if ui.button("Del").clicked {
                let _ = std::fs::remove_file(&file.path);
            }
        }
        ui.label(format!("{}", file.path.display()));
    });

}

fn gen_light_style() -> Style {
    let mut style = Style::default();
    style.visuals.window_corner_radius = 1.;
    style
}

fn paint_size_bar_before_next(ui: &mut Ui, scale: f32, color: Srgba) {
    // ask for available space - this is just to get the cursor
    let mut paint_rect = ui.available();
    paint_rect.max.y = paint_rect.min.y + ui.style().spacing.interact_size.y + 2.;
    paint_rect.max.x = paint_rect.min.x + ui.available().size().x * scale;

    ui.painter().add(PaintCmd::Rect {
        rect: paint_rect,
        corner_radius: 2.,
        fill: color,
        stroke: egui::paint::command::Stroke::default(),
    });
}

impl egui::app::App for MyApp {
    /// This function will be called whenever the Ui needs to be shown,
    /// which may be many times per second.
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut dyn egui::app::Backend) {
        let accent_color = Srgba::new(120, 50, 200, 255);

        ui.set_style(gen_light_style());
        ui.style_mut().visuals.window_corner_radius = 1.;
        ui.style_mut().visuals.dark_bg_color = Srgba::new(100, 0, 100, 255);

        // ui.style_mut().visuals.dark_bg_color

        let MyApp {
            my_string,
            max_types,
            max_files,
            max_dirs,
            info,
            allow_delete,
            filter_chain,
        } = self;

        Window::new("Setup").show(ui.ctx(), |ui| {
            ui.horizontal(|ui| {
                if ui.button("Home").clicked {
                    if let Some(dir) = dirs::home_dir() {
                        *my_string = dir.to_string_lossy().to_string();
                    }
                }
                if ui.button("Downloads").clicked {
                    if let Some(dir) = dirs::download_dir() {
                        *my_string = dir.to_string_lossy().to_string();
                    }
                }
                if ui.button("Videos").clicked {
                    if let Some(dir) = dirs::video_dir() {
                        *my_string = dir.to_string_lossy().to_string();
                    }
                }
                if ui.button("Cache").clicked {
                    if let Some(dir) = dirs::cache_dir() {
                        *my_string = dir.to_string_lossy().to_string();
                    }
                }
                if ui.button("Temp").clicked {
                    *my_string = std::env::temp_dir().to_string_lossy().to_string();
                }
            });

            ui.text_edit(my_string);

            ui.checkbox("Allow deletion", allow_delete);
            if ui.button("Scan!").clicked {
                *info = Some(diskspace_insight::scan(&my_string));
            }
        });

        Window::new("Filetypes").scroll(true).show(ui.ctx(), |ui| {
            ui.label(format!("Files by type, largest first"));
            ui.add(Slider::i32(max_types, 1..=100).text("max results"));
            //ui.painter().rect_filled(Rect::from_min_max(pos2(0., 0.), pos2(100., 100.)), 2., Srgba::new(255,0,255, 255));
            // let visuals = ui.style().interact(&response);

            if let Some(info) = info {
                for (i, filetype) in info.types_by_size.iter().enumerate() {
                    if i as i32 >= *max_types {
                        break;
                    }

                    let scale = filetype.size as f32 / info.combined_size as f32;
                    paint_size_bar_before_next(ui, scale, accent_color);

                    ui.collapsing(
                        format!(
                            "{} | {}MB | {}% | {} files",
                            filetype.ext,
                            filetype.size / 1024 / 1024,
                            (scale * 100.) as u8,
                            filetype.files.len()
                        ),
                        |ui| {
                            for file in &filetype.files {
                                draw_file(ui, file, *allow_delete);
                            }
                        },
                    );
                }
            }
        });

        Window::new("Files").scroll(true).show(ui.ctx(), |ui| {
            ui.label(format!("Files by size, largest first"));
            ui.add(Slider::i32(max_files, 1..=100).text("max results"));

            if let Some(info) = info {
                for (i, file) in info.files_by_size.iter().enumerate() {
                    if i as i32 >= *max_files {
                        break;
                    }
                    draw_file(ui, file, *allow_delete);
                }
            }
        });

        Window::new("Directories")
            .scroll(true)
            .show(ui.ctx(), |ui| {
                ui.label(format!("Directories"));
                ui.add(Slider::i32(max_dirs, 1..=100).text("max results"));

                if let Some(info) = info {
                    for (i, dir) in info.dirs_by_size.iter().enumerate() {
                        if i as i32 > *max_dirs {
                            break;
                        }

                        let scale = dir.size as f32 / info.combined_size as f32;

                        paint_size_bar_before_next(ui, scale, accent_color);
                        
                        // ui.label(format!("{:?} {}", dir.path, dir.size / 1024 / 1024));
                        ui.collapsing(
                            format!(
                                "{} | {}MB | {}%",
                                dir.path.file_name().map(|d| d.to_string_lossy().to_string()).unwrap_or_default(),
                                dir.size / 1024 / 1024,
                                (scale * 100.) as u8
                            ),
                            |ui| {
                                // for file in &filetype.files {
                                //     draw_file(ui, file, *allow_delete);
                                // }
                            },
                        );
                    }
                }
            });

        Window::new("Filter builder")
            .scroll(true)
            .show(ui.ctx(), |ui| {
                ui.label(format!("Filtered files"));

                if ui.button("Add min size").clicked {
                    filter_chain.push(Filter::MinSize(5));
                }
                if ui.button("Add min age").clicked {
                    filter_chain.push(Filter::MinAge(1));
                }
                if ui.button("Add max age").clicked {
                    filter_chain.push(Filter::MaxAge(30));
                }
                if ui.button("Add max results").clicked {
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
                            if ui.button("X").clicked {
                                //filter_chain.retain(|x| *x != *filter);
                                drop(filter);
                            }
                        }
                        Filter::MinAge(age) => {
                            ui.add(Slider::i32(age, 1..=500).text("hide files newer than (days)"));
                        }
                        Filter::MaxAge(age) => {
                            ui.add(Slider::i32(age, 1..=500).text("hide files older than (days)"));
                        }
                        Filter::MaxResults(max) => {
                            ui.add(Slider::i32(max, 1..=100).text("max results"));
                        }
                        _ => (),
                    }
                }

                if let Some(info) = info {
                    if !filter_chain.is_empty() {
                        let mut i = 0;
                        'filter: for file in &info.files_by_size {
                            for filter in filter_chain.iter() {
                                match filter {
                                    Filter::MinSize(minsize) => {
                                        println!(
                                            "min{:?} / {:?}",
                                            minsize * 1024 * 1024,
                                            file.size
                                        );
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
                                    }
                                    _ => (),
                                }
                            }

                            draw_file(ui, file, *allow_delete);

                            i += 1;
                        }
                    }
                }
            });
    }

    // fn on_exit(&mut self, storage: &mut dyn egui::app::Storage) {
    //     // egui::app::set_value(storage, egui::app::APP_KEY, self);
    // }
}




fn main() {
    // let i = diskspace_insight::scan("/home/woelper/Downloads");
    let title = "birdseye";
    let storage = FileStorage::from_path(".birdseye.json".into());
    let mut app: MyApp = MyApp::default();
    app.my_string = dirs::home_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    app.max_types = 10;
    app.max_files = 40;
    app.max_dirs = 20;
    egui_glium::run(title, storage, app);
}
