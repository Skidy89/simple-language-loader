use crate::parser::{parse_lang_data, LangMap};
use rayon::prelude::*;
use std::{collections::HashMap, fs, path::Path};

pub type LangCache = HashMap<String, LangMap>;

pub fn load_lang_dir(dir: &Path) -> napi::Result<LangCache> {
  if !dir.is_dir() {
    return Err(napi::Error::from_reason("Not a directory"));
  }

  let files: Vec<_> = fs::read_dir(dir)?
    .filter_map(|e| {
      let p = e.ok()?.path();
      (p.extension()? == "lang").then_some(p)
    })
    .collect();

  Ok(
    files
      .par_iter()
      .filter_map(|p| {
        let name = p.file_stem()?.to_string_lossy().to_string();
        let data = fs::read_to_string(p).ok()?;
        Some((name, parse_lang_data(&data)))
      })
      .collect(),
  )
}
