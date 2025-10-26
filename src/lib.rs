use napi::{
  bindgen_prelude::{JsObjectValue, Object},
  Env, Error, Result,
};
use napi_derive::napi;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use std::{collections::HashMap, fs, path::Path, sync::RwLock};

type LangCache = HashMap<String, HashMap<String, String>>;
static LANG_CACHE: Lazy<RwLock<Option<LangCache>>> = Lazy::new(|| RwLock::new(None));

#[napi]
pub fn load_lang(env: &Env, path: String) -> Result<Object<'_>> {
  let path = Path::new(&path);
  if !path.is_file() {
    return Err(Error::from_reason(format!(
      "Path '{}' is not a file",
      path.display()
    )));
  }

  let data = fs::read_to_string(path)
    .map_err(|e| Error::from_reason(format!("Cannot read file '{}': {}", path.display(), e)))?;
  let map = parse_lang_data(&data);
  let mut js = Object::new(&env)?;
  for (k, v) in map {
    js.set_named_property(&k, env.create_string(&v)?)?;
  }
  Ok(js)
}

fn parse_lang_data(data: &str) -> HashMap<String, String> {
  let mut map = HashMap::new();
  let mut ckey: Option<String> = None;
  let mut cval = String::new();
  let mut mtl = false;

  for line in data.lines() {
    let line = line.trim_end();
    if line.is_empty() || line.starts_with('#') {
      continue;
    }

    if mtl {
      cval.push_str(line);
      cval.push('\n');
      if line.trim().ends_with('"') && !line.trim().ends_with("\\\"") {
        mtl = false;
        if cval.starts_with('"') {
          cval = cval[1..cval.len() - 1].trim_end().to_string();
        }
        if cval.ends_with('"') {
          cval.pop();
        }
        if let Some(key) = ckey.take() {
          map.insert(key, cval.trim_end().to_string());
        }
        cval.clear();
      }
      continue;
    }
    if let Some(pos) = line.find('=') {
      let key = line[..pos].trim().to_string();
      let value = line[pos + 1..].trim();

      if key.is_empty() {
        continue;
      }
      if value.is_empty() {
        mtl = true;
        ckey = Some(key);
        cval.clear();
        continue;
      }

      if value.starts_with('"') {
        if !value.ends_with('"') || value.ends_with("\\\"") {
          mtl = true;
          ckey = Some(key);
          cval = value.to_string();
          cval.push('\n');
        } else {
          let value = value.trim_matches('"').to_string();
          map.insert(key, value);
        }
      } else {
        map.insert(key, value.to_string());
      }
    } else if ckey.is_some() {
      if line.starts_with('"') {
        if !line.ends_with('"') || line.ends_with("\\\"") {
          mtl = true;
          cval = line.to_string();
          cval.push('\n');
        } else {
          if let Some(key) = ckey.take() {
            let value = line.trim_matches('"').to_string();
            map.insert(key, value);
          }
        }
      }
    }
  }

  if mtl {
    if let Some(key) = ckey {
      let value = if cval.starts_with('"') {
        cval[1..].trim_end().to_string()
      } else {
        cval.trim_end().to_string()
      };
      map.insert(key, value);
    }
  }

  map
}
pub fn validate_path_is_dir(dir: &str) -> Result<&Path> {
  let dirpath = Path::new(dir);
  if !dirpath.is_dir() {
    return Err(Error::from_reason(format!(
      "Path '{}' is not a directory",
      dir
    )));
  }
  Ok(dirpath)
}

pub fn validate_files(files: &Path) -> Result<Vec<std::path::PathBuf>> {
  let fl: Vec<_> = fs::read_dir(files)
    .map_err(|e| Error::from_reason(e.to_string()))?
    .filter_map(|entry| {
      let entry = entry.ok()?;
      let path = entry.path();
      if path.extension()? == "lang" {
        Some(path)
      } else {
        None
      }
    })
    .collect();
  Ok(fl)
}

fn load_lang_dsk(dir: &str) -> Result<LangCache> {
  let dirpath = validate_path_is_dir(dir)?;
  let files = validate_files(dirpath)?;
  let results: LangCache = files
    .par_iter()
    .filter_map(|path| {
      let name = path.file_stem()?.to_string_lossy().to_string();
      let data = fs::read_to_string(path).ok()?;
      Some((name, parse_lang_data(&data)))
    })
    .collect();
  Ok(results)
}

#[napi]
pub fn load_langs(env: &Env, dir: String) -> Result<Object<'_>> {
  let results = load_lang_dsk(&dir)?;
  to_js(env, &results)
}

#[napi]
pub fn load_chdlang(env: &Env, dir: String) -> Result<Object<'_>> {
  {
    let cache = LANG_CACHE.read().unwrap();
    if let Some(cached) = &*cache {
      return to_js(env, cached);
    }
  }

  let langs = load_lang_dsk(&dir)?;
  {
    let mut cache = LANG_CACHE.write().unwrap();
    *cache = Some(langs.clone());
  }

  to_js(env, &langs)
}

fn to_js<'a>(env: &'a Env, langs: &LangCache) -> Result<Object<'a>> {
  let mut root = Object::new(env)?;
  for (lang, kv_map) in langs {
    let mut obj = Object::new(env)?;
    for (k, v) in kv_map {
      obj.set_named_property(k, env.create_string(v)?)?;
    }
    root.set_named_property(lang, obj)?;
  }
  Ok(root)
}

#[napi]
pub fn generate_typescript_defs(dir: String, output: String) -> Result<()> {
  let langs = load_lang_dsk(&dir)?;
  let mut defs = String::new();
  defs.push_str("// THIS FILE WAS GENERATED BY SSL\n");
  defs.push_str("// DO NOT EDIT MANUALLY OR ELSE IT WILL BE OVERWRITTEN\n\n");
  defs.push_str("/* eslint-disable */\n\n");
  defs.push_str("export interface Lang {\n");
  if let Some(first_lang) = langs.values().next() {
    for key in first_lang.keys() {
      defs.push_str(&format!("    '{}': string;\n", key));
    }
  }
  defs.push_str("}\n\n");
  defs.push_str("export interface Langs {\n");
  for lang in langs.keys() {
    defs.push_str(&format!("    '{}': Lang;\n", lang));
  }
  defs.push_str("}\n\n");
  defs.push_str("export const langs: Langs;\n");

  fs::write(output, defs)
    .map_err(|e| Error::from_reason(format!("Failed to write TypeScript definitions: {}", e)))?;
  Ok(())
}

#[napi]
pub fn clear_lang_cache() {
  let mut cache = LANG_CACHE.write().unwrap();
  *cache = None;
}
