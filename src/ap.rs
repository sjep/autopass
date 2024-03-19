use egui::{Button, Color32, Label, Layout, SelectableLabel, Separator, TextEdit, Ui, ViewportBuilder};

use gui::{confirmbox::{Action, ConfirmBox}, msgbox::launch_msgbox, pwdprompt::prompt_password, validator::{textedit, NotEmpty, Validator}, Display, Windowed};
use pass::{api, spec::{service_v1::ServiceEntryV1, Serializable}};

mod gui;


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

struct ApCtx {
    masterpwd: String,
    refresh_service_list: bool
}

impl ApCtx {
    fn new(masterpwd: String) -> Self {
        Self {
            masterpwd,
            refresh_service_list: false
        }
    }
}

struct DeleteService {
    service: String
}

impl Action<ApCtx> for DeleteService {
    fn doit(&mut self, apctx: &mut ApCtx) {
        if let Err(e) = api::delete(&self.service) {
            eprintln!("Error deleting service {}: {}", self.service, e);
        } else {
            apctx.refresh_service_list = true;
        }
    }
}

struct CurrentService {
    entry: ServiceEntryV1,
    show_pass: bool,
    copied: bool,
    newkey: String,
    newval: String,
    confirm: Windowed<ConfirmBox<DeleteService>>,
}

impl CurrentService {
    fn new(entry: ServiceEntryV1) -> Self {
        Self {
            entry,
            show_pass: false,
            copied: false,
            newkey: String::new(),
            newval: String::new(),
            confirm: Windowed::new()
        }
    }

    fn savekvs(&mut self, apctx: &mut ApCtx) {
        if self.newkey.len() == 0 || self.newval.len() == 0 {
            return;
        }
        api::set_kvs(self.entry.name(), &apctx.masterpwd, &[(&self.newkey, &self.newval)], false)
            .expect("Error saving key value");
        self.newkey = String::new();
        self.newval = String::new();

        self.entry = api::get_all(self.entry.name(), &apctx.masterpwd)
            .expect("Unable to reset entry after setting kvs");
    }
}

impl Display<ApCtx, bool> for CurrentService {
    fn display(&mut self, ctx: &egui::Context, ui: &mut Ui, apctx: &mut ApCtx) -> bool {
        self.confirm.display(ctx, ui, apctx);
        
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

        /* Kvs section */
        let kvs = self.entry.get_kvs();
        ui.add(Separator::default());
        for (key, value) in kvs {
            ui.horizontal(|ui| {
                ui.add(Label::new(key));
                ui.add(Label::new("="));
                ui.add(Label::new(value)
                    .truncate(true));
                ui.scope(|ui| {
                    ui.visuals_mut().override_text_color = Some(Color32::DARK_RED);
                    if ui.add(Button::new("X")).clicked() {
                        println!("Delete kv pair");
                    }
                });
            });
        }

        ui.horizontal(|ui| {
            textedit(ui, &mut self.newkey, &NotEmpty, |te, _valid| {
                te.desired_width(50.0)
            });
            ui.add(Label::new("="));
            textedit(ui, &mut self.newval, &NotEmpty, |te, _valid| {
                te.desired_width(50.0)
            });
            let save = Button::new("Save");
            let enabled = NotEmpty.valid(&self.newkey).is_ok() && NotEmpty.valid(&self.newval).is_ok();
            if ui.add_enabled(enabled, save).clicked() {
                self.savekvs(apctx);
            }
        });
        ui.add(Separator::default());
        
        /* Service level buttons */
        ui.horizontal(|ui| {
            if ui.add(Button::new("Hide Service")).clicked() {
                keep = false
            }
            if ui.add(Button::new("Delete")).clicked() {
                self.confirm.set(
                    "Delete Service".to_owned(), 
                    ConfirmBox::new(
                        format!("Are you sure you want to delete service {}", self.entry.name()),
                        DeleteService { service: self.entry.name().to_owned() }
                    )
                );
            }
        });

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
    current: Option<CurrentService>,
    services: Vec<String>,
    newservice: Option<NewService>,
    ctx: ApCtx
}

impl ApApp {
    fn new(pwd: String) -> Self {
        let services: Vec<String> = api::list(&pwd);

        Self {
            current: None,
            services,
            newservice: None,
            ctx: ApCtx::new(pwd)
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
                                api::get_all(service, &self.ctx.masterpwd).ok().map(|se| CurrentService::new(se))
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
                    if self.ctx.refresh_service_list {
                        self.services = api::list(&self.ctx.masterpwd);
                        self.ctx.refresh_service_list = false;
                        self.current = None;
                    } else if !se.display(ctx, ui, &mut self.ctx) {
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