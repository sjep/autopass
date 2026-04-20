use std::{cell::RefCell, ops::DerefMut, rc::Rc};

use egui::ViewportBuilder;

use super::validator::{textedit2, Validator};



pub fn prompt_input<V: Validator<String>>(app_name: &str, size: (f32, f32), label: Option<String>, hint: &str, validation: V, is_password: bool) -> String {
    let viewport = ViewportBuilder::default()
        .with_inner_size(size);
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport = viewport;
    let state = InputState { label, hint: hint.to_owned(), input: String::new(), is_password };
    let state = Rc::new(RefCell::new(state));
    let cstate = state.clone();
    eframe::run_native(app_name, native_options, Box::new(|_cc| Ok(Box::new(InputPrompt{validation, state: cstate}))))
        .unwrap();
    state.take().input.to_owned()
}

#[derive(Default)]
struct InputState {
    label: Option<String>,
    hint: String,
    input: String,
    is_password: bool
}


struct InputPrompt<V> {
    validation: V,
    state: Rc<RefCell<InputState>>,
}

impl<V: Validator<String>> eframe::App for InputPrompt<V> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut stateref = (*self.state).borrow_mut();
            let state = stateref.deref_mut();
            if let Some(label) = &state.label {
                ui.label(label);
            }
            let hint = state.hint.clone();
            let is_password = state.is_password;
            let (response, valid) = textedit2(ui, &mut state.input, &self.validation, |te, _| {
                te
                    .password(is_password)
                    .hint_text(&hint)
            });
            if response.lost_focus()
                && valid
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