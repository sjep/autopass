use egui::{Button, Ui};

use crate::gui::validator::{Validator, textedit2};

use super::Display;

pub trait InputAction<C> {
    fn doit(&mut self, input: &str, apctx: &mut C);
}

pub struct InputBox<A, V> {
    msg: String,
    input: String,
    action: A,
    validation: V
}

impl<A, V> InputBox<A, V> {
    pub fn new(msg: String, action: A, validation: V) -> Self {
        Self { msg, input: String::new(), action, validation }
    }
}

impl<C, A: InputAction<C>, V: Validator<String>> Display<C, bool> for InputBox<A, V> {
    fn display(&mut self, _ctx: &egui::Context, ui: &mut Ui, apctx: &mut C) -> bool {
        let mut close  = false;
        ui.label(&self.msg);
        let (resp, _) = textedit2(ui, &mut self.input, &self.validation, |te, _| te);
        ui.memory_mut(|m| {
            m.request_focus(resp.id);
        });
        ui.horizontal(|ui| {
            let ok = Button::new("Ok");
            if ui.add(ok).clicked() {
                self.action.doit(&self.input, apctx);
                close = true;
            }
            let cancel = Button::new("Cancel");
            if ui.add(cancel).clicked() {
                close = true;
            }
        });

        !close
    }
}