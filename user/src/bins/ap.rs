
use egui::{Button, Color32, Label, Layout, RichText, SelectableLabel, Separator, Ui, ViewportBuilder};

use pass::{api::APError, gui::{
    confirmbox::{Action, ConfirmBox}, inputprompt::prompt_input, msgbox::launch_msgbox, servicelist::ServiceList, validator::{textedit2, LengthBounds, NotEmpty, NotInList, Validator}, Display, Windowed
}, spec::{IdentityType, ServiceType}};
use pass::{api, spec::Serializable};


fn main() -> Result<(), APError> {
    let empty = api::empty()?;
    let pwd = if empty {
        let pwd1 = prompt_input(
            "Password Prompt",
            (200.0, 50.0),
            None,
            "New master password",
            Box::new(()),
            true);
        if pwd1 == "" {
            return Ok(());
        }
        let pwd2 = prompt_input(
            "Password Prompt",
            (200.0, 50.0),
            None,
            "Confirm new master password",
            Box::new(()),
            true);
        if pwd1 != pwd2 {
            launch_msgbox("Passwords didn't match".to_owned(), "Mismatch".to_owned());
            return Ok(());
        }
        let username = prompt_input(
            "Identity Prompt",
            (200.0, 100.0),
            Some("One last thing: provide an identifier (username) for yourself for sharing purposes".to_owned()),
            "Username",
            Box::new(NotEmpty),
            false);
        api::init::<&str>(&username, &pwd1, &[])?;
        pwd1
    } else {
        prompt_input("Password Prompt", (200.0, 50.0), None, "Master Password", Box::new(()), true)
    };

    if pwd != "" {
        launch_ap(pwd);
    }
    Ok(())
}

fn launch_ap(pwd: String) {
    let viewport = ViewportBuilder::default()
        .with_inner_size((500.0, 500.0));
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport = viewport;

    eframe::run_native("AutoPass", native_options, Box::new(|_cc| Ok(Box::new(ApApp::new(pwd))))).unwrap();
}

struct ApCtx {
    username: String,
    masterpwd: String,
    services: ServiceList,
    refresh_service: bool,
    refresh_service_list: bool,
    set_service: Option<Option<Current>>, // First optional: are we setting anything, second optional: what we're setting to
}

impl ApCtx {
    fn new(username: String, masterpwd: String, services: ServiceList) -> Self {
        Self {
            username,
            masterpwd,
            services,
            refresh_service: false,
            refresh_service_list: false,
            set_service: None,
        }
    }
}

struct DeleteService {
    service: String
}

impl Action<ApCtx> for Box<DeleteService> {
    fn doit(&mut self, apctx: &mut ApCtx) {
        if let Err(e) = api::delete(&self.service, &apctx.masterpwd) {
            eprintln!("Error deleting service {}: {}", self.service, e);
        } else {
            apctx.refresh_service_list = true;
            apctx.set_service = Some(None);
        }
    }
}

struct KvDelete {
    service: Option<String>, // None for id kv delete
    key: String
}

impl KvDelete {
    fn save(&self, existing: &[(String, String)], apctx: &mut ApCtx) {
        let mut kvs = vec![];
        for (key, value) in existing {
            if key != &self.key {
                kvs.push((key.as_str(), value.as_str()));
            }
        }

        match &self.service {
            Some(s) => api::set_kvs(s, &apctx.masterpwd, &kvs, true),
            None => api::set_kvs_id(&apctx.masterpwd, &kvs, true)
        }.unwrap_or_else(|e| {
            panic!("Failed to save kvs: {}", e);
        })
    }
}

impl Action<ApCtx> for Box<KvDelete> {
    fn doit(&mut self, apctx: &mut ApCtx) {
        match &self.service {
            Some(s) => api::get_all(s, &apctx.masterpwd)
                .map(|s| self.save(s.get_kvs(), apctx)),
            None => api::get_id(&apctx.masterpwd)
                .map(|id| self.save(id.get_kvs(), apctx))
        }.unwrap_or_else(|e| {
            panic!("Unable to retrieve kvs: {}", e);
        });
        apctx.refresh_service = true;
    }
}

struct TagDelete {
    service: String,
    tag_to_remove: String
}

impl TagDelete {
    fn save(&self, apctx: &mut ApCtx) -> Result<(), APError> {
        let s = api::get_all(&self.service, &apctx.masterpwd)?;
        let mut tags = vec![];
        for t in s.get_tags() {
            if *t != self.tag_to_remove {
                tags.push(t);
            }
        }
        api::set_tags(&self.service, &apctx.masterpwd, &tags, true)
    }
}

impl Action<ApCtx> for Box<TagDelete> {
    fn doit(&mut self, apctx: &mut ApCtx) {
        if let Err(e) = self.save(apctx) {
            panic!("Unable to remove tag from service {}: {}", self.service, e);
        }
        apctx.refresh_service = true;
        apctx.refresh_service_list = true;
    }
}

fn newpwdprompt(ui: &mut Ui, password: &mut Option<String>) -> bool {
    ui.horizontal(|ui| {
        match password {
            Some(pwd) => {
                let (_, valid) = textedit2(ui, pwd, LengthBounds::new(8, 16), |te, _valid| {
                    te
                        .password(true)
                        .interactive(true)
                        .hint_text("Service Password")
                });

                if ui.button("Auto").clicked() {
                    password.take();
                }
                valid
            }
            None => {
                let mut stub = String::new();
                let newpassword = egui::TextEdit::singleline(&mut stub)
                    .password(true)
                    .interactive(false)
                    .hint_text("Password Auto Generated");
                ui.add(newpassword);
                if ui.button("Manual").clicked() {
                    password.replace(String::new());
                }
                true
            }
        }
    }).inner
}

struct PasswordRefresh {
    service: String,
    password: Option<String>,
}

impl PasswordRefresh {
    fn new(service: String) -> Self {
        Self {
            service,
            password: None
        }
    }

    fn refresh_password(&self, apctx: &mut ApCtx) {
        if let Err(e) = api::upgrade(&self.service, &apctx.masterpwd, self.password.as_ref().map(|s| s.as_str())) {
            eprintln!("Error updating password for service {}: {}", self.service, e);
        }
        apctx.refresh_service = true;
    }
}

impl Display<ApCtx, bool> for PasswordRefresh {
    fn display(&mut self, _ctx: &egui::Context, ui: &mut Ui, apctx: &mut ApCtx) -> bool {
        let mut keep = true;
        let enabled = newpwdprompt(ui, &mut self.password);

        ui.horizontal(|ui| {
            ui.with_layout(Layout::left_to_right(egui::Align::Max), |ui| {
                let save = Button::new("Save");
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

fn display_new_kvs(ui: &mut Ui, newkvp: &mut Option<(String, String)>, is_save: bool) -> bool {
    let mut rmnew = false;
    let mut savekv = false;
    match newkvp {
        Some((key, val)) => {
            ui.horizontal(|ui| {
                let (_, key_valid) = textedit2(ui, key, NotEmpty{}, |te, _valid| te.desired_width(50.0));
                ui.add(Label::new("="));
                let (_, val_valid) = textedit2(ui, val, NotEmpty{}, |te, _valid| te.desired_width(50.0));

                let msg = if is_save { "Save" } else { "Commit" };
                let commit = Button::new(msg);
                if ui.add_enabled(key_valid && val_valid, commit).clicked() {
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
                *newkvp = Some((String::new(), String::new()));
            }
        }
    }

    if rmnew {
        *newkvp = None;
    }
    savekv
}

fn display_kvs(ui: &mut Ui, service: Option<&str>, kvs: &[(String, String)], confirm: &mut Windowed<Box<dyn Display<ApCtx, bool>>>) {
    let mut delkey = None;

    for (key, value) in kvs {
        ui.horizontal(|ui| {
            ui.add(Label::new(key));
            ui.add(Label::new("="));
            ui.add(Label::new(value)
                .truncate());
            ui.scope(|ui| {
                ui.visuals_mut().override_text_color = Some(Color32::DARK_RED);
                if ui.add(Button::new("X")).clicked() {
                    delkey = Some(key.to_owned());
                }
            });
        });
    }
    if let Some(key) = delkey {
        confirm.set(
            "Delete key/value pair".to_owned(),
            Box::new(ConfirmBox::new(
                format!("Are you sure you want to delete {}?", key),
                Box::new(KvDelete { service: service.map(|s| s.to_owned()), key: key.to_owned() })
            )
        ));
    }
}

struct CurrentId {
    entry: IdentityType,
    newkvp: Option<(String, String)>,
    confirm: Windowed<Box<dyn Display<ApCtx, bool>>>,
}

impl CurrentId {
    fn new(apctx: &ApCtx) -> Self {
        let entry = api::get_id(&apctx.masterpwd)
            .expect("Unable to parse id entry");
        Self {
            entry,
            newkvp: None,
            confirm: Windowed::new()
        }
    }

    fn refresh(&mut self, apctx: &ApCtx) {
        let entry = api::get_id(&apctx.masterpwd)
            .expect("Unable to parse id entry");
        self.entry = entry;
    }

    fn savekvs(&mut self, apctx: &mut ApCtx) {
        if let Some((k, v)) = &self.newkvp {
            api::set_kvs_id(&apctx.masterpwd, &[(k, v)], false)
                .expect("Error saving key value");
            self.newkvp = None;

            self.refresh(apctx);
        }
    }

    fn dirty_msg(&self) -> Option<String> {
        match &self.newkvp {
            Some((nk, nv)) => {
                if nk.len() == 0 && nv.len() == 0 {
                    None
                } else {
                    Some(format!("Are you sure you want to discard unsaved key/value {} = {}?", nk, nv))
                }
            }
            _ => None
        }
    }
}

impl Display<ApCtx, bool> for CurrentId {
    fn display(&mut self, _ctx: &egui::Context, ui: &mut Ui, apctx: &mut ApCtx) -> bool {
        ui.add(Label::new(format!("Username: {}", self.entry.name())));
        ui.add(Label::new(format!("Created: {}", self.entry.created())));
        ui.add(Label::new(format!("Last Modified: {}", self.entry.modified())));

        /* Kvs section */
        let kvs = self.entry.get_kvs();

        ui.add(Separator::default());
        display_kvs(ui, None, &kvs, &mut self.confirm);

        if display_new_kvs(ui, &mut self.newkvp, true) {
            self.savekvs(apctx);
        }

        ui.add(Separator::default());
        true
    }
}

struct CurrentService {
    entry: ServiceType,
    show_pass: bool,
    copied: bool,
    newkvp: Option<(String, String)>,
    newtag: String,
    confirm: Windowed<Box<dyn Display<ApCtx, bool>>>,
}

impl CurrentService {
    fn new(service: &str, apctx: &ApCtx) -> Self {
        let entry = api::get_all(service, &apctx.masterpwd)
            .expect("Unable to parse service entry");
        Self {
            entry,
            show_pass: false,
            copied: false,
            newkvp: None,
            newtag: String::new(),
            confirm: Windowed::new()
        }
    }

    fn refresh(&mut self, apctx: &ApCtx) {
        let entry = api::get_all(self.entry.name(), &apctx.masterpwd)
            .expect("Unable to parse service entry");
        self.entry = entry;
        self.show_pass = false;
        self.copied = false;
    }

    fn savekvs(&mut self, apctx: &mut ApCtx) {
        if let Some((k, v)) = &self.newkvp {
            api::set_kvs(self.entry.name(), &apctx.masterpwd, &[(k, v)], false)
                .expect("Error saving key value");
            self.newkvp = None;

            self.refresh(apctx);
        }
    }

    fn savetag(&mut self, apctx: &mut ApCtx) {
        api::set_tags(self.entry.name(), &apctx.masterpwd, &[&self.newtag], false)
            .expect("Error saving tag");
        self.newtag = String::new();

        self.refresh(apctx);
        apctx.refresh_service_list = true;
    }

    fn dirty_msg(&self) -> Option<String> {
        match &self.newkvp {
            Some((nk, nv)) => {
                if nk.len() == 0 && nv.len() == 0 {
                    None
                } else {
                    Some(format!("Are you sure you want to discard unsaved key/value {} = {}?", nk, nv))
                }
            }
            _ => None
        }
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
        let kvs = self.entry.get_kvs();

        ui.add(Separator::default());
        display_kvs(ui, Some(self.entry.name()), &kvs, &mut self.confirm);

        if display_new_kvs(ui, &mut self.newkvp, true) {
            self.savekvs(apctx);
        }

        ui.add(Separator::default());

        ui.horizontal_wrapped(|ui| {
            let validations: &[&dyn Validator<String>] = &[&NotEmpty{}, &apctx.services.not_in_tags(self.entry.get_name())];
            let (_, tag_valid) = textedit2(ui, &mut self.newtag, validations, |te, _valid| te.desired_width(50.0));
            let addtag = Button::new("Add tag");
            if ui.add_enabled(tag_valid, addtag).clicked() {
                self.savetag(apctx);
            }

            if self.entry.get_tags().len() > 0 {
                ui.end_row();
            }

            for tag in self.entry.get_tags() {
                let tagbutton = Button::new(tag)
                    .corner_radius(5.0);
                let resp = ui.add(tagbutton)
                    .on_hover_text("Delete tag");
                if resp.clicked() {
                    self.confirm.set(
                        "Delete Tag".to_owned(), 
                        Box::new(ConfirmBox::new(
                            format!("Are you sure you want to delete tag {} from {}?", tag, self.entry.name()),
                            Box::new(TagDelete { service: self.entry.name().to_owned(), tag_to_remove: tag.to_owned() })
                        ))
                    );
                }
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
    name: String,
    password: Option<String>,
    kvs: Vec<(String, String)>,
    newkvp: Option<(String, String)>,
    tags: Vec<String>,
    newtag: Option<String>
}

impl NewService {
    fn new() -> Self {
        Self { name: String::new(), password: None, kvs: vec![], newkvp: None, tags: vec![], newtag: None }
    }

    fn save(&self, apctx: &mut ApCtx) {
        if let Err(e) = api::new(
            &self.name,
            &apctx.masterpwd,
            &pass::hash::TextMode::NoWhiteSpace,
            16,
            &self.kvs,
            &self.tags,
            self.password.as_ref().map(|s| s.as_str())
        ) {
            eprintln!("Error saving new service {}: {}", self.name, e);
        }
        apctx.refresh_service_list = true;
    }
}

impl Display<ApCtx, bool> for NewService {
    fn display(&mut self, _ctx: &egui::Context, ui: &mut Ui, apctx: &mut ApCtx) -> bool {
        let mut keep = true;
        let validations: &[&dyn Validator<String>] = &[&NotEmpty{}, &apctx.services.not_in_services()];
        let (_, name_valid) = textedit2(ui, &mut self.name, validations, |te, _valid| {
            te
                .hint_text("Service Name")
        });

        let pass_valid = ui.horizontal(|ui| {
            newpwdprompt(ui, &mut self.password)
        }).inner;

        ui.add(Separator::default());

        let mut delidx = None;
        for (idx, (key, value)) in self.kvs.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.add(Label::new(key));
                ui.add(Label::new("="));
                ui.add(Label::new(value)
                    .truncate());
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

        if display_new_kvs(ui, &mut self.newkvp, false) {
            if let Some((k, v)) = self.newkvp.take() {
                self.kvs.push((k, v));
            }
        }

        ui.add(Separator::default());

        ui.horizontal_wrapped(|ui| {
            let mut rmnewtag = false;

            match &mut self.newtag {
                Some(nt) => {
                    let validations: &[&dyn Validator<String>] = &[&NotEmpty{}, &NotInList::new(&self.tags)];
                    let (_, tag_valid) = textedit2(ui, nt, validations, |te, _valid| te.desired_width(50.0));
                    let addtag = Button::new("Add tag");
                    if ui.add_enabled(tag_valid, addtag).clicked() {
                        self.tags.push(nt.to_owned());
                        rmnewtag = true;
                    }
                    ui.scope(|ui| {
                        ui.visuals_mut().override_text_color = Some(Color32::DARK_RED);
                        if ui.add(Button::new("X")).clicked() {
                            rmnewtag = true;
                        }
                    });
                }
                None => {
                    if ui.add(Button::new("New tag")).clicked() {
                        self.newtag = Some(String::new());
                    }
                }
            }
            if rmnewtag {
                self.newtag = None;
            }

            if self.tags.len() > 0 {
                ui.end_row();
            }

            let mut torm = vec![];

            for (idx, tag) in self.tags.iter().enumerate() {
                let tagbutton = Button::new(tag).corner_radius(5.0);
                let resp = ui.add(tagbutton)
                    .on_hover_text("Remove");
                if resp.clicked() {
                    torm.push(idx);
                }
            }

            for tagidx in torm {
                self.tags.remove(tagidx);
            }
        });

        ui.add(Separator::default());

        ui.horizontal(|ui| {
            ui.with_layout(Layout::left_to_right(egui::Align::Max), |ui| {
                let save = Button::new("Save");
                let enabled = name_valid
                    && pass_valid
                    && self.newkvp.is_none()
                    && self.newtag.is_none();
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

enum Current {
    Id(CurrentId),
    Service(CurrentService)
}

impl Current {
    fn refresh(&mut self, apctx: &ApCtx) {
        match self {
            Self::Id(i) => i.refresh(apctx),
            Self::Service(s) => s.refresh(apctx)
        }
    }

    fn is_service(&self, service: &str) -> bool {
        if let Self::Service(s) = self {
            s.entry.name() == service
        } else {
            false
        }
    }

    fn is_id(&self) -> bool {
        match self {
            Self::Id(_) => true,
            _ => false
        }
    }

    fn dirty_msg(&self) -> Option<String> {
        match self {
            Self::Service(s) => s.dirty_msg(),
            Self::Id(id) => id.dirty_msg()
        }
    }
}

impl Display<ApCtx, bool> for Current {
    fn display(&mut self, ctx: &egui::Context, ui: &mut Ui, apctx: &mut ApCtx) -> bool {
        match self {
            Current::Id(c) => c.display(ctx, ui, apctx),
            Current::Service(s) => s.display(ctx, ui, apctx)
        }
    }
}

struct MoveTo {
    target: Option<Current>
}

impl Action<ApCtx> for MoveTo {
    fn doit(&mut self, apctx: &mut ApCtx) {
        apctx.set_service = Some(self.target.take());
    }
}

struct ApApp {
    current: Option<Current>,
    newservice: Windowed<NewService>,
    confirm: Windowed<Box<dyn Display<ApCtx, bool>>>,
    ctx: ApCtx
}

impl ApApp {
    fn new(pwd: String) -> Self {
        let username = api::get_id(&pwd).unwrap_or_else(|e| {
            panic!("Unable to parse identity file: {}", e);
        }).name().to_owned();
        let services = ServiceList::new(&pwd).unwrap_or_else(|e| {
            panic!("Unable to list services: {}", e);
        });

        Self {
            current: None,
            newservice: Windowed::new(),
            confirm: Windowed::new(),
            ctx: ApCtx::new(username, pwd, services)
        }
    }

    fn set_current(&mut self, target: Option<Current>) {
        /* Reset the service to none if you reclick on the same service */
        if let Some(msg) = self.current
            .as_ref()
            .map(|c| c.dirty_msg())
            .flatten()
        {
            self.confirm.set(
                "Lose unsaved information".to_owned(),
                Box::new(ConfirmBox::new(msg, MoveTo { target }))
            )
        } else {
            self.current = target;
        }
    }
}

impl eframe::App for ApApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.confirm.display(ctx, &mut self.ctx);

        if self.ctx.refresh_service_list {
            self.ctx.services.refresh(&self.ctx.masterpwd).unwrap_or_else(|e| {
                panic!("Unable to list services: {}", e);
            });
            self.ctx.refresh_service_list = false;
        }

        if let Some(ns) = self.ctx.set_service.take() {
            self.current = ns;
        }

        if self.ctx.refresh_service {
            if let Some(s) = self.current.as_mut() {
                s.refresh(&self.ctx);
            }
            self.ctx.refresh_service = false;
        }

        self.newservice.display(ctx, &mut self.ctx);

        egui::SidePanel::left("services")
            .resizable(false)
            .max_width(120.0)
            .show(ctx, |ui| {
                egui::TopBottomPanel::bottom("Bottom Left").min_height(25.0).show_separator_line(false).show_inside(ui, |ui| {
                    ui.with_layout(Layout::bottom_up(egui::Align::Center), |ui| {
                        let addservice = Button::new("Add Service").wrap_mode(egui::TextWrapMode::Truncate);
                        if ui.add(addservice).clicked() {
                            self.newservice.set("New Service".to_owned(), NewService::new());
                        }
                    });
                });

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut selected = None;

                    let is_selected = self.current.as_ref().map(|c| c.is_id()).unwrap_or(false);
                    if ui.add(SelectableLabel::new(is_selected, RichText::new(&self.ctx.username).strong())).clicked() {
                        let target = if self.current.is_none() || !self.current.as_ref().unwrap().is_id() {
                            Some(Current::Id(CurrentId::new(&self.ctx)))
                        } else {
                            None
                        };
                        selected = Some(target);
                    }

                    // Tags for filtering results
                    let tags = self.ctx.services.tags_mut();
                    if tags.len() > 0 {
                        ui.add(Separator::default());

                        let mut selected = false;
                        ui.horizontal_wrapped(|ui| {
                            for (tag, enabled) in &mut *tags {
                                let color = if *enabled {
                                    selected = true;
                                    Color32::GRAY
                                } else {
                                    Color32::LIGHT_GRAY
                                };

                                let tagfilter = Button::new(tag.clone()).fill(color).corner_radius(5);

                                if ui.add(tagfilter).clicked() {
                                    *enabled = !*enabled;
                                }
                            }
                        });

                        /*
                        ui.vertical_centered(|ui| {
                            let toggle_text = if selected { "Unselect Tags" } else { "List All" };
                            if ui.add(Button::new(toggle_text)).clicked() {
                                for (_, enabled) in tags {
                                    *enabled = !selected;
                                }
                            }
                        });
                        */
                    }

                    ui.add(Separator::default());

                    for service in self.ctx.services.iter_visible_services() {
                        let is_selected = self.current.as_ref().map(|c| c.is_service(&service)).unwrap_or(false);
                        if ui.add(SelectableLabel::new(is_selected, service)).clicked() {
                            let target = if self.current.is_none() || !self.current.as_ref().unwrap().is_service(service) {
                                Some(Current::Service(CurrentService::new(service, &self.ctx)))
                            } else {
                                None
                            };
                            selected = Some(target);
                        }
                    }
                    if let Some(target) = selected {
                        self.set_current(target);
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