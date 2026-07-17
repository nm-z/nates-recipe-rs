//! Chat-template rendering: turn a conversation history into the single prompt
//! string a model expects, using the Jinja `tokenizer.chat_template` baked into
//! the gguf metadata (the standard llama.cpp key). `render_template` is the pure
//! renderer over an explicit template string; `render_chat` is the gguf wrapper
//! that reads the template plus the bos/eos token strings for a model on disk.

use crate::gguf::{Gguf, Val};
use anyhow::{Result, anyhow, bail};
use minijinja::{Environment, Value, context};
use std::path::Path;

/// One conversation turn. `role` is the HF convention (`system`/`user`/`assistant`);
/// `content` is the raw text for that turn.
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

/// Render `msgs` through a Jinja chat template string. Provides `messages`,
/// `add_generation_prompt`, `bos_token`, `eos_token` and a `raise_exception`
/// callable (many HF templates call it to reject malformed histories).
pub fn render_template(
      tmpl: &str,
      msgs: &[Msg],
      add_generation_prompt: bool,
      bos_token: &str,
      eos_token: &str,
) -> Result<String> {
      let mut env = Environment::new();
      env.add_function(
            "raise_exception",
            |msg: String| -> std::result::Result<Value, minijinja::Error> {
                  return Err(minijinja::Error::new(
                        minijinja::ErrorKind::InvalidOperation,
                        msg,
                  ));
            },
      );
      env.add_template("chat", tmpl)
            .map_err(|e| anyhow!("chat template parse: {e}"))?;
      let tpl = env
            .get_template("chat")
            .map_err(|e| anyhow!("chat template get: {e}"))?;
      let messages: Vec<Value> = msgs
            .iter()
            .map(|m| context! { role => m.role.as_str(), content => m.content.as_str() })
            .collect();
      let rendered = tpl
            .render(context! {
                  messages => messages,
                  add_generation_prompt => add_generation_prompt,
                  bos_token => bos_token,
                  eos_token => eos_token,
            })
            .map_err(|e| anyhow!("chat template render: {e}"))?;
      return Ok(rendered);
}

/// Render the conversation for a gguf model on disk. Reads `tokenizer.chat_template`;
/// per the no-fallback-defaults invariant, a missing key is a clean Err, never a
/// synthesized default. A leading bos-token string is stripped from the result
/// because `generate()` prepends the bos id itself before tokenizing (deterministic
/// single-bos: strip the text here, let generate add exactly one back).
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
