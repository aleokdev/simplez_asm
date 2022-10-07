use eframe::egui::{self, TextEdit};
use simplez_common::{Address, Instruction};
use twelve_bit::u12::U12;

use crate::highlighter;

#[derive(serde::Deserialize, serde::Serialize)]
struct AssemblerError {
    loc: usize,
    description: String,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    program: String,
    assembler_err: Option<AssemblerError>,
    context: simplez_interpreter::ExecutionContext,

    #[serde(skip)]
    executing: bool,
    #[serde(skip)]
    ran_program: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            program: String::new(),
            assembler_err: None,
            context: Default::default(),

            executing: false,
            ran_program: false,
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
        egui::SidePanel::left("state_panel")
            .resizable(true)
            .default_width(400.)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| ui.heading("Registers"));
                let heading_height = ui.text_style_height(&egui::TextStyle::Heading);
                ui.push_id("registers", |ui| {
                    egui_extras::TableBuilder::new(ui)
                        .striped(true)
                        .cell_layout(egui::Layout::centered_and_justified(
                            egui::Direction::LeftToRight,
                        ))
                        .columns(egui_extras::Size::relative(1. / 3.), 3)
                        .header(heading_height, |mut header| {
                            header.col(|ui| {
                                ui.heading("ACC (Z)");
                            });
                            header.col(|ui| {
                                ui.heading("PC");
                            });
                            header.col(|ui| {
                                ui.heading("IR");
                            });
                        })
                        .body(|mut body| {
                            body.row(16., |mut row| {
                                row.col(|ui| {
                                    ui.monospace(format!(
                                        "{} ({})",
                                        u16::from(self.context.acc),
                                        self.context.zero() as u8
                                    ));
                                });
                                row.col(|ui| {
                                    ui.monospace(format!("[{}]", u16::from(self.context.pc.0)));
                                });
                                row.col(|ui| {
                                    ui.monospace(format!(
                                        "{} ({})",
                                        u16::from(self.context.ir),
                                        Instruction::from(self.context.ir)
                                    ));
                                });
                            });
                        });
                });

                ui.vertical_centered(|ui| ui.heading("Execution"));
                ui.horizontal(|ui| {
                    if ui
                        .button(if self.executing { "Pause" } else { "Run" })
                        .clicked()
                    {
                        self.executing = !self.executing;
                    }
                    if ui.button("Reset").clicked() {
                        self.context.reset_registers();
                    }
                    if ui
                        .add_enabled(!self.executing, egui::Button::new("Step"))
                        .clicked()
                    {
                        self.ran_program = true;
                        self.context.step();
                    }
                });

                ui.vertical_centered(|ui| ui.heading("Memory"));
                let text_color = ui.style().visuals.text_color();
                let mut loc_rect = ui.available_rect_before_wrap();
                let mut render_loc_rect = false;
                egui_extras::TableBuilder::new(ui)
                    .striped(true)
                    .cell_layout(egui::Layout::centered_and_justified(
                        egui::Direction::LeftToRight,
                    ))
                    .column(egui_extras::Size::relative(1. / 3.))
                    .column(egui_extras::Size::remainder())
                    .column(egui_extras::Size::relative(1. / 3.))
                    .header(heading_height, |mut header| {
                        header.col(|ui| {
                            ui.heading("Address (DEC)");
                        });
                        header.col(|ui| {
                            ui.heading("Contents (BIN)");
                        });
                        header.col(|ui| {
                            ui.heading("Instruction");
                        });
                    })
                    .body(|body| {
                        body.rows(16., self.context.memory().0.len(), |addr, mut row| {
                            let addr = Address(U12::from_u16(addr as u16));
                            let word = self.context.memory()[addr];
                            let color = self
                                .context
                                .last_modifications()
                                .iter()
                                .enumerate()
                                .find(|(_, a)| addr == **a)
                                .map(|(idx, _)| {
                                    let color1 = egui::Rgba::from(text_color);
                                    let color2 = egui::Rgba::from(egui::Color32::RED);
                                    let f = idx as f32 / 5.;
                                    egui::Color32::from(egui::Rgba::from_rgb(
                                        color1.r() * f + color2.r() * (1. - f),
                                        color1.g() * f + color2.g() * (1. - f),
                                        color1.b() * f + color2.b() * (1. - f),
                                    ))
                                })
                                .unwrap_or(text_color);
                            row.col(|ui| {
                                let response = ui.monospace(format!("[{}]", addr));

                                if addr == self.context.pc {
                                    loc_rect.min.y = response.rect.min.y;
                                    loc_rect.set_height(response.rect.height());
                                    render_loc_rect = true;
                                }
                            });
                            row.col(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("{:012b}", u16::from(word)))
                                        .monospace()
                                        .color(color),
                                );
                            });
                            row.col(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("{}", Instruction::from(word)))
                                        .monospace()
                                        .color(color),
                                );
                            });
                        });
                    });

                if render_loc_rect {
                    ui.painter().rect_filled(
                        loc_rect,
                        1.,
                        ui.style()
                            .visuals
                            .widgets
                            .active
                            .bg_fill
                            .linear_multiply(0.7),
                    );
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::warn_if_debug_build(ui);
            ui.add_enabled_ui(!self.ran_program, |ui| {
                let theme = highlighter::CodeTheme::from_memory(ui.ctx());

                let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                    let mut layout_job = highlighter::highlight(ui.ctx(), &theme, string);
                    layout_job.wrap.max_width = wrap_width;
                    ui.fonts().layout_job(layout_job)
                };

                #[derive(Clone, Copy, Default)]
                struct CodeLineCount(usize);
                #[derive(Default)]
                struct CodeLineCounter;
                type CodeLineCache = egui::util::cache::FrameCache<CodeLineCount, CodeLineCounter>;

                impl egui::util::cache::ComputerMut<&str, CodeLineCount> for CodeLineCounter {
                    fn compute(&mut self, code: &str) -> CodeLineCount {
                        CodeLineCount(code.chars().filter(|x| x == &'\n').count())
                    }
                }

                let lines_of_code = ui
                    .memory()
                    .caches
                    .cache::<CodeLineCache>()
                    .get(&self.program)
                    .0;

                let textedit_response = egui::ScrollArea::vertical()
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            let mut style = (**ui.style()).clone();
                            style.spacing.item_spacing.y = 0.;
                            ui.set_style(style);
                            egui_extras::TableBuilder::new(ui)
                                .column(egui_extras::Size::exact(20.)).scroll(false).clip(false).striped(true)
                                .body(|mut body| {
                                    body.rows(
                                        highlighter::CODE_EDITOR_LINE_HEIGHT,
                                        lines_of_code,
                                        |row_index, mut row| {
                                            row.col(|ui| {
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Max,
                                                    ),
                                                    |ui| ui.label(egui::RichText::new(format!("{}", row_index + 1)).font(egui::FontId::monospace(highlighter::CODE_EDITOR_LINE_HEIGHT))),
                                                );
                                            });
                                        },
                                    )
                                });

                            ui.add(
                                TextEdit::multiline(&mut self.program)
                                    .code_editor()
                                    .desired_width(ui.available_width())
                                    .desired_rows(10)
                                    .frame(false)
                                    .layouter(&mut layouter),
                            )
                        })
                    })
                    .inner
                    .inner;

                if let Some(err) = &mut self.assembler_err {
                    let mut error_rect = textedit_response.rect;
                    error_rect.min.y =
                        error_rect.min.y + highlighter::CODE_EDITOR_LINE_HEIGHT * err.loc as f32;
                    error_rect.set_height(highlighter::CODE_EDITOR_LINE_HEIGHT);

                    ui.allocate_rect(error_rect, egui::Sense::hover())
                        .on_hover_text(&err.description);
                    ui.painter().rect_filled(
                        error_rect,
                        1.,
                        ui.style().visuals.error_fg_color.linear_multiply(0.1),
                    );
                }

                if textedit_response.changed() {
                    self.assemble_program();
                }
            });
            if self.ran_program {
                egui::Window::new("Reassemble program")
                    .anchor(egui::Align2::CENTER_CENTER, [0., 0.])
                    .title_bar(false)
                    .show(ctx, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.heading("Program has been ran since last assembly.");
                            if ui.button("Click here to reassemble.").clicked() {
                                self.ran_program = false;
                                self.executing = false;
                                self.context.reset_registers();
                                self.assemble_program();
                            }
                        })
                    });
            }
        });

        if self.executing {
            self.ran_program = true;
            self.context.step();
            ctx.request_repaint();
        }
    }
}

impl App {
    fn assemble_program(&mut self) {
        match simplez_assembler::assemble(&self.program) {
            Ok(res) => {
                self.context.set_memory(res);
                self.assembler_err = None;
            }
            Err(err) => {
                let characters_consumed = self.program.len() - err.input.len();

                let newlines = self
                    .program
                    .chars()
                    .take(characters_consumed)
                    .filter(|x| x == &'\n')
                    .count();

                self.assembler_err = Some(AssemblerError {
                    description: format!("{:?}", err),
                    loc: newlines,
                });
            }
        }
    }
}
