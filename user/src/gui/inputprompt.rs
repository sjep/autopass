use std::{cell::RefCell, ops::DerefMut, rc::Rc};

use egui::ViewportBuilder;

use super::validator::{textedit, ValidString, Validator};



pub fn prompt_input(app_name: &str, size: (f32, f32), label: Option<String>, hint: &str, validation: Box<dyn Validator<String>>, is_password: bool) -> String {
    let viewport = ViewportBuilder::default()
        .with_inner_size(size);
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport = viewport;
    let state = InputState { label, hint: hint.to_owned(), input: ValidString::new(validation), is_password };
    let state = Rc::new(RefCell::new(state));
    let cstate = state.clone();
    eframe::run_native(app_name, native_options, Box::new(|_cc| Box::new(InputPrompt{state: cstate})))
        .unwrap();
    state.take().input.to_owned()
}

#[derive(Default)]
struct InputState {
    label: Option<String>,
    hint: String,
    input: ValidString,
    is_password: bool
}


struct InputPrompt {
    state: Rc<RefCell<InputState>>,
}

impl eframe::App for InputPrompt {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut stateref = (*self.state).borrow_mut();
            let state = stateref.deref_mut();
            if let Some(label) = &state.label {
                ui.label(label);
            }
            let hint = state.hint.clone();
            let is_password = state.is_password;
            let response = textedit(ui, &mut state.input, None, |te, _| {
                te
                    .password(is_password)
                    .hint_text(&hint)
            });
            if response.lost_focus()
                && state.input.is_valid()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
            {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ui.memory_mut(|m| {
                    m.request_focus(response.id);
                });
            }
        });
    }
}