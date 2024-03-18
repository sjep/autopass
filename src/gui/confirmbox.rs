use egui::Ui;

use super::Display;


pub trait Action {
    fn doit(&mut self);
}

pub struct ConfirmBox<A> {
    msg: String,
    action: A
}

impl<A> ConfirmBox<A> {
    pub fn new(msg: String, action: A) -> Self {
        Self { msg, action }
    }
}

impl<A: Action> Display<bool> for ConfirmBox<A> {
    fn display(&mut self, _ctx: &egui::Context, ui: &mut Ui) -> bool {
        let mut close  = false;
        ui.label(&self.msg);
        ui.horizontal(|ui| {
            let ok = ui.button("Yes");
            let cancel = ui.button("No");
            if ok.clicked() {
                self.action.doit();
            }
            close = ok.clicked() || cancel.clicked();
        });

        !close
    }
}