use egui::Ui;

pub mod msgbox;
pub mod confirmbox;
pub mod pwdprompt;
pub mod validator;

pub trait Display<C, T> {
    fn display(&mut self, ctx: &egui::Context, ui: &mut Ui, apctx: &mut C) -> T;
}

pub struct Windowed<T> {
    title: String,
    inner: Option<T>
}

impl<T> Windowed<T> {
    pub fn new() -> Self {
        Self {
            title: String::new(),
            inner: None
        }
    }

    pub fn set(&mut self, title: String, val: T) {
        self.title = title;
        self.inner.replace(val);
    }

    pub fn display<C>(&mut self, ctx: &egui::Context, apctx: &mut C) -> bool
    where T: Display<C, bool> {
        let mut clear = false;
        if let Some(d) = &mut self.inner {
            egui::Window::new(&self.title)
                .resizable(false)
                .auto_sized()
                .collapsible(false)
                .show(ctx, |ui| 
                {
                    clear = !d.display(ctx, ui, apctx);
                });
        }
        if clear {
            self.inner = None;
        }
        !clear
    }
}