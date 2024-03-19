use std::{cell::RefCell, ops::DerefMut, rc::Rc};

use egui::ViewportBuilder;



pub fn prompt_password(msg: &str) -> String {
    let viewport = ViewportBuilder::default()
        .with_inner_size((200.0, 50.0));
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport = viewport;
    let pwdstate = PwdState { msg: msg.to_owned(), pwd: String::new() };
    let pwd = Rc::new(RefCell::new(pwdstate));
    let cpwd = pwd.clone();
    eframe::run_native("PasswordPrompt", native_options, Box::new(|_cc| Box::new(PasswordPrompt{password: cpwd})))
        .unwrap();
    pwd.take().pwd
}

#[derive(Default)]
struct PwdState {
    msg: String,
    pwd: String
}

struct PasswordPrompt {
    password: Rc<RefCell<PwdState>>,
}

impl eframe::App for PasswordPrompt {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut pwdref = (*self.password).borrow_mut();
            let pwd = pwdref.deref_mut();
            let textedit = egui::TextEdit::singleline(&mut pwd.pwd)
                .password(true)
                .hint_text(&pwd.msg);
            let response = ui.add(textedit);
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ui.memory_mut(|m| {
                    m.request_focus(response.id);
                });
            }
        });
    }
}