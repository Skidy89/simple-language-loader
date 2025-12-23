use std::collections::HashMap;

pub type LangMap = HashMap<String, String>;

pub fn parse_lang_data(data: &str) -> LangMap {
  let mut map = HashMap::new();
  let mut key: Option<String> = None;
  let mut val = String::new();
  let mut multiline = false;
  let mut is_array = false;

  for raw in data.lines() {
    let line = raw.trim_end();
    if line.is_empty() || line.starts_with('#') {
      continue;
    }

    if multiline {
      val.push_str(line);
      val.push('\n');

      let end = if is_array {
        line.trim().ends_with(']')
      } else {
        line.trim().ends_with('"') && !line.trim().ends_with("\\\"")
      };

      if end {
        if let Some(k) = key.take() {
          map.insert(k, normalize_value(&val));
        }
        val.clear();
        multiline = false;
        is_array = false;
      }
      continue;
    }

    let Some(eq) = line.find('=') else { continue };
    let k = line[..eq].trim();
    let v = line[eq + 1..].trim();

    if k.is_empty() {
      continue;
    }

    if v.starts_with('[') {
      if v.ends_with(']') {
        map.insert(k.to_string(), v.to_string());
      } else {
        key = Some(k.to_string());
        val = v.to_string();
        val.push('\n');
        multiline = true;
        is_array = true;
      }
    } else if v.starts_with('"') {
      if v.ends_with('"') && !v.ends_with("\\\"") {
        map.insert(k.to_string(), normalize_value(v));
      } else {
        key = Some(k.to_string());
        val = v.to_string();
        val.push('\n');
        multiline = true;
      }
    } else {
      map.insert(k.to_string(), v.to_string());
    }
  }

  map
}

fn normalize_value(v: &str) -> String {
  let t = v.trim();
  if t.starts_with('"') && t.ends_with('"') {
    t.trim_matches('"')
      .replace("\\n", "\n")
      .replace("\\\"", "\"")
  } else {
    t.to_string()
  }
}
