use egui::{Color32, Response, TextEdit, Ui};

const ERR_COLOR: Color32 = Color32::LIGHT_RED;

pub trait Validator<T> {
    fn valid(&self, obj: &T) -> Result<(), String>;
}

pub struct NotEmpty;

impl Validator<String> for NotEmpty {
    fn valid(&self, obj: &String) -> Result<(), String> {
        if obj.is_empty() {
            return Err("Entry cannot be empty".to_owned());
        }
        Ok(())
    }
}


pub fn textedit(ui: &mut Ui, text: &mut String, validator: &dyn Validator<String>, modify_textedit: impl FnOnce(TextEdit, bool) -> TextEdit) -> Response {
    ui.scope(|ui| {
        match validator.valid(text) {
            Ok(()) => {
                let textedit = TextEdit::singleline(text);
                let textedit = modify_textedit(textedit, true);
                ui.add(textedit)
            }
            Err(msg) => {
                ui.visuals_mut().extreme_bg_color = ERR_COLOR;
                let textedit = TextEdit::singleline(text);
                let textedit = modify_textedit(textedit, false);
                ui.add(textedit).on_hover_text(msg)
            }
        }
    }).inner
}