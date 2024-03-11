use std::{cell::RefCell, ops::DerefMut, rc::Rc};

use egui::{Button, Label, SelectableLabel, ViewportBuilder};
use pass::{api, spec::service_v1::ServiceEntryV1};


fn main() {
    let pwd = prompt_password();
    if pwd != "" {
        launch_ap(pwd);
    }
}

fn launch_ap(pwd: String) {
    let viewport = ViewportBuilder::default()
        .with_inner_size((500.0, 500.0));
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport = viewport;


    eframe::run_native("AutoPass", native_options, Box::new(|_cc| Box::new(ApApp::new(pwd)))).unwrap();
}


struct CurrentService {
    entry: ServiceEntryV1,
    show_pass: bool
}

impl CurrentService {
    fn new(entry: ServiceEntryV1) -> Self {
        Self {
            entry,
            show_pass: false
        }
    }
}

struct ApApp {
    pwd: String,
    current: Option<CurrentService>,
    services: Vec<String>
}

impl ApApp {
    fn new(pwd: String) -> Self {
        let services: Vec<String> = api::list(&pwd);

        Self {
            pwd,
            current: None,
            services
        }
    }
}

impl eframe::App for ApApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("services")
            .resizable(false)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for service in self.services.iter() {
                        let selected = self.current.as_ref().map(|se| {
                            se.entry.get_name() == service
                        }).unwrap_or(false);
                        let resp = ui.add(SelectableLabel::new(selected, service));
                        if resp.clicked() {
                            self.current = if self.current.is_none() || self.current.as_ref().unwrap().entry.get_name() != service {
                                api::get_all(service, &self.pwd).ok().map(|se| CurrentService::new(se))
                            } else {
                                None
                            };
                        }
                }
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            match &mut self.current {
                Some(se) => {
                    ui.add(Label::new(format!("Name: {}", se.entry.get_name())));
                    ui.horizontal(|ui| {
                        ui.add(Label::new("Password:"));
                        let resp = if se.show_pass {
                            ui.add(Button::new(se.entry.get_pass(false).unwrap()))
                        } else {
                            ui.add(Button::new("Show Password"))
                        };
                        if resp.clicked() {
                            se.show_pass = !se.show_pass;
                        }
                        if ui.add(Button::new("Copy")).clicked() {
                            se.entry.get_pass(true);
                        }
                    });
                    ui.add(Label::new(format!("Created: {}", se.entry.created())));
                    ui.add(Label::new(format!("Last Modified: {}", se.entry.modified())));
                    if ui.add(Button::new("Hide")).clicked() {
                        self.current = None;
                    }
                }
                None => {
                    ui.add(Label::new("Select service to view"));
                }
            }
        });

    }
}



fn prompt_password() -> String {
    let viewport = ViewportBuilder::default()
        .with_inner_size((200.0, 50.0));
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport = viewport;
    let pwd =Rc::new(RefCell::new(String::new()));
    let cpwd = pwd.clone();
    eframe::run_native("PasswordPrompt", native_options, Box::new(|_cc| Box::new(PasswordPrompt{password: cpwd})))
        .unwrap();
    pwd.take()
}

struct PasswordPrompt {
    password: Rc<RefCell<String>>,
}

impl eframe::App for PasswordPrompt {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut pwdref = (*self.password).borrow_mut();
            let pwd = pwdref.deref_mut();
            let textedit = egui::TextEdit::singleline(pwd)
                .password(true)
                .lock_focus(true)
                .hint_text("Master Password");
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