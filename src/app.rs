use eframe::egui;
use egui::CollapsingHeader;
use serde::Serialize;
use std::path::PathBuf;
use crate::bonsai::diff_reader::read_block_log;
use crate::bonsai::node::Node;
use crate::bonsai::path::{felt_to_path, PathBits};
use crate::bonsai::proof::{build_proof, verify_proof, ProofNode};
use crate::bonsai::trie_reader::{TrieReader, TrieSpec};
use crate::db::cf_map;
use crate::db::RocksDb;
use crate::model::TrieKind;
use crate::util::hex::{bytes_to_hex, decode_felt_scale, format_felt_short, parse_felt_hex};
use crate::Args;

pub struct BonsaiApp {
    args: Args,
    status: String,
    active_tab: Tab,

    db_path_input: String,
    db: Option<RocksDb>,
    db_error: Option<String>,

    tree_trie: TrieKind,
    storage_identifier_input: String,

    key_input: String,
    path_trace_status: String,
    proof_status: String,
    proof_output_path: String,
    last_proof: Vec<ProofNode>,

    diff_block_input: String,
    diff_trie: TrieKind,

    screenshot_dir: String,
    pending_screenshot: bool,
    last_screenshot: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Tab {
    Tree,
    Path,
    Diff,
    Proof,
    Stats,
}

impl BonsaiApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, args: Args) -> Self {
        let db_path_input = args.db_path.clone().unwrap_or_default();
        let status = if db_path_input.is_empty() {
            "DB path not set".to_string()
        } else {
            format!("DB path: {db_path_input}")
        };
        Self {
            args,
            status,
            active_tab: Tab::Tree,
            db_path_input,
            db: None,
            db_error: None,
            tree_trie: TrieKind::Contract,
            storage_identifier_input: String::new(),
            key_input: String::new(),
            path_trace_status: String::new(),
            proof_status: String::new(),
            proof_output_path: "assets/screenshots/proof.json".to_string(),
            last_proof: Vec::new(),
            diff_block_input: String::new(),
            diff_trie: TrieKind::Contract,
            screenshot_dir: "assets/screenshots".to_string(),
            pending_screenshot: false,
            last_screenshot: None,
        }
    }

    fn ensure_db(&mut self) {
        if self.db.is_some() || self.db_path_input.trim().is_empty() {
            return;
        }
        match RocksDb::open_read_only(self.db_path_input.trim()) {
            Ok(db) => {
                self.status = format!("DB open: {}", self.db_path_input.trim());
                self.db = Some(db);
                self.db_error = None;
            }
            Err(err) => {
                self.db = None;
                self.db_error = Some(err.to_string());
                self.status = "DB open failed".to_string();
            }
        }
    }

    fn build_spec(&self, trie: TrieKind) -> Option<TrieSpec> {
        let identifier = match trie {
            TrieKind::Contract => trie.identifier().to_vec(),
            TrieKind::Class => trie.identifier().to_vec(),
            TrieKind::Storage => {
                let felt = parse_felt_hex(&self.storage_identifier_input).ok()?;
                felt.to_bytes_be().to_vec()
            }
        };

        let (trie_cf, flat_cf, log_cf) = match trie {
            TrieKind::Contract => (
                cf_map::BONSAI_CONTRACT_TRIE,
                cf_map::BONSAI_CONTRACT_FLAT,
                cf_map::BONSAI_CONTRACT_LOG,
            ),
            TrieKind::Storage => (
                cf_map::BONSAI_CONTRACT_STORAGE_TRIE,
                cf_map::BONSAI_CONTRACT_STORAGE_FLAT,
                cf_map::BONSAI_CONTRACT_STORAGE_LOG,
            ),
            TrieKind::Class => (
                cf_map::BONSAI_CLASS_TRIE,
                cf_map::BONSAI_CLASS_FLAT,
                cf_map::BONSAI_CLASS_LOG,
            ),
        };

        Some(TrieSpec {
            identifier,
            trie_cf: trie_cf.to_string(),
            flat_cf: flat_cf.to_string(),
            log_cf: log_cf.to_string(),
        })
    }

    fn render_status(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("DB Path:");
            ui.text_edit_singleline(&mut self.db_path_input);
            if ui.button("Open").clicked() {
                self.db = None;
                self.ensure_db();
            }
        });

        if let Some(err) = &self.db_error {
            ui.colored_label(egui::Color32::RED, err);
        }
    }

    fn render_stats(&mut self, ui: &mut egui::Ui) {
        self.ensure_db();
        self.render_status(ui);
        ui.separator();

        let Some(db) = &self.db else {
            ui.label("DB not opened.");
            return;
        };

        ui.heading("Column Families");
        ui.label(format!("Total: {}", db.cf_names().len()));
        egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
            for name in db.cf_names() {
                let required = cf_map::BONSAI_COLUMNS.contains(&name.as_str());
                if required {
                    ui.label(format!("{name} (bonsai)"));
                } else {
                    ui.label(name);
                }
            }
        });
    }

    fn render_screenshot_controls(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.separator();
        ui.heading("Screenshots");
        ui.horizontal(|ui| {
            ui.label("Dir:");
            ui.text_edit_singleline(&mut self.screenshot_dir);
            if ui.button("Capture").clicked() {
                self.pending_screenshot = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
        });
        if let Some(path) = &self.last_screenshot {
            ui.label(format!("Last screenshot: {path}"));
        }
    }

    fn render_tree(&mut self, ui: &mut egui::Ui) {
        self.ensure_db();
        self.render_status(ui);
        ui.separator();

        let Some(db) = &self.db else {
            ui.label("DB not opened.");
            return;
        };

        ui.horizontal(|ui| {
            ui.label("Trie:");
            ui.selectable_value(&mut self.tree_trie, TrieKind::Contract, "Contract");
            ui.selectable_value(&mut self.tree_trie, TrieKind::Storage, "Storage");
            ui.selectable_value(&mut self.tree_trie, TrieKind::Class, "Class");
        });

        if self.tree_trie == TrieKind::Storage {
            ui.horizontal(|ui| {
                ui.label("Contract address:");
                ui.text_edit_singleline(&mut self.storage_identifier_input);
            });
        }

        let Some(spec) = self.build_spec(self.tree_trie) else {
            ui.label("Provide a valid contract address for storage trie.");
            return;
        };

        let mut reader = TrieReader::new(db.clone(), spec);
        let Some(root) = reader.load_root_node() else {
            ui.label("Empty trie or root missing.");
            return;
        };

        ui.separator();
        ui.heading("Tree");
        let root_path = PathBits::default();
        render_node(ui, &mut reader, &root_path, &root, 0);
        let ctx = ui.ctx().clone();
        self.render_screenshot_controls(ui, &ctx);
    }

    fn render_path_trace(&mut self, ui: &mut egui::Ui) {
        self.ensure_db();
        self.render_status(ui);
        ui.separator();

        let Some(db) = &self.db else {
            ui.label("DB not opened.");
            return;
        };

        ui.horizontal(|ui| {
            ui.label("Trie:");
            ui.selectable_value(&mut self.tree_trie, TrieKind::Contract, "Contract");
            ui.selectable_value(&mut self.tree_trie, TrieKind::Storage, "Storage");
            ui.selectable_value(&mut self.tree_trie, TrieKind::Class, "Class");
        });
        if self.tree_trie == TrieKind::Storage {
            ui.horizontal(|ui| {
                ui.label("Contract address:");
                ui.text_edit_singleline(&mut self.storage_identifier_input);
            });
        }

        ui.horizontal(|ui| {
            ui.label("Key (hex felt):");
            ui.text_edit_singleline(&mut self.key_input);
            if ui.button("Trace").clicked() {
                self.path_trace_status.clear();
            }
        });

        let Some(spec) = self.build_spec(self.tree_trie) else {
            ui.label("Provide a valid contract address for storage trie.");
            return;
        };
        let mut reader = TrieReader::new(db.clone(), spec);

        let felt = match parse_felt_hex(&self.key_input) {
            Ok(felt) => felt,
            Err(err) => {
                ui.colored_label(egui::Color32::RED, err);
                return;
            }
        };

        let key_path = felt_to_path(&felt);
        let mut path = PathBits::default();
        let mut steps: Vec<String> = Vec::new();

        let Some(mut node) = reader.load_root_node() else {
            ui.label("Empty trie.");
            return;
        };

        loop {
            match &node {
                Node::Binary(bin) => {
                    let bit_index = path.len();
                    if bit_index >= key_path.len() {
                        self.path_trace_status = "Reached end of key.".to_string();
                        break;
                    }
                    let direction = key_path.0[bit_index];
                    let dir_str = if direction { "R" } else { "L" };
                    steps.push(format!(
                        "Binary h={} hash={} -> {dir_str}",
                        bin.height,
                        bin.hash.map(|h| format_felt_short(&h)).unwrap_or("none".to_string())
                    ));
                    path.push(direction);
                }
                Node::Edge(edge) => {
                    let edge_bits = PathBits(edge.path.0.clone());
                    let matches = key_path.0.get(path.len()..(path.len() + edge_bits.len()))
                        == Some(&edge_bits.0);
                    steps.push(format!(
                        "Edge h={} path_len={} match={}",
                        edge.height,
                        edge_bits.len(),
                        matches
                    ));
                    path.extend_from_bitslice(&edge_bits.0);
                    if !matches {
                        self.path_trace_status = "Path mismatch (non-member).".to_string();
                        break;
                    }
                }
            }

            if path.len() >= key_path.len() {
                let value = reader.load_flat_value(&key_path);
                self.path_trace_status = match value {
                    Some(value) => format!("Leaf value: {}", value.to_string()),
                    None => "Leaf not found".to_string(),
                };
                break;
            }

            match reader.load_node_by_path(&path) {
                Some(next) => node = next,
                None => {
                    self.path_trace_status = "Missing node in trie".to_string();
                    break;
                }
            }
        }

        ui.separator();
        ui.heading("Steps");
        egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
            for (idx, step) in steps.iter().enumerate() {
                ui.label(format!("{:02}: {step}", idx + 1));
            }
        });

        if !self.path_trace_status.is_empty() {
            ui.separator();
            ui.label(&self.path_trace_status);
        }
        let ctx = ui.ctx().clone();
        self.render_screenshot_controls(ui, &ctx);
    }

    fn render_diff(&mut self, ui: &mut egui::Ui) {
        self.ensure_db();
        self.render_status(ui);
        ui.separator();

        let Some(db) = &self.db else {
            ui.label("DB not opened.");
            return;
        };

        ui.horizontal(|ui| {
            ui.label("Block:");
            ui.text_edit_singleline(&mut self.diff_block_input);
            ui.label("Trie:");
            ui.selectable_value(&mut self.diff_trie, TrieKind::Contract, "Contract");
            ui.selectable_value(&mut self.diff_trie, TrieKind::Storage, "Storage");
            ui.selectable_value(&mut self.diff_trie, TrieKind::Class, "Class");
        });

        let block: u64 = match self.diff_block_input.trim().parse() {
            Ok(val) => val,
            Err(_) => {
                ui.label("Enter a block number.");
                return;
            }
        };

        let log_cf = match self.diff_trie {
            TrieKind::Contract => cf_map::BONSAI_CONTRACT_LOG,
            TrieKind::Storage => cf_map::BONSAI_CONTRACT_STORAGE_LOG,
            TrieKind::Class => cf_map::BONSAI_CLASS_LOG,
        };

        let entries = read_block_log(db, log_cf, block);

        ui.separator();
        ui.label(format!("Entries: {}", entries.len()));
        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            for entry in entries {
                let key_bits = entry
                    .key_bits
                    .as_ref()
                    .map(|k| format!("len={}", k.len()))
                    .unwrap_or_else(|| "unknown".to_string());
                let change = match entry.change_type {
                    0 => "new",
                    1 => "old",
                    _ => "unknown",
                };
                let key_type = match entry.key_type {
                    0 => "trie",
                    1 => "flat",
                    _ => "unknown",
                };
                let value_display = decode_felt_scale(&entry.value)
                    .map(|felt| format_felt_short(&felt))
                    .unwrap_or_else(|| bytes_to_hex(&entry.value));
                ui.label(format!(
                    "{} {:?} key_type={} change={} key={} value={}",
                    entry.block, entry.trie_kind, key_type, change, key_bits, value_display
                ));
            }
        });
        let ctx = ui.ctx().clone();
        self.render_screenshot_controls(ui, &ctx);
    }

    fn render_proof(&mut self, ui: &mut egui::Ui) {
        self.ensure_db();
        self.render_status(ui);
        ui.separator();

        let Some(db) = &self.db else {
            ui.label("DB not opened.");
            return;
        };

        ui.horizontal(|ui| {
            ui.label("Trie:");
            ui.selectable_value(&mut self.tree_trie, TrieKind::Contract, "Contract");
            ui.selectable_value(&mut self.tree_trie, TrieKind::Storage, "Storage");
            ui.selectable_value(&mut self.tree_trie, TrieKind::Class, "Class");
        });
        if self.tree_trie == TrieKind::Storage {
            ui.horizontal(|ui| {
                ui.label("Contract address:");
                ui.text_edit_singleline(&mut self.storage_identifier_input);
            });
        }

        ui.horizontal(|ui| {
            ui.label("Key (hex felt):");
            ui.text_edit_singleline(&mut self.key_input);
            if ui.button("Build Proof").clicked() {
                self.proof_status.clear();
            }
        });

        let Some(spec) = self.build_spec(self.tree_trie) else {
            ui.label("Provide a valid contract address for storage trie.");
            return;
        };
        let mut reader = TrieReader::new(db.clone(), spec);

        let felt = match parse_felt_hex(&self.key_input) {
            Ok(felt) => felt,
            Err(err) => {
                ui.colored_label(egui::Color32::RED, err);
                return;
            }
        };
        let key_path = felt_to_path(&felt);

        let Some(root_node) = reader.load_root_node() else {
            ui.label("Empty trie.");
            return;
        };
        let root_hash = match root_node {
            Node::Binary(bin) => bin.hash,
            Node::Edge(edge) => edge.hash,
        };

        let Some(root_hash) = root_hash else {
            ui.label("Root hash not found.");
            return;
        };

        let proof = match build_proof(&mut reader, &key_path) {
            Some(proof) => proof,
            None => {
                ui.label("Failed to build proof.");
                return;
            }
        };

        let verified = verify_proof(root_hash, &key_path, &proof, self.tree_trie);
        self.proof_status = if verified {
            "Proof verified".to_string()
        } else {
            "Proof failed".to_string()
        };
        self.last_proof = proof.clone();

        ui.separator();
        ui.label(&self.proof_status);
        ui.label(format!("Proof nodes: {}", proof.len()));
        ui.horizontal(|ui| {
            ui.label("Export JSON:");
            ui.text_edit_singleline(&mut self.proof_output_path);
            if ui.button("Save").clicked() {
                if let Err(err) = save_proof_json(&self.proof_output_path, &proof, self.tree_trie) {
                    self.proof_status = format!("Save failed: {err}");
                } else {
                    self.proof_status = "Saved proof JSON".to_string();
                }
            }
        });
        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            for (idx, node) in proof.iter().enumerate() {
                let _ = match node {
                    ProofNode::Binary { left, right } => ui.label(format!(
                        "{:02} Binary L={} R={}",
                        idx + 1,
                        format_felt_short(left),
                        format_felt_short(right)
                    )),
                    ProofNode::Edge { child, path } => ui.label(format!(
                        "{:02} Edge path_len={} child={}",
                        idx + 1,
                        path.len(),
                        format_felt_short(child)
                    )),
                };
            }
        });
        let ctx = ui.ctx().clone();
        self.render_screenshot_controls(ui, &ctx);
    }
}

impl eframe::App for BonsaiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Bonsai Trie Visualizer");
                ui.separator();
                ui.label(&self.status);
            });
        });

        egui::SidePanel::left("nav").min_width(160.0).show(ctx, |ui| {
            ui.heading("Views");
            ui.separator();
            ui.selectable_value(&mut self.active_tab, Tab::Tree, "Tree");
            ui.selectable_value(&mut self.active_tab, Tab::Path, "Path Trace");
            ui.selectable_value(&mut self.active_tab, Tab::Diff, "Diff");
            ui.selectable_value(&mut self.active_tab, Tab::Proof, "Proof");
            ui.selectable_value(&mut self.active_tab, Tab::Stats, "Stats");
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.active_tab {
            Tab::Tree => self.render_tree(ui),
            Tab::Path => self.render_path_trace(ui),
            Tab::Diff => self.render_diff(ui),
            Tab::Proof => self.render_proof(ui),
            Tab::Stats => self.render_stats(ui),
        });

        if self.pending_screenshot {
            handle_screenshot_events(ctx, &mut self.last_screenshot, &self.screenshot_dir);
            self.pending_screenshot = false;
        } else {
            handle_screenshot_events(ctx, &mut self.last_screenshot, &self.screenshot_dir);
        }
    }
}

fn render_node(ui: &mut egui::Ui, reader: &mut TrieReader, path: &PathBits, node: &Node, depth: usize) {
    let title = match node {
        Node::Binary(binary) => format!(
            "Binary h={} path_len={} hash={}",
            binary.height,
            path.len(),
            binary
                .hash
                .map(|h| format_felt_short(&h))
                .unwrap_or_else(|| "none".to_string())
        ),
        Node::Edge(edge) => format!(
            "Edge h={} path_len={} edge_len={} hash={}",
            edge.height,
            path.len(),
            edge.path.len(),
            edge.hash
                .map(|h| format_felt_short(&h))
                .unwrap_or_else(|| "none".to_string())
        ),
    };

    if depth > 6 {
        ui.label("Depth limit reached.");
        return;
    }

    CollapsingHeader::new(title)
        .default_open(depth < 2)
        .show(ui, |ui| match node {
            Node::Binary(binary) => {
                ui.label(format!("Left hash: {}", format_node_hash(binary.left)));
                ui.label(format!("Right hash: {}", format_node_hash(binary.right)));

                let left_path = path.with_bit(false);
                let right_path = path.with_bit(true);

                if let Some(left_node) = reader.load_node_by_path(&left_path) {
                    render_node(ui, reader, &left_path, &left_node, depth + 1);
                } else {
                    ui.label("Left child missing");
                }

                if let Some(right_node) = reader.load_node_by_path(&right_path) {
                    render_node(ui, reader, &right_path, &right_node, depth + 1);
                } else {
                    ui.label("Right child missing");
                }
            }
            Node::Edge(edge) => {
                let edge_bits = PathBits(edge.path.0.clone());
                let mut child_path = path.clone();
                child_path.extend_from_bitslice(edge_bits.0.as_bitslice());
                ui.label(format!("Child hash: {}", format_node_hash(edge.child)));
                if let Some(child_node) = reader.load_node_by_path(&child_path) {
                    render_node(ui, reader, &child_path, &child_node, depth + 1);
                } else {
                    ui.label("Child missing");
                }
            }
        });
}

fn format_node_hash(handle: crate::bonsai::node::NodeHandle) -> String {
    handle
        .as_hash()
        .map(|h| format_felt_short(&h))
        .unwrap_or_else(|| "in-memory".to_string())
}

fn handle_screenshot_events(ctx: &egui::Context, last_path: &mut Option<String>, dir: &str) {
    let events = ctx.input(|i| i.events.clone());
    for event in events {
        if let egui::Event::Screenshot { image, .. } = event {
            if let Some(path) = save_color_image(&image, dir) {
                *last_path = Some(path);
            }
        }
    }
}

fn save_color_image(image: &egui::ColorImage, dir: &str) -> Option<String> {
    let mut path = PathBuf::from(dir);
    if std::fs::create_dir_all(&path).is_err() {
        return None;
    }
    let filename = format!("screenshot-{}.png", chrono_stamp());
    path.push(filename);

    let size = [image.size[0] as u32, image.size[1] as u32];
    let mut rgba = Vec::with_capacity(image.pixels.len() * 4);
    for pixel in &image.pixels {
        rgba.push(pixel.r());
        rgba.push(pixel.g());
        rgba.push(pixel.b());
        rgba.push(pixel.a());
    }

    let buffer = image::RgbaImage::from_raw(size[0], size[1], rgba)?;
    if buffer.save(&path).is_ok() {
        return path.to_str().map(|s| s.to_string());
    }
    None
}

fn chrono_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    nanos.to_string()
}

#[derive(Serialize)]
struct ProofNodeJson {
    kind: &'static str,
    left: Option<String>,
    right: Option<String>,
    child: Option<String>,
    path_len: Option<usize>,
}

fn save_proof_json(path: &str, proof: &[ProofNode], kind: TrieKind) -> Result<(), String> {
    let nodes: Vec<ProofNodeJson> = proof
        .iter()
        .map(|node| match node {
            ProofNode::Binary { left, right } => ProofNodeJson {
                kind: "binary",
                left: Some(format!("{left:#x}")),
                right: Some(format!("{right:#x}")),
                child: None,
                path_len: None,
            },
            ProofNode::Edge { child, path } => ProofNodeJson {
                kind: "edge",
                left: None,
                right: None,
                child: Some(format!("{child:#x}")),
                path_len: Some(path.len()),
            },
        })
        .collect();

    let payload = serde_json::json!({
        "trie": format!("{:?}", kind),
        "nodes": nodes
    });
    std::fs::write(path, serde_json::to_vec_pretty(&payload).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
