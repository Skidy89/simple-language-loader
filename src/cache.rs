use once_cell::sync::Lazy;
use std::{collections::HashMap, sync::RwLock};

use crate::parser::LangMap;

pub type LangCache = HashMap<String, LangMap>;

pub static LANG_CACHE: Lazy<RwLock<Option<LangCache>>> = Lazy::new(|| RwLock::new(None));

pub fn get() -> Option<LangCache> {
  LANG_CACHE.read().unwrap().as_ref().cloned()
}

pub fn set(data: LangCache) {
  *LANG_CACHE.write().unwrap() = Some(data);
}

pub fn clear() {
  *LANG_CACHE.write().unwrap() = None;
}
