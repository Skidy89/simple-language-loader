use crate::loader::LangCache;
use once_cell::sync::OnceCell;

static LANG_CACHE: OnceCell<LangCache> = OnceCell::new();

pub fn get() -> Option<&'static LangCache> {
  LANG_CACHE.get()
}

pub fn set(langs: LangCache) {
  let _ = LANG_CACHE.set(langs);
}
