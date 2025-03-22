use egui::{Color32, Response, TextEdit, Ui};

const ERR_COLOR: Color32 = Color32::LIGHT_RED;

pub trait Validator<T> {
    fn valid(&self, obj: &T) -> Result<(), String>;
}

impl<T> Validator<T> for &[&dyn Validator<T>] {
    fn valid(&self, obj: &T) -> Result<(), String> {
        let mut errs = vec![];
        for validator in self.iter() {
            if let Err(e) = validator.valid(obj) {
                errs.push(e);
            }
        }
        if errs.len() == 0 {
            Ok(())
        } else {
            Err(errs.join(" and "))
        }
    }
}

impl<T> Validator<T> for &Box<dyn Validator<T>>
{
    fn valid(&self, obj: &T) -> Result<(), String> {
        self.as_ref().valid(obj)
    }
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

impl Validator<String> for () {
    fn valid(&self, _obj: &String) -> Result<(), String> {
        Ok(())
    }
}

pub struct LengthBounds {
    least: usize,
    most: usize
}

impl LengthBounds {
    pub fn new(least: usize, most: usize) -> Self {
        Self {
            least,
            most
        }
    }
}

impl Validator<String> for LengthBounds {
    fn valid(&self, obj: &String) -> Result<(), String> {
        if obj.len() < self.least {
            return Err(format!("Must be at least {} characters", self.least));
        } else if obj.len() > self.most {
            return Err(format!("Must be at most {} characters", self.most));
        }
        Ok(())
    }
}

pub struct NotInList<'a, T> {
    list: &'a [T]
}

impl<'a, T> NotInList<'a, T> {
    pub fn new(list: &'a [T]) -> Self {
        Self { list }
    }
}

impl<'a, T: PartialEq> Validator<T> for NotInList<'a, T> {
    fn valid(&self, obj: &T) -> Result<(), String> {
        for item in self.list {
            if item == obj {
                return Err("Entry already exists".to_owned());
            }
        }
        Ok(())
    }
}

pub fn textedit2<V: Validator<String>>(ui: &mut Ui, string: &mut String, validation: V, modify_textedit: impl FnOnce(TextEdit, bool) -> TextEdit) -> (Response, bool) {
    let resp = ui.scope(|ui| {
        match validation.valid(string) {
            Ok(()) => {
                let textedit = TextEdit::singleline(string);
                let textedit = modify_textedit(textedit, true);
                (ui.add(textedit), true)
            }
            Err(msg) => {
                ui.visuals_mut().extreme_bg_color = ERR_COLOR;
                let textedit = TextEdit::singleline(string);
                let textedit = modify_textedit(textedit, false);
                (ui.add(textedit).on_hover_text(msg), false)
            }
        }
    });
    resp.inner
}