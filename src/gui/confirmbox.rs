use egui::Ui;

use super::Display;


pub trait Action<C> {
    fn doit(&mut self, apctx: &mut C);
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

impl<C, A: Action<C>> Display<C, bool> for ConfirmBox<A> {
    fn display(&mut self, _ctx: &egui::Context, ui: &mut Ui, apctx: &mut C) -> bool {
        let mut close  = false;
        ui.label(&self.msg);
        ui.horizontal(|ui| {
            let ok = ui.button("Yes");
            let cancel = ui.button("No");
            if ok.clicked() {
                self.action.doit(apctx);
            }
            close = ok.clicked() || cancel.clicked();
        });

        !close
    }
}