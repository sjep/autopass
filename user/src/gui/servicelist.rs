use std::collections::{HashMap, HashSet};

use crate::{api::{self, APError}, bitmap::Bitmap, spec::Serializable};


pub struct ServiceList {
    tags: Vec<(String, bool)>,
    services: Vec<(String, Bitmap)>
}

impl ServiceList {
    pub fn refresh(&mut self, pass: &str) -> Result<(), APError> {
        let rawservices = api::list_all(pass, &[])?;
        let mut tagset = HashSet::new();

        for service in &rawservices {
            for tag in service.get_tags() {
                tagset.insert(tag);
            }
        }
        self.tags = tagset.drain().map(|t| (t.to_owned(), false)).collect();
        self.tags.sort();
        let mut taglookup = HashMap::new();
        for tag in self.tags.iter().enumerate() {
            taglookup.insert(&tag.1.0, tag.0);
        }

        self.services.clear();
        for service in &rawservices {
            let mut bmp = Bitmap::new(self.tags.len());
            for tag in service.get_tags() {
                let idx = taglookup.get(tag).unwrap();
                bmp.set(*idx);
            }
            self.services.push((service.name().to_owned(), bmp));
        }
        self.services.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(())
    }

    pub fn new(pass: &str) -> Result<Self, APError> {
        let mut inst = Self {
            tags: vec![],
            services: vec![]
        };
        inst.refresh(pass)?;
        Ok(inst)
    }
}