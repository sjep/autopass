use egui::Ui;

use super::Display;

pub struct MsgBox {
    msg: String
}

impl MsgBox {
    pub fn new(msg: String) -> Self {
        Self { msg }
    }
}

impl Display<(), bool> for MsgBox {
    fn display(&mut self, _ctx: &egui::Context, ui: &mut Ui, _apctx: &mut ()) -> bool {
        let mut close  = false;
        ui.label(&self.msg);
        ui.vertical_centered(|ui| {
            let response = ui.button("Ok");
            response.request_focus();
            close = response.clicked();
        });

        !close
    }
}

impl eframe::App for MsgBox {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.display(ctx, ui, &mut ()) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }
}

pub fn launch_msgbox(msg: String, app_name: String) {
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size((200.0, 50.0));
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport = viewport;
    let msgbox = MsgBox::new(msg);
    eframe::run_native(&app_name, native_options, Box::new(|_cc| Box::new(msgbox)))
        .unwrap();
}
