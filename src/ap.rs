use egui::{Button, Color32, Label, Layout, SelectableLabel, Separator, Ui, ViewportBuilder};

use gui::{
    confirmbox::{Action, ConfirmBox},
    msgbox::launch_msgbox,
    pwdprompt::prompt_password,
    validator::{textedit, LengthBounds, NotEmpty, NotInList, ValidString},
    Display,
    Windowed
};
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

impl Action<ApCtx> for Box<DeleteService> {
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

impl Action<ApCtx> for Box<KvDelete> {
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

fn newpwdprompt(ui: &mut Ui, password: &mut Option<ValidString>) {
    ui.horizontal(|ui| {
        match password {
            Some(pwd) => {
                textedit(ui, pwd, None, |te, _valid| {
                    te
                        .password(true)
                        .interactive(true)
                        .hint_text("Service Password")
                });

                if ui.button("Auto").clicked() {
                    password.take();
                }
            }
            None => {
                let mut stub = String::new();
                let newpassword = egui::TextEdit::singleline(&mut stub)
                    .password(true)
                    .interactive(false)
                    .hint_text("Password Auto Generated");
                ui.add(newpassword);
                if ui.button("Manual").clicked() {
                    password.replace(ValidString::new(Box::new(LengthBounds::new(8, 16))));
                }
            }
        }
    });
}

struct PasswordRefresh {
    service: String,
    password: Option<ValidString>,
}

impl PasswordRefresh {
    fn new(service: String) -> Self {
        Self {
            service,
            password: None
        }
    }

    fn refresh_password(&self, apctx: &mut ApCtx) {
        let pwd = self.password.as_ref().map(|pwd| pwd.string());
        if let Err(e) = api::upgrade(&self.service, &apctx.masterpwd, pwd) {
            eprintln!("Error updating password for service {}: {}", self.service, e);
        }
        apctx.refresh_service = true;
    }
}

impl Display<ApCtx, bool> for PasswordRefresh {
    fn display(&mut self, _ctx: &egui::Context, ui: &mut Ui, apctx: &mut ApCtx) -> bool {
        let mut keep = true;
        newpwdprompt(ui, &mut self.password);

        ui.horizontal(|ui| {
            ui.with_layout(Layout::left_to_right(egui::Align::Max), |ui| {
                let save = Button::new("Save");
                let enabled = self.password.as_ref().map_or(true, |vs| vs.is_valid());
                if ui.add_enabled(enabled, save).clicked() {
                    self.refresh_password(apctx);
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

struct CurrentService {
    entry: ServiceEntryV1,
    show_pass: bool,
    copied: bool,
    newkey: ValidString,
    newval: ValidString,
    confirm: Windowed<Box<dyn Display<ApCtx, bool>>>,
}

impl CurrentService {
    fn new(service: &str, apctx: &ApCtx) -> Option<Self> {
        api::get_all(service, &apctx.masterpwd).ok().map(|entry| {
            Self {
                entry,
                show_pass: false,
                copied: false,
                newkey: ValidString::new(Box::new(NotEmpty)),
                newval: ValidString::new(Box::new(NotEmpty)),
                confirm: Windowed::new()
            }
        })
    }

    fn savekvs(&mut self, apctx: &mut ApCtx) {
        if !self.newkey.is_valid() || !self.newval.is_valid() {
            return;
        }
        api::set_kvs(self.entry.name(), &apctx.masterpwd, &[(&self.newkey.string(), &self.newval.string())], false)
            .expect("Error saving key value");
        self.newkey = ValidString::new(Box::new(NotEmpty));
        self.newval = ValidString::new(Box::new(NotEmpty));

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
            ui.with_layout(Layout::right_to_left(egui::Align::Max), |ui| {
                let incrpwd = Button::new("Reset Password");
                if ui.add(incrpwd).clicked() {
                    self.confirm.set(
                        format!("New password for {}", self.entry.name()),
                        Box::new(PasswordRefresh::new(self.entry.name().to_owned()))
                    );
                }
            })
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
                Box::new(ConfirmBox::new(
                    format!("Are you sure you want to delete {}?", key),
                    Box::new(KvDelete { service: self.entry.name().to_owned(), key })
                )
            ));
        }

        ui.horizontal(|ui| {
            textedit(ui, &mut self.newkey, None, |te, _valid| {
                te.desired_width(50.0)
            });
            ui.add(Label::new("="));
            textedit(ui, &mut self.newval, None, |te, _valid| {
                te.desired_width(50.0)
            });
            let save = Button::new("Save");
            let enabled = self.newkey.is_valid() && self.newval.is_valid();
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
            if ui.add(Button::new("Delete Service")).clicked() {
                self.confirm.set(
                    "Delete Service".to_owned(), 
                    Box::new(ConfirmBox::new(
                        format!("Are you sure you want to delete service {}", self.entry.name()),
                        Box::new(DeleteService { service: self.entry.name().to_owned() })
                    ))
                );
            }
        });

        keep
    }
}

struct NewService {
    name: ValidString,
    password: Option<ValidString>,
    kvs: Vec<(String, String)>,
    newkvp: Option<(ValidString, ValidString)>
}

impl NewService {
    fn new() -> Self {
        Self { name: ValidString::new(Box::new(NotEmpty)), password: None, kvs: vec![], newkvp: None }
    }

    fn save(&self, apctx: &mut ApCtx) {
        if let Err(e) = api::new(
            self.name.string(),
            &apctx.masterpwd,
            &pass::hash::TextMode::NoWhiteSpace,
            16,
            &self.kvs,
            self.password.as_ref().map(|vs| vs.string())
        ) {
            eprintln!("Error saving new service {}: {}", self.name.string(), e);
        }
        apctx.refresh_service_list = true;
    }
}

impl Display<ApCtx, bool> for NewService {
    fn display(&mut self, _ctx: &egui::Context, ui: &mut Ui, apctx: &mut ApCtx) -> bool {
        let mut keep = true;
        textedit(ui, &mut self.name, Some(&NotInList::new(&apctx.services)), |te, _valid| {
            te
                .hint_text("Service Name")
        });

        ui.horizontal(|ui| {
            newpwdprompt(ui, &mut self.password);
        });

        ui.add(Separator::default());

        let mut delidx = None;
        for (idx, (key, value)) in self.kvs.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.add(Label::new(key));
                ui.add(Label::new("="));
                ui.add(Label::new(value)
                    .truncate(true));
                ui.scope(|ui| {
                    ui.visuals_mut().override_text_color = Some(Color32::DARK_RED);
                    if ui.add(Button::new("X")).clicked() {
                        delidx = Some(idx);
                    }
                });
            });
        }

        if let Some(idx) = delidx {
            self.kvs.remove(idx);
        }

        let mut rmnew = false;
        let mut savekv = false;
        match &mut self.newkvp {
            Some((key, val)) => {
                ui.horizontal(|ui| {
                    textedit(ui, key, None, |te, _valid| te.desired_width(50.0));
                    ui.add(Label::new("="));
                    textedit(ui, val, None, |te, _valid| te.desired_width(50.0));

                    let commit = Button::new("Commit");
                    let enabled = key.is_valid() && val.is_valid();
                    if ui.add_enabled(enabled, commit).clicked() {
                        savekv = true;
                    }
                    ui.scope(|ui| {
                        ui.visuals_mut().override_text_color = Some(Color32::DARK_RED);
                        if ui.add(Button::new("X")).clicked() {
                            rmnew = true;
                        }
                    });
                });
            }
            None => {
                if ui.add(Button::new("Add key/value")).clicked() {
                    self.newkvp = Some((ValidString::new(Box::new(NotEmpty)), ValidString::new(Box::new(NotEmpty))));
                }
            }
        }

        if rmnew {
            self.newkvp = None;
        }
        if savekv {
            if let Some((k, v)) = self.newkvp.take() {
                self.kvs.push((k.string().to_owned(), v.string().to_owned()));
            }
        }

        ui.add(Separator::default());

        ui.horizontal(|ui| {
            ui.with_layout(Layout::left_to_right(egui::Align::Max), |ui| {
                let save = Button::new("Save");
                let enabled = self.name.is_valid()
                    && self.password.as_ref().map_or(true, |vs| vs.is_valid())
                    && self.newkvp.is_none();
                if ui.add_enabled(enabled, save).clicked() {
                    self.save(apctx);
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
            .max_width(100.0)
            .show(ctx, |ui| {
                egui::TopBottomPanel::bottom("Bottom Left").min_height(25.0).show_separator_line(false).show_inside(ui, |ui| {
                    ui.with_layout(Layout::bottom_up(egui::Align::Center), |ui| {
                        let addservice = Button::new("Add Service");
                        if ui.add(addservice).clicked() {
                            self.newservice.set("New Service".to_owned(), NewService::new());
                        }
                    });
                });

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
        });

    }
}