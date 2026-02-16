use serde::{Deserialize, Serialize};

use super::{identity_v2::IdentityV2, service_v2::ServiceEntryV2, Serializable, CHANGELOG_MAGIC};


#[derive(Deserialize, Serialize, Debug)]
enum ObjType {
    ServiceV2(ServiceEntryV2),
    IdentityV2(IdentityV2),
    Removed
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ChangeLog {
    magic: u32,
    id: u32,
    previd: u32,
    objs: Vec<ObjType>
}

impl ChangeLog {
    const NONE_MARKER: u32 = u32::MAX;

    pub fn init() -> Self {
        Self {
            magic: CHANGELOG_MAGIC,
            id: 0,
            previd: Self::NONE_MARKER,
            objs: vec![]
        }
    }

    pub fn new(previd: u32) -> Self {
        Self {
            magic: CHANGELOG_MAGIC,
            id: previd + 1,
            previd,
            objs: vec![]
        }
    }

    pub fn save_id(&mut self, id: IdentityV2) {
        self.objs.push(ObjType::IdentityV2(id));
    }

    pub fn save_service(&mut self, service: ServiceEntryV2) {
        self.objs.push(ObjType::ServiceV2(service));
    }

    pub fn get_prev(&self) -> Option<u32> {
        if self.previd == Self::NONE_MARKER {
            None
        } else {
            Some(self.previd)
        }
    }

    fn find_prev<T, F>(&self, before_idx: Option<usize>, filter: F) -> Option<(usize, &T)>
    where F: Fn(&ObjType) -> Option<&T>
    {
        let mut idx = before_idx.unwrap_or(self.objs.len());
        if idx > self.objs.len() {
            idx = self.objs.len();
        }
        while idx >= 1 {
            idx -= 1;

            if let Some(val) = filter(&self.objs[idx]) {
                return Some((idx, val))
            }
        }
        None
    }

    pub fn find_prev_id(&self, before_idx: Option<usize>) -> Option<(usize, &IdentityV2)> {
        self.find_prev(before_idx, |obj| {
            match obj {
                ObjType::IdentityV2(id) => Some(id),
                _ => None
            }
        })
    }

    pub fn find_prev_service(&self, before_idx: Option<usize>, service: &str) -> Option<(usize, &ServiceEntryV2)> {
        self.find_prev(before_idx, |obj| {
            match obj {
                ObjType::ServiceV2(sv) => {
                    if sv.get_name() == service {
                        Some(sv)
                    } else {
                        None
                    }
                },
                _ => None
            }
        })
    }

    pub fn remove(&mut self, idx: usize) {
        self.objs[idx] = ObjType::Removed;
    }

    pub fn spec_type() -> super::SpecType {
        super::SpecType::ChangeLog
    }

    pub fn version() -> u16 {
        1
    }

}

impl<'de> Serializable<'de> for ChangeLog {
    fn name(&self) -> &str {
        "CHANGELOG"
    }

    fn to_binary(&self) -> Vec<u8> {
        todo!()
    }

    fn from_binary(bin: &[u8]) -> Option<Self> {
        todo!()
    }

    fn sanity_check(&self) -> bool {
        todo!()
    }

    fn version(&self) -> u16 {
        todo!()
    }

    fn spec_type(&self) -> super::SpecType {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_changelog() {
        let mut cl = ChangeLog::init();

        assert!(cl.find_prev_service(None, "sv1").is_none());
        assert!(cl.find_prev_service(Some(1), "sv1").is_none());
        let sv1 = ServiceEntryV2::new::<&str>("sv1", "testpass", 1, &[], &[], 8, &crate::hash::TextMode::NoWhiteSpace);
        let sv2 = ServiceEntryV2::new::<&str>("sv2", "testpass", 1, &[], &[], 8, &crate::hash::TextMode::NoWhiteSpace);
        cl.save_service(sv1);
        assert!(matches!(cl.find_prev_service(None, "sv1"), Some((0, _))));
        cl.save_service(sv2);
        assert!(matches!(cl.find_prev_service(None, "sv1"), Some((0, _))));
        assert!(matches!(cl.find_prev_service(Some(1), "sv1"), Some((0, _))));
        assert!(matches!(cl.find_prev_service(None, "sv2"), Some((1, _))));
    }
}