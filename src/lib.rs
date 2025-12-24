pub mod cache;
pub mod loader;
pub mod parser;
use napi::{
  bindgen_prelude::{JsObjectValue, Object},
  Env, Error, Result,
};
use napi_derive::napi;

use std::fs;
use std::path::Path;
#[derive(Debug)]
pub enum Severity {
  Fatal,
  Error,
  Warning,
  Info,
}

#[derive(Debug)]
pub struct LangError {
  pub code: String,
  pub severity: Severity,
  pub lang: String,
  pub engine: Option<String>,
  pub key: Option<String>,
  pub message: String,
}

#[napi(object)]
pub struct JsLangError {
  pub code: String,
  pub severity: String,
  pub lang: String,
  pub engine: Option<String>,
  pub key: Option<String>,
  pub message: String,
}
use crate::{
  loader::{load_lang_dir, LangCache},
  parser::push_error,
};

#[napi]
pub fn load_langs<'a>(env: &'a Env, dir: String) -> Result<Object<'a>> {
  let langs = load_lang_dir(Path::new(&dir))?;
  to_js(env, &langs)
}

#[napi]
pub fn load_cached_langs<'a>(env: &'a Env, dir: String) -> Result<Object<'a>> {
  if let Some(cached) = cache::get() {
    return to_js(env, &cached);
  }

  let langs = load_lang_dir(Path::new(&dir))?;
  cache::set(langs);
  to_js(env, &cache::get().unwrap())
}

// should return cached language and custom languages
#[napi]
pub fn get_languages<'a>(env: &'a Env) -> Result<Object<'a>> {
  if let Some(cached) = cache::get() {
    return to_js(env, &cached);
  }
  Err(Error::from_reason("No cached languages found"))
}

#[napi]
pub fn get_language<'a>(env: &'a Env, language: String) -> Result<Option<Object<'a>>> {
  let cached = cache::get().ok_or_else(|| Error::from_reason("No cached languages found"))?;

  let lang_map = match cached.get(&language) {
    Some(l) => l,
    None => return Ok(None),
  };

  let mut single = LangCache::with_capacity(1);
  single.insert(language, lang_map.clone());

  let obj = to_js(env, &single)?;
  Ok(Some(obj))
}

#[napi]
pub fn load_custom_language<'a>(dir: String, custom_dir: String) -> Result<Vec<JsLangError>> {
  let base_langs = load_lang_dir(Path::new(&dir))?;
  let custom_langs = load_lang_dir(Path::new(&custom_dir))?;

  let mut errors: Vec<LangError> = Vec::new();
  let mut merged = base_langs.clone();

  for (lang, kv_map) in custom_langs {
    if merged.contains_key(&lang) {
      continue;
    }

    let engine = kv_map.get("meta.engine");
    let mut is_errored = false;

    if let Some(engine_name) = engine {
      let base_lang = match merged.get(engine_name.as_str()) {
        Some(v) => v,
        None => {
          push_error(
            &mut errors,
            "ENGINE_NOT_FOUND",
            Severity::Fatal,
            &lang,
            Some(engine_name),
            None,
            format!("Engine '{}' not found for language '{}'", engine_name, lang),
          );
          continue;
        }
      };

      for base_key in base_lang.keys() {
        if !kv_map.contains_key(base_key) {
          push_error(
            &mut errors,
            "MISSING_KEY",
            Severity::Error,
            &lang,
            Some(engine_name),
            Some(base_key),
            format!("Missing key '{}' in '{}'", base_key, lang),
          );
          is_errored = true;
        }
      }

      for custom_key in kv_map.keys().filter(|k| !k.starts_with("meta.")) {
        if !base_lang.contains_key(custom_key) {
          push_error(
            &mut errors,
            "EXTRA_KEY",
            Severity::Warning,
            &lang,
            Some(engine_name),
            Some(custom_key),
            format!("Extra key '{}' in '{}'", custom_key, lang),
          );
        }
      }
    }

    if !is_errored {
      merged.insert(lang, kv_map);
    }
  }

  if errors.iter().any(|e| matches!(e.severity, Severity::Fatal)) {
    return Err(Error::from_reason(
      "Fatal errors occurred while loading custom languages",
    ));
  }

  cache::clear();
  cache::set(merged);
  Ok(
    errors
      .into_iter()
      .map(|e| JsLangError {
        code: e.code,
        severity: match e.severity {
          Severity::Fatal => "Fatal".to_string(),
          Severity::Error => "Error".to_string(),
          Severity::Warning => "Warning".to_string(),
          Severity::Info => "Info".to_string(),
        },
        lang: e.lang,
        engine: e.engine,
        key: e.key,
        message: e.message,
      })
      .collect(),
  )
}

#[napi]
pub fn clear_language(language: String) -> Result<bool> {
  if let Some(mut cached) = cache::get() {
    let removed = cached.remove(&language).is_some();
    cache::set(cached);
    return Ok(removed);
  }
  Ok(false)
}

fn to_js<'a>(env: &'a Env, langs: &LangCache) -> Result<Object<'a>> {
  let mut root = Object::new(env)?;
  for (lang, kv_map) in langs {
    let mut obj = Object::new(env)?;
    let mut meta_obj = Object::new(env)?;
    let mut has_meta = false;

    for (k, v) in kv_map {
      if k.starts_with("meta.") {
        let meta_key = &k[5..];
        let trimmed = v.trim();
        if trimmed.starts_with('"') && trimmed.ends_with('"') {
          let val = trimmed
            .trim_matches('"')
            .replace("\\n", "\n")
            .replace("\\\"", "\"");
          meta_obj.set_named_property(meta_key, env.create_string(&val)?)?;
        } else {
          meta_obj.set_named_property(meta_key, env.create_string(v)?)?;
        }
        has_meta = true;
        continue;
      }

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

    if has_meta {
      obj.set_named_property("meta", meta_obj)?;
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
  let langs = load_lang_dir(Path::new(&dir))?;
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
