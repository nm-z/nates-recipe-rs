use crate::gguf::{Gguf, Val};
use anyhow::{Result, anyhow, bail};
use hf_chat_template::ChatTemplate;
use hf_chat_template::minijinja::{Value, context};
use std::path::Path;

pub struct Msg {
      pub role: String,
      pub content: String,
}

impl Msg {
      pub fn new(role: impl Into<String>, content: impl Into<String>) -> Msg {
            return Msg {
                  role: role.into(),
                  content: content.into(),
            };
      }
}

pub fn render_template(
      tmpl: &str,
      msgs: &[Msg],
      add_generation_prompt: bool,
      bos_token: &str,
      eos_token: &str,
) -> Result<String> {
      let messages: Vec<Value> = msgs
            .iter()
            .map(|m| context! { role => m.role.as_str(), content => m.content.as_str() })
            .collect();
      return ChatTemplate::from_str(tmpl)
            .map_err(|e| anyhow!("chat template compile: {e}"))?
            .render_value(context! {
                  messages => messages,
                  add_generation_prompt => add_generation_prompt,
                  bos_token => bos_token,
                  eos_token => eos_token,
            })
            .map_err(|e| anyhow!("chat template render: {e}"));
}

pub fn render_chat(gguf: &Path, msgs: &[Msg], add_generation_prompt: bool) -> Result<String> {
      let g = Gguf::open(gguf)?;
      let tmpl = match g.kv.get("tokenizer.chat_template") {
            Some(Val::Str(s)) => s.clone(),
            Some(_other) => bail!("gguf: kv tokenizer.chat_template is not a string"),
            None => bail!("gguf: kv tokenizer.chat_template not found"),
      };
      let tokens = g.str_arr("tokenizer.ggml.tokens")?;
      let bos_token = token_str(&g, &tokens, "tokenizer.ggml.bos_token_id");
      let eos_token = token_str(&g, &tokens, "tokenizer.ggml.eos_token_id");
      let rendered = render_template(&tmpl, msgs, add_generation_prompt, &bos_token, &eos_token)?;
      let out = match (!bos_token.is_empty(), rendered.strip_prefix(bos_token.as_str())) {
            (true, Some(rest)) => rest.to_string(),
            _other => rendered,
      };
      return Ok(out);
}

fn token_str(g: &Gguf, tokens: &[String], key: &str) -> String {
      let id = match g.kv.get(key) {
            Some(Val::U32(v)) => *v as usize,
            Some(Val::U64(v)) => *v as usize,
            Some(Val::I32(v)) if *v >= 0 => *v as usize,
            Some(Val::I64(v)) if *v >= 0 => *v as usize,
            _other => return String::new(),
      };
      return tokens.get(id).cloned().unwrap_or_default();
}
