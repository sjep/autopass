use egui::{Button, Color32, Label, Layout, SelectableLabel, Separator, Ui, ViewportBuilder};

use gui::{confirmbox::{Action, ConfirmBox}, msgbox::launch_msgbox, pwdprompt::prompt_password, validator::{textedit, NotEmpty, NotInList, Validator}, Display, Windowed};
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
    services: Vec<String>,
    refresh_service: bool,
    refresh_service_list: bool,
}

impl ApCtx {
    fn new(masterpwd: String, services: Vec<String>) -> Self {
        Self {
            masterpwd,
            services,
            refresh_service: false,
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

struct KvDelete {
    service: String,
    key: String
}

impl Action<ApCtx> for KvDelete {
    fn doit(&mut self, apctx: &mut ApCtx) {
        if let Ok(entry) = api::get_all(&self.service, &apctx.masterpwd) {
            let mut kvs = vec![];
            for (key, value) in entry.get_kvs() {
                if key != &self.key {
                    kvs.push((key.as_str(), value.as_str()));
                }
            }
            if let Err(e) = api::set_kvs(&self.service, &apctx.masterpwd, &kvs, true) {
                eprintln!("Failed to save kvs for service {}: {}", self.service, e);
            }
            apctx.refresh_service = true;
        }
    }
}

struct CurrentService {
    entry: ServiceEntryV1,
    show_pass: bool,
    copied: bool,
    newkey: String,
    newval: String,
    confirm: Windowed<ConfirmBox<Box<dyn Action<ApCtx>>>>,
}

impl CurrentService {
    fn new(service: &str, apctx: &ApCtx) -> Option<Self> {
        api::get_all(service, &apctx.masterpwd).ok().map(|entry| {
            Self {
                entry,
                show_pass: false,
                copied: false,
                newkey: String::new(),
                newval: String::new(),
                confirm: Windowed::new()
            }
        })
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
        self.confirm.display(ctx, apctx);
        
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
        let mut delkey = None;
        let mut kvs = self.entry.get_kvs().iter().collect::<Vec<(&String, &String)>>();
        kvs.sort();

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
                        delkey = Some(key.to_owned());
                    }
                });
            });
        }
        if let Some(key) = delkey {
            self.confirm.set(
                "Delete key/value pair".to_owned(),
                ConfirmBox::new(
                    format!("Are you sure you want to delete {}?", key),
                    Box::new(KvDelete { service: self.entry.name().to_owned(), key })
                )
            );
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
                        Box::new(DeleteService { service: self.entry.name().to_owned() })
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
}

impl Display<ApCtx, bool> for NewService {
    fn display(&mut self, _ctx: &egui::Context, ui: &mut Ui, apctx: &mut ApCtx) -> bool {
        let mut keep = true;
        textedit(ui, &mut self.name, &NotInList::new(&apctx.services), |te, _valid| {
            te
                .hint_text("Service Name")
        });

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
        keep
    }
}

struct ApApp {
    current: Option<CurrentService>,
    newservice: Windowed<NewService>,
    ctx: ApCtx
}

impl ApApp {
    fn new(pwd: String) -> Self {
        let services: Vec<String> = api::list(&pwd);

        Self {
            current: None,
            newservice: Windowed::new(),
            ctx: ApCtx::new(pwd, services)
        }
    }
}

impl eframe::App for ApApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        if self.ctx.refresh_service_list {
            self.ctx.services = api::list(&self.ctx.masterpwd);
            self.ctx.refresh_service_list = false;
            self.current = None;
        }

        if self.ctx.refresh_service {
            let service = self.current.as_ref().map(|se| se.entry.name());
            if let Some(service) = service {
                self.current = CurrentService::new(service, &self.ctx);
            }
            self.ctx.refresh_service = false;
        }

        self.newservice.display(ctx, &mut self.ctx);

        egui::SidePanel::left("services")
            .resizable(false)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for service in self.ctx.services.iter() {
                        let selected = self.current.as_ref().map(|se| {
                            se.entry.get_name() == service
                        }).unwrap_or(false);
                        let resp = ui.add(SelectableLabel::new(selected, service));
                        if resp.clicked() {
                            self.current = if self.current.is_none() || self.current.as_ref().unwrap().entry.get_name() != service {
                                CurrentService::new(service, &self.ctx)
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
                    if !se.display(ctx, ui, &mut self.ctx) {
                        self.current = None;
                    }
                }
                None => {
                    ui.add(Label::new("Select service to view"));
                }
            }

            ui.with_layout(Layout::bottom_up(egui::Align::Center), |ui| {
                if ui.button("Add Service").clicked() {
                    self.newservice.set("New Service".to_owned(), NewService::new());
                }
            });
        });

    }
}