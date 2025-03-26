use std::collections::{HashMap, HashSet};

use crate::{api::{self, APError}, bitmap::Bitmap, spec::Serializable};

use super::validator::Validator;


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

    pub fn tags_mut(&mut self) -> &mut [(String, bool)] {
        &mut self.tags
    }

    fn service_visible(&self, bmp: &Bitmap) -> bool {
        self.tags.iter().enumerate().fold(false, |show, (idx, (_, tagset))| {
            show || (*tagset && bmp.check_set(idx))
        })
    }

    fn iter_tags<'a, 'b>(&'a self, bmp: &'b Bitmap) -> impl Iterator<Item=&'a String> + use<'a, 'b> {
        self.tags.iter()
            .enumerate()
            .filter_map(move |(idx, (tag, _))| bmp.check_set(idx).then(|| tag) )
    }

    pub fn iter_visible_services(&self) -> impl Iterator<Item=&String> {
        let none_set = !self.tags.iter()
            .fold(false, |none_set, (_, set)| none_set || *set);

        self.services.iter()
            .filter(move |(_, bmp)| none_set || self.service_visible(bmp))
            .map(|(s, _)| s)
    }

    pub fn not_in_services<'a>(&'a self) -> NotAService<'a> {
        NotAService { services: self }
    }

    pub fn not_in_tags<'a>(&'a self, service: &'a str) -> NotATag<'a> {
        let tags: Vec<&String> = self.services.iter()
            .filter_map(|(s, bmp)| (s == service).then(|| self.iter_tags(bmp)))
            .next()
            .unwrap()
            .collect::<Vec<&String>>();
        NotATag{ tags }
    }

}

pub struct NotAService<'a> {
    services: &'a ServiceList
}

impl<'a> Validator<String> for NotAService<'a> {
    fn valid(&self, obj: &String) -> Result<(), String> {
        for (item, _) in &self.services.services {
            if item == obj {
                return Err("Service already exists".to_owned());
            }
        }
        Ok(())
    }
}

pub struct NotATag<'a> {
    tags: Vec<&'a String>
}

impl<'a> Validator<String> for NotATag<'a> {
    fn valid(&self, obj: &String) -> Result<(), String> {
        for tag in &self.tags {
            if *tag == obj {
                return Err("Tag already exists".to_owned())
            }
        }
        Ok(())
    }
}