use egui::Ui;

pub mod msgbox;
pub mod confirmbox;

pub trait Display<T> {
    fn display(&mut self, ctx: &egui::Context, ui: &mut Ui) -> T;
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
}

impl<T: Display<bool>> Display<bool> for Windowed<T> {
    fn display(&mut self, ctx: &egui::Context, _ui: &mut Ui) -> bool {
        let mut clear = false;
        if let Some(d) = &mut self.inner {
            egui::Window::new(&self.title)
                .resizable(false)
                .auto_sized()
                .collapsible(false)
                .show(ctx, |ui| 
                {
                    clear = !d.display(ctx, ui);
                });
        }
        if clear {
            self.inner = None;
        }
        !clear
    }
}