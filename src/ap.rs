use std::{cell::RefCell, ops::DerefMut, rc::Rc};

use egui::{Button, Label, Layout, SelectableLabel, Separator, Ui, ViewportBuilder};
use pass::{api, spec::service_v1::ServiceEntryV1};


fn main() {
    let pwd = if api::empty() {
        let pwd1 = prompt_password("New master password");
        if pwd1 == "" {
            return;
        }
        let pwd2 = prompt_password("Confirm new master password");
        if pwd1 != pwd2 {
            launch_msgbox("Passwords didn't match".to_owned(), "Mismatch".to_owned());
            return;
        }
        pwd1
    } else {
        prompt_password("Master Password")
    };

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
    show_pass: bool,
    copied: bool,
}

impl CurrentService {
    fn new(entry: ServiceEntryV1) -> Self {
        Self {
            entry,
            show_pass: false,
            copied: false
        }
    }

    fn display(&mut self, ui: &mut Ui) -> bool {
        let mut keep = true;
        ui.add(Label::new(format!("Name: {}", self.entry.get_name())));
        ui.horizontal(|ui| {
            ui.add(Label::new("Password:"));
            let resp = if self.show_pass {
                ui.add(Button::new(self.entry.get_pass(false).unwrap()))
            } else {
                ui.add(Button::new("Show Password"))
            };
            if resp.clicked() {
                self.show_pass = !self.show_pass;
            }
            let copytxt = if self.copied {
                "Copied 👍"
            } else {
                "Copy"
            };
            if ui.add(Button::new(copytxt)).clicked() {
                self.entry.get_pass(true);
                self.copied = true;
            }
        });
        ui.add(Label::new(format!("Created: {}", self.entry.created())));
        ui.add(Label::new(format!("Last Modified: {}", self.entry.modified())));

        let kvs = self.entry.get_kvs();
        if kvs.len() > 0 {
            ui.add(Separator::default());
            
            for (key, value) in kvs {
                ui.horizontal(|ui| {
                    ui.add(Label::new(key));
                    ui.add(Separator::default());
                    ui.add(Label::new(value));
                });
            }
            ui.add(Separator::default());
        }
        if ui.add(Button::new("Hide Service")).clicked() {
            keep = false
        }
        keep
    }
}

struct NewService {
    name: String,
    password: Option<String>,
}

impl NewService {
    fn new() -> Self {
        Self { name: String::new(), password: None }
    }

    fn window(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) -> bool {
        let mut keep = true;
        egui::Window::new("New Service")
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                let servicename = egui::TextEdit::singleline(&mut self.name)
                    .hint_text("Service Name");
                ui.add(servicename);

                ui.horizontal(|ui| {
                    match &mut self.password {
                        Some(pwd) => {
                            let newpassword = egui::TextEdit::singleline(pwd)
                                .password(true)
                                .interactive(true)
                                .hint_text("Service Password");
                            ui.add(newpassword);
                            if ui.button("Auto").clicked() {
                                self.password = None;
                            }
                        }
                        None => {
                            let mut stub = String::new();
                            let newpassword = egui::TextEdit::singleline(&mut stub)
                                .password(true)
                                .interactive(false)
                                .hint_text("Auto Generated");
                            ui.add(newpassword);
                            if ui.button("Manual").clicked() {
                                self.password = Some(String::new());
                            }
                        }
                    }

                });
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::left_to_right(egui::Align::Max), |ui| {
                        if ui.button("Save").clicked() {
                            keep = false;
                        }
                    });
                    ui.with_layout(Layout::right_to_left(egui::Align::Max), |ui| {
                        if ui.button("Cancel").clicked() {
                            keep = false;
                        }
                    });
                });
            }
        );
        keep
    }
}

struct ApApp {
    pwd: String,
    current: Option<CurrentService>,
    services: Vec<String>,
    newservice: Option<NewService>
}

impl ApApp {
    fn new(pwd: String) -> Self {
        let services: Vec<String> = api::list(&pwd);

        Self {
            pwd,
            current: None,
            services,
            newservice: None
        }
    }
}

impl eframe::App for ApApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Some(ns) = &mut self.newservice {
            if !ns.window(ctx, frame) {
                self.newservice = None;
            }
        }

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
                    if !se.display(ui) {
                        self.current = None;
                    }
                }
                None => {
                    ui.add(Label::new("Select service to view"));
                }
            }

            ui.with_layout(Layout::bottom_up(egui::Align::Center), |ui| {
                if ui.button("Add Service").clicked() {
                    self.newservice = Some(NewService::new());
                }
            });
        });

    }
}

fn prompt_password(msg: &'static str) -> String {
    let viewport = ViewportBuilder::default()
        .with_inner_size((200.0, 50.0));
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport = viewport;
    let pwd = Rc::new(RefCell::new((String::new(), msg)));
    let cpwd = pwd.clone();
    eframe::run_native("PasswordPrompt", native_options, Box::new(|_cc| Box::new(PasswordPrompt{password: cpwd})))
        .unwrap();
    pwd.take().0
}

struct PasswordPrompt {
    password: Rc<RefCell<(String, &'static str)>>,
}

impl eframe::App for PasswordPrompt {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut pwdref = (*self.password).borrow_mut();
            let pwd = pwdref.deref_mut();
            let textedit = egui::TextEdit::singleline(&mut pwd.0)
                .password(true)
                .lock_focus(true)
                .hint_text(pwd.1);
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

fn launch_msgbox(msg: String, app_name: String) {
    let viewport = ViewportBuilder::default()
        .with_inner_size((200.0, 50.0));
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport = viewport;
    let msgbox = MsgBox { msg };
    eframe::run_native(&app_name, native_options, Box::new(|_cc| Box::new(msgbox)))
        .unwrap();
}

struct MsgBox {
    msg: String
}

impl eframe::App for MsgBox {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(&self.msg);
            ui.with_layout(Layout::bottom_up(egui::Align::Center), |ui| {
                let response = ui.button("Ok");
                response.request_focus();
                if response.clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }
}