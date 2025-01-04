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

impl<T: Validator<T>> Validator<T> for Box<dyn Validator<T>> {
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

pub struct ValidString {
    string: String,
    validator: Box<dyn Validator<String>>
}

impl ValidString {
    pub fn new(validator: Box<dyn Validator<String>>) -> Self {
        Self {
            string: String::new(),
            validator
        }
    }

    pub fn string_mut(&mut self) -> &mut String {
        &mut self.string
    }

    pub fn string(&self) -> &str {
        &self.string
    }

    pub fn to_owned(self) -> String {
        self.string
    }

    pub fn is_valid(&self) -> bool {
        self.validator.valid(&self.string).is_ok()
    }

    pub fn check(&self, additional: Option<&dyn Validator<String>>) -> Result<(), String> {
        let mut validators = vec![self.validator.as_ref()];
        if let Some(v) = additional {
            validators.push(v);
        }
        validators.as_slice().valid(&self.string)
    }
}

impl Default for ValidString {
    fn default() -> Self {
        Self { string: String::new(), validator: Box::new(()) }
    }
}

pub fn textedit(ui: &mut Ui, string: &mut ValidString, additional: Option<&dyn Validator<String>>, modify_textedit: impl FnOnce(TextEdit, bool) -> TextEdit) -> Response {
    ui.scope(|ui| {
        match string.check(additional) {
            Ok(()) => {
                let textedit = TextEdit::singleline(string.string_mut());
                let textedit = modify_textedit(textedit, true);
                ui.add(textedit)
            }
            Err(msg) => {
                ui.visuals_mut().extreme_bg_color = ERR_COLOR;
                let textedit = TextEdit::singleline(string.string_mut());
                let textedit = modify_textedit(textedit, false);
                ui.add(textedit).on_hover_text(msg)
            }
        }
    }).inner
}