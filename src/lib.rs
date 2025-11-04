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
  let mut is_array = false;

  for line in data.lines() {
    let line = line.trim_end();
    if line.is_empty() || line.starts_with('#') {
      continue;
    }

    if mtl {
      cval.push_str(line);
      cval.push('\n');
      if (is_array && line.trim().ends_with(']'))
        || (!is_array && line.trim().ends_with('"') && !line.trim().ends_with("\\\""))
      {
        mtl = false;
        if let Some(key) = ckey.take() {
          map.insert(key, cval.trim().to_string());
        }
        cval.clear();
        is_array = false;
      }
      continue;
    }

    if let Some(pos) = line.find('=') {
      let key = line[..pos].trim().to_string();
      let value = line[pos + 1..].trim();

      if key.is_empty() {
        continue;
      }

      if value.starts_with('[') {
        if !value.ends_with(']') {
          mtl = true;
          is_array = true;
          ckey = Some(key);
          cval = value.to_string();
          cval.push('\n');
        } else {
          map.insert(key, value.to_string());
        }
      } else if value.starts_with('"') {
        if !value.ends_with('"') || value.ends_with("\\\"") {
          mtl = true;
          ckey = Some(key);
          cval = value.to_string();
          cval.push('\n');
        } else {
          map.insert(key, value.trim_matches('"').to_string());
        }
      } else {
        map.insert(key, value.to_string());
      }
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
      let trimmed = v.trim();
      if trimmed.starts_with('[') && trimmed.ends_with(']') {
        let inr = &trimmed[1..trimmed.len() - 1];
        let mut elm = Vec::new();
        for line in inr.lines() {
          let line = line.trim().trim_end_matches(',');
          if line.is_empty() {
            continue;
          }
          if line.starts_with('"') && line.ends_with('"') {
            elm.push(line.trim_matches('"').to_string())
          }
        }

        let mut arr = env.create_array(elm.len() as u32)?;
        for (i, val) in elm.iter().enumerate() {
          arr.set_element(i as u32, env.create_string(val)?)?
        }

        obj.set_named_property(k, arr)?;
        continue;
      }
      if trimmed.starts_with('"') && trimmed.ends_with('"') {
        let val = trimmed
          .trim_matches('"')
          .replace("\\n", "\n")
          .replace("\\\"", "\"");
        obj.set_named_property(k, env.create_string(&val)?)?;
        continue;
      }
      obj.set_named_property(k, env.create_string(v)?)?;
    }
    root.set_named_property(lang, obj)?;
  }
  Ok(root)
}
#[napi]
pub fn generate_typescript_defs(
  dir: String,
  output: String,
  gen_placeholder: Option<bool>,
) -> Result<()> {
  let langs = load_lang_dsk(&dir)?;
  let mut defs = String::new();
  defs.push_str("// THIS FILE WAS GENERATED BY SSL\n");
  defs.push_str("// DO NOT EDIT MANUALLY OR ELSE IT WILL BE OVERWRITTEN\n\n");
  defs.push_str("/* eslint-disable */\n");
  defs.push_str("export interface Lang {\n");
  let rgx = regex::Regex::new(r"\{([a-zA-Z0-9_]+)\}").unwrap();
  let should_gen_placeholder = gen_placeholder.unwrap_or(false);
  if let Some(first_lang) = langs.values().next() {
    for key in first_lang.keys() {
      if let Some(value) = first_lang.get(key) {
        let trimmed = value.trim();
        let cm: Vec<&str> = value.lines().collect();
        if cm.len() == 1 {
          defs.push_str(&format!("    /** {} */\n", cm[0]));
        } else {
          defs.push_str("    /**\n");
          for line in cm {
            defs.push_str(&format!("     * {}\n", line));
          }
          defs.push_str("     */\n");
        }
        let mut placeholders: Vec<String> = Vec::new();
        for cap in rgx.captures_iter(value) {
          if let Some(p) = cap.get(1) {
            if !placeholders.contains(&p.as_str().to_string()) {
              placeholders.push(p.as_str().to_string());
            }
          }
        }
        if !placeholders.is_empty() && should_gen_placeholder && !trimmed.starts_with('[') {
          defs.push_str(&format!("    '{}': (args: {{ ", key));
          for (i, p) in placeholders.iter().enumerate() {
            if i > 0 {
              defs.push_str(", ");
            }
            defs.push_str(&format!("{}: string", p));
          }
          defs.push_str(" }) => string;\n");
        } else if trimmed.starts_with('[') && trimmed.ends_with(']') {
          defs.push_str(&format!("    '{}': string[];\n", key));
        } else {
          defs.push_str(&format!("    '{}': string;\n", key));
        }
      }
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
  if cfg!(debug_assertions) {
    println!("TypeScript definitions generated successfully.");
  }
  Ok(())
}

#[napi]
pub fn clear_lang_cache() {
  let mut cache = LANG_CACHE.write().unwrap();
  *cache = None;
}
