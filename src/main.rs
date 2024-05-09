use eframe::egui::{Align, Button, CentralPanel, Color32, Label, Layout, ScrollArea, TextEdit, TopBottomPanel};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Text File Splitter",
        options,
        Box::new(|_cc| Box::new(TextFileSplitter::default())),
    )
}

#[derive(Default)]
struct TextFileSplitter {
    input_path: String,
    output_dir: String,
    logs: Arc<Mutex<String>>,
    splitting: bool,
    lines_per_file: String,
}

impl eframe::App for TextFileSplitter {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.heading("Text File Splitter");
        });

        CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Input File:");
                    ui.text_edit_singleline(&mut self.input_path);
                    if ui.button("Browse").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.input_path = path.to_string_lossy().to_string();
                        }
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Output Directory:");
                    ui.text_edit_singleline(&mut self.output_dir);
                    if ui.button("Browse").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.output_dir = path.to_string_lossy().to_string();
                        }
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Lines per File:");
                    ui.text_edit_singleline(&mut self.lines_per_file);
                });

                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if !self.splitting {
                            if ui.add(Button::new("Start Splitting")).clicked() {
                                self.start_splitting();
                            }
                        } else {
                            ui.label("Splitting in progress...");
                        }
                    });
                });

                ui.separator();
                ui.label("Logs:");
                ScrollArea::vertical().show(ui, |ui| {
                    ui.style_mut().visuals.override_text_color = Some(Color32::WHITE);
                    ui.visuals_mut().override_text_color = Some(Color32::WHITE);
                    ui.style_mut().visuals.widgets.noninteractive.bg_fill = Color32::BLACK;

                    let logs = self.logs.lock().unwrap();
                    ui.add(Label::new(&*logs).wrap(false));
                });
            });
        });

        ctx.request_repaint();
    }
}

impl TextFileSplitter {
    fn start_splitting(&mut self) {
        let input_path = self.input_path.clone();
        let output_dir = self.output_dir.clone();
        let logs = Arc::clone(&self.logs);
        let lines_per_file: usize = self.lines_per_file.parse().unwrap_or(100_000);

        self.splitting = true;
        thread::spawn(move || {
            let result = split_text_file(&input_path, &output_dir, &logs, lines_per_file);
            if let Err(e) = result {
                let mut logs = logs.lock().unwrap();
                logs.push_str(&format!("Error: {}\n", e));
            }

            let mut logs = logs.lock().unwrap();
            logs.push_str("Splitting completed successfully!\n");
        });
    }
}

fn split_text_file(
    input_file_path: &str,
    output_folder_path: &str,
    logs: &Arc<Mutex<String>>,
    lines_per_file: usize,
) -> io::Result<()> {
    let input_file_path = Path::new(input_file_path);
    let output_folder_path = Path::new(output_folder_path);

    let mut logs_lock = logs.lock().unwrap();
    logs_lock.push_str(&format!("Splitting file: {}\n", input_file_path.display()));
    fs::create_dir_all(&output_folder_path)?;
    logs_lock.push_str(&format!("Output directory: {}\n", output_folder_path.display()));
    drop(logs_lock);

    let input_file = File::open(&input_file_path)?;
    let buffered_reader = BufReader::new(input_file);

    let mut file_count = 0;
    let mut line_count = 0;
    let mut output_file_name = format!("split_{:03}.txt", file_count);
    let mut output_file_path = output_folder_path.join(&output_file_name);
    let mut output_file = BufWriter::new(File::create(&output_file_path)?);

    for (index, byte_result) in buffered_reader.split(b'\n').enumerate() {
        let bytes = byte_result?;
        let line = String::from_utf8_lossy(&bytes);
        writeln!(output_file, "{}", line.trim_end())?;

        line_count += 1;
        if line_count >= lines_per_file {
            file_count += 1;
            line_count = 0;

            output_file_name = format!("split_{:03}.txt", file_count);
            output_file_path = output_folder_path.join(&output_file_name);
            output_file = BufWriter::new(File::create(&output_file_path)?);
        }

        if index % 10_000 == 0 {
            let mut logs_lock = logs.lock().unwrap();
            logs_lock.push_str(&format!("Processed {} lines\n", index));
        }
    }

    let mut logs_lock = logs.lock().unwrap();
    logs_lock.push_str(&format!(
        "Splitting completed. {} files created in {}\n",
        file_count + 1,
        output_folder_path.display()
    ));

    Ok(())
}
