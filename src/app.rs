use eframe::egui::{self, Label, RichText, TextEdit};
use simplez_common::Instruction;

mod arrays {
    use std::{convert::TryInto, marker::PhantomData};

    use serde::{
        de::{SeqAccess, Visitor},
        ser::SerializeTuple,
        Deserialize, Deserializer, Serialize, Serializer,
    };
    pub fn serialize<S: Serializer, T: Serialize, const N: usize>(
        data: &[T; N],
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        let mut s = ser.serialize_tuple(N)?;
        for item in data {
            s.serialize_element(item)?;
        }
        s.end()
    }

    struct ArrayVisitor<T, const N: usize>(PhantomData<T>);

    impl<'de, T, const N: usize> Visitor<'de> for ArrayVisitor<T, N>
    where
        T: Deserialize<'de>,
    {
        type Value = [T; N];

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str(&format!("an array of length {}", N))
        }

        #[inline]
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            // can be optimized using MaybeUninit
            let mut data = Vec::with_capacity(N);
            for _ in 0..N {
                match (seq.next_element())? {
                    Some(val) => data.push(val),
                    None => return Err(serde::de::Error::invalid_length(N, &self)),
                }
            }
            match data.try_into() {
                Ok(arr) => Ok(arr),
                Err(_) => unreachable!(),
            }
        }
    }
    pub fn deserialize<'de, D, T, const N: usize>(deserializer: D) -> Result<[T; N], D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        deserializer.deserialize_tuple(N, ArrayVisitor::<T, N>(PhantomData))
    }
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    program: String,

    #[serde(with = "arrays")]
    memory: [u16; 512],
}

impl Default for App {
    fn default() -> Self {
        Self {
            program: String::new(),
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
            let textedit_response = ui.add(
                TextEdit::multiline(&mut self.program)
                    .code_editor()
                    .desired_width(ui.available_width() / 2.)
                    .desired_rows(40),
            );

            if textedit_response.changed() {
                if let Ok(res) = simplez_assembler::assemble(&self.program) {
                    self.memory = [0; 512];
                    res.1
                        .into_iter()
                        .enumerate()
                        .for_each(|(i, x)| self.memory[i] = x);
                }
            }
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
