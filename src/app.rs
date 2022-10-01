use eframe::{
    egui::{self, Label, RichText, Slider, TextEdit},
    emath::vec2,
};
use simplez_assembler::nom;
use simplez_common::Instruction;

use crate::highlighter;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    program: String,
    assembler_output: String,
    error_loc: Option<usize>,

    #[serde(with = "crate::util::arrays")]
    memory: [u16; 512],
}

impl Default for App {
    fn default() -> Self {
        Self {
            program: String::new(),
            error_loc: Some(5),
            assembler_output: String::new(),
            memory: [0; 512],
        }
    }
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for App {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::warn_if_debug_build(ui);
            ui.horizontal_top(|ui| {
                let theme = highlighter::CodeTheme::from_memory(ui.ctx());

                let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                    let mut layout_job = highlighter::highlight(ui.ctx(), &theme, string);
                    layout_job.wrap.max_width = wrap_width;
                    ui.fonts().layout_job(layout_job)
                };

                let textedit_response = ui.add(
                    TextEdit::multiline(&mut self.program)
                        .code_editor()
                        .desired_width(ui.available_width() / 2.)
                        .desired_rows(40)
                        .layouter(&mut layouter),
                );

                if let Some(error_loc) = &mut self.error_loc {
                    let mut error_rect = textedit_response.rect;
                    error_rect.min.y = error_rect.min.y + 12. * *error_loc as f32;
                    error_rect.set_height(12.);

                    ui.painter().rect_filled(
                        error_rect,
                        1.,
                        ui.style().visuals.error_fg_color.linear_multiply(0.1),
                    );
                }

                if textedit_response.changed() {
                    match simplez_assembler::assemble(&self.program) {
                        Ok(res) => {
                            self.memory = [0; 512];
                            self.assembler_output = "Successfully assembled program.".to_owned();
                            self.error_loc = None;
                            res.1
                                .into_iter()
                                .enumerate()
                                .for_each(|(i, x)| self.memory[i] = x);
                        }
                        Err(err) => {
                            self.assembler_output = err.to_string();
                            let err = match err {
                                nom::Err::Error(x) => x,
                                nom::Err::Failure(x) => x,
                                _ => unreachable!(),
                            };
                            let characters_consumed = self.program.len() - err.input.len();

                            let newlines = self
                                .program
                                .chars()
                                .take(characters_consumed)
                                .filter(|x| x == &'\n')
                                .count();

                            self.error_loc = Some(newlines);
                        }
                    }
                }

                ui.vertical_centered(|ui| {
                    ui.heading("Output");
                    ui.label(&self.assembler_output);
                });
            });
        });

        egui::Window::new("Memory").show(ctx, |ui| {
            egui::ScrollArea::vertical().show_viewport(ui, |ui, _viewport| {
                egui::Grid::new("asm_grid")
                    .num_columns(3)
                    .spacing([4., 4.])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.heading("Address (DEC)");
                        ui.heading("Contents (BIN)");
                        ui.heading("Instruction");
                        ui.end_row();
                        for (addr, word) in self.memory.iter().enumerate() {
                            ui.vertical_centered(|ui| ui.monospace(format!("[{}]", addr)));
                            ui.vertical_centered(|ui| ui.monospace(format!("{:012b}", word)));
                            ui.vertical_centered(|ui| {
                                ui.monospace(format!("{}", Instruction::from(*word)))
                            });
                            ui.end_row();
                        }
                    });
            });
        });
    }
}
