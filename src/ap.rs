use std::{ops::DerefMut, sync::{Arc, Mutex}};

use egui::ViewportBuilder;


fn main() {
    let pwd = PasswordPrompt::prompt_password();
    println!("Retrieved password: {}", pwd);
}

struct PasswordPrompt {
    password: Arc<Mutex<String>>,
}

impl PasswordPrompt {
    fn prompt_password() -> String {
        let viewport = ViewportBuilder::default()
            .with_inner_size((200.0, 50.0));
        let mut native_options = eframe::NativeOptions::default();
        native_options.viewport = viewport;
        let pwd = Arc::new(Mutex::new(String::new()));
        let cpwd = pwd.clone();
        eframe::run_native("PasswordPrompt", native_options, Box::new(|_cc| Box::new(PasswordPrompt{password: cpwd})))
            .unwrap();
        pwd.clone().lock().unwrap().clone()
    }
}

impl eframe::App for PasswordPrompt {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut guard = self.password.lock().unwrap();
            let pwd: &mut String = guard.deref_mut();
            let textedit = egui::TextEdit::singleline(pwd)
                .password(true)
                .hint_text("Master Password");
            let response = ui.add(textedit);
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                println!("Password: {}", pwd);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }
}