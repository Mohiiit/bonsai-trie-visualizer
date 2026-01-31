use eframe::egui;

use crate::Args;

pub struct BonsaiApp {
    args: Args,
    status: String,
    active_tab: Tab,
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
        let status = match &args.db_path {
            Some(path) => format!("DB path: {path}"),
            None => "DB path not set".to_string(),
        };
        Self {
            args,
            status,
            active_tab: Tab::Tree,
        }
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

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.active_tab {
                Tab::Tree => {
                    ui.heading("Tree View");
                    ui.label("Tree rendering will appear here.");
                }
                Tab::Path => {
                    ui.heading("Path Trace");
                    ui.label("Key path tracing UI will appear here.");
                }
                Tab::Diff => {
                    ui.heading("Diff View");
                    ui.label("Block diff view will appear here.");
                }
                Tab::Proof => {
                    ui.heading("Proof Viewer");
                    ui.label("Merkle proof visualizer will appear here.");
                }
                Tab::Stats => {
                    ui.heading("DB Stats");
                    ui.label("Database stats and diagnostics will appear here.");
                }
            }

            ui.separator();
            ui.label(format!("Args: db_path={:?} block={:?} diff={:?}", self.args.db_path, self.args.block, self.args.diff));
        });
    }
}
