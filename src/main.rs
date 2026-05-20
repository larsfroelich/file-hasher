use anyhow::{anyhow, Context, Result};
use clap::{Parser, ValueEnum};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use regex::Regex;
use std::sync::{Arc, OnceLock, atomic::{AtomicBool, Ordering}};
use eframe::egui;
use crossbeam_channel::{unbounded, Receiver, Sender};

static HASH_REGEX: OnceLock<Regex> = OnceLock::new();

#[derive(Parser, Debug)]
#[command(author, version, about = "File Hashing Tool", long_about = None)]
struct Args {
    /// The folder- or filepath to perform operations in
    path: Option<PathBuf>,

    /// Calculate hashes and rename files
    #[arg(long, group = "action")]
    hash: bool,

    /// Verify the hashes of previously processed files
    #[arg(long, group = "action")]
    verify: bool,

    /// The number of characters of the hash to include in the filename
    #[arg(long, default_value_t = 12)]
    hash_length: usize,

    /// The hashing algorithm to use
    #[arg(long, value_enum, default_value_t = HashAlgorithm::Sha256)]
    hash_algorithm: HashAlgorithm,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Launch the GUI
    #[arg(long)]
    gui: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug, Default)]
enum HashAlgorithm {
    #[default]
    Sha256,
}

trait Reporter: Send {
    fn log(&self, message: String);
    fn log_verbose(&self, message: String);
    fn progress(&self, current: usize, total: usize);
    fn should_abort(&self) -> bool;
}

struct CliReporter {
    verbose: bool,
}

impl Reporter for CliReporter {
    fn log(&self, message: String) {
        println!("{}", message);
    }
    fn log_verbose(&self, message: String) {
        if self.verbose {
            println!("{}", message);
        }
    }
    fn progress(&self, _current: usize, _total: usize) {}
    fn should_abort(&self) -> bool {
        false
    }
}

enum GuiMessage {
    Log(String),
    Progress(usize, usize),
    Finished(Result<()>),
}

struct GuiReporter {
    sender: Sender<GuiMessage>,
    abort_flag: Arc<AtomicBool>,
}

impl Reporter for GuiReporter {
    fn log(&self, message: String) {
        let _ = self.sender.send(GuiMessage::Log(message));
    }
    fn log_verbose(&self, message: String) {
        let _ = self.sender.send(GuiMessage::Log(message));
    }
    fn progress(&self, current: usize, total: usize) {
        let _ = self.sender.send(GuiMessage::Progress(current, total));
    }
    fn should_abort(&self) -> bool {
        self.abort_flag.load(Ordering::SeqCst)
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let no_args = std::env::args().len() == 1;

    if args.gui || no_args {
        return run_gui();
    }

    let path = args.path.as_ref().ok_or_else(|| anyhow!("Path is required in CLI mode"))?;
    let reporter = CliReporter { verbose: args.verbose };

    if args.hash {
        run_hash(path, args.hash_length, &reporter)?;
    } else if args.verify {
        run_verify(path, &reporter)?;
    } else {
        return Err(anyhow!("Either --hash or --verify must be specified in CLI mode"));
    }

    Ok(())
}

fn run_gui() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "File Hasher",
        options,
        Box::new(|_cc| Ok(Box::new(FileHasherApp::default()))),
    ).map_err(|e| anyhow!("Failed to run eframe: {}", e))
}

struct FileHasherApp {
    path: String,
    hash_length: usize,
    logs: Vec<String>,
    progress: (usize, usize),
    is_running: bool,
    abort_flag: Arc<AtomicBool>,
    receiver: Option<Receiver<GuiMessage>>,
}

impl Default for FileHasherApp {
    fn default() -> Self {
        Self {
            path: String::new(),
            hash_length: 12,
            logs: Vec::new(),
            progress: (0, 0),
            is_running: false,
            abort_flag: Arc::new(AtomicBool::new(false)),
            receiver: None,
        }
    }
}

impl FileHasherApp {
    fn start_task(&mut self, is_hash: bool) {
        self.logs.clear();
        self.progress = (0, 0);
        self.is_running = true;
        self.abort_flag.store(false, Ordering::SeqCst);

        let (sender, receiver) = unbounded();
        self.receiver = Some(receiver);

        let path = PathBuf::from(&self.path);
        let hash_length = self.hash_length;
        let abort_flag = Arc::clone(&self.abort_flag);

        std::thread::spawn(move || {
            let reporter = GuiReporter { sender: sender.clone(), abort_flag };
            let result = if is_hash {
                run_hash(&path, hash_length, &reporter)
            } else {
                run_verify(&path, &reporter)
            };
            let _ = sender.send(GuiMessage::Finished(result));
        });
    }
}

impl eframe::App for FileHasherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(ref rx) = self.receiver {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    GuiMessage::Log(line) => self.logs.push(line),
                    GuiMessage::Progress(current, total) => self.progress = (current, total),
                    GuiMessage::Finished(result) => {
                        self.is_running = false;
                        if let Err(e) = result {
                            self.logs.push(format!("ERROR: {:?}", e));
                        }
                        self.logs.push("Done.".to_string());
                    }
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("File Hasher");

            ui.horizontal(|ui| {
                ui.label("Path:");
                ui.text_edit_singleline(&mut self.path);
                if ui.button("Select...").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.path = path.display().to_string();
                    } else if let Some(path) = rfd::FileDialog::new().pick_file() {
                        self.path = path.display().to_string();
                    }
                }
            });

            ui.horizontal(|ui| {
                ui.label("Hash Length:");
                ui.add(egui::DragValue::new(&mut self.hash_length).range(6..=64));
                ui.label("Algorithm: SHA256");
            });

            ui.separator();

            ui.horizontal(|ui| {
                let can_start = !self.is_running && !self.path.is_empty();
                if ui.add_enabled(can_start, egui::Button::new("Hash")).clicked() {
                    self.start_task(true);
                }
                if ui.add_enabled(can_start, egui::Button::new("Verify")).clicked() {
                    self.start_task(false);
                }
                if ui.add_enabled(self.is_running, egui::Button::new("Abort")).clicked() {
                    self.abort_flag.store(true, Ordering::SeqCst);
                }
            });

            if self.progress.1 > 0 {
                let fraction = self.progress.0 as f32 / self.progress.1 as f32;
                ui.add(egui::ProgressBar::new(fraction).text(format!("{}/{}", self.progress.0, self.progress.1)));
            }

            ui.separator();
            ui.label("Logs:");
            egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                for log in &self.logs {
                    ui.label(log);
                }
            });
        });

        if self.is_running {
            ctx.request_repaint();
        }
    }
}

fn get_files(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }

    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect()
}

fn calc_sha256(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hex::encode(hasher.finalize()))
}

fn extract_hash(path: &Path) -> Option<String> {
    let filename = path.file_stem()?.to_str()?;
    let re = HASH_REGEX.get_or_init(|| Regex::new(r"_SHA256_([a-fA-F0-9]{6,})$").unwrap());
    if let Some(caps) = re.captures(filename) {
        return Some(caps.get(1)?.as_str().to_string());
    }
    None
}

fn run_hash(path: &Path, hash_length: usize, reporter: &dyn Reporter) -> Result<()> {
    reporter.log("** HASH **".to_string());
    let files = get_files(path);
    let total = files.len();

    for (i, file) in files.into_iter().enumerate() {
        if reporter.should_abort() {
            reporter.log("Operation aborted by user".to_string());
            break;
        }

        if let Some(_existing_hash) = extract_hash(&file) {
            reporter.log_verbose(format!("Skipping existing file: {}", file.display()));
            reporter.progress(i + 1, total);
            continue;
        }

        let file_hash = calc_sha256(&file)?.to_uppercase();
        let truncated_hash = &file_hash[..hash_length.min(file_hash.len())];

        let stem = file.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("Invalid filename: {}", file.display()))?;
        let extension = file.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e)).unwrap_or_default();

        let new_filename = format!("{}_SHA256_{}{}", stem, truncated_hash, extension);
        let mut new_path = file.clone();
        new_path.set_file_name(new_filename);

        reporter.log_verbose(format!("hash: {} - file: {}", file_hash, file.display()));

        fs::rename(&file, &new_path)
            .with_context(|| format!("Failed to rename {} to {}", file.display(), new_path.display()))?;

        reporter.progress(i + 1, total);
    }

    Ok(())
}

fn run_verify(path: &Path, reporter: &dyn Reporter) -> Result<()> {
    reporter.log("** VERIFY **".to_string());
    let files = get_files(path);
    let total = files.len();

    for (i, file) in files.into_iter().enumerate() {
        if reporter.should_abort() {
            reporter.log("Operation aborted by user".to_string());
            break;
        }

        if let Some(extracted_hash) = extract_hash(&file) {
            let actual_hash = calc_sha256(&file)?;
            let actual_truncated = &actual_hash[..extracted_hash.len().min(actual_hash.len())];

            if extracted_hash.to_lowercase() == actual_truncated.to_lowercase() {
                reporter.log(format!("all_good - file: {}", file.display()));
            } else {
                reporter.log(format!(
                    "HASH MISMATCH!!! from name: {} - actual: {} (file: \"{}\")",
                    extracted_hash, actual_truncated, file.display()
                ));
            }
        } else {
            reporter.log_verbose(format!("Skipping file without hash: {}", file.display()));
        }
        reporter.progress(i + 1, total);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_extract_hash() {
        assert_eq!(extract_hash(Path::new("file_SHA256_A1B2C3D4E5F6.txt")), Some("A1B2C3D4E5F6".to_string()));
        assert_eq!(extract_hash(Path::new("file.txt")), None);
        assert_eq!(extract_hash(Path::new("file_SHA256_123.txt")), None); // too short
        assert_eq!(extract_hash(Path::new("file_SHA256_A1B2C3D4E5F6G7.txt")), None); // invalid hex
    }
}
