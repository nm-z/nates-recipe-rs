use recipe_infer::chat::{Msg, render_template};

const QWEN_TOKENIZER_CONFIG: &str =
      include_str!("../examples/datasets/tokenizers/qwen2.5-0.5b/tokenizer_config.json");

fn qwen_template() -> String {
      let cfg: serde_json::Value =
            serde_json::from_str(QWEN_TOKENIZER_CONFIG).expect("parse tokenizer_config.json");
      return cfg["chat_template"]
            .as_str()
            .expect("chat_template is a string")
            .to_string();
}

#[test]
fn qwen25_multiturn_renders_im_markers() {
      let tmpl = qwen_template();
      let msgs = vec![
            Msg::new("user", "Hello"),
            Msg::new("assistant", "Hi there."),
            Msg::new("user", "How are you?"),
      ];
      let out = render_template(&tmpl, &msgs, true, "", "<|endoftext|>").expect("render");

      assert!(
            out.contains("<|im_start|>system\nYou are a helpful assistant.<|im_end|>\n"),
            "default system turn missing:\n{out}"
      );
      assert!(
            out.contains("<|im_start|>user\nHello<|im_end|>\n"),
            "first user turn missing:\n{out}"
      );
      assert!(
            out.contains("<|im_start|>assistant\nHi there.<|im_end|>\n"),
            "assistant turn missing:\n{out}"
      );
      assert!(
            out.contains("<|im_start|>user\nHow are you?<|im_end|>\n"),
            "second user turn missing:\n{out}"
      );
      assert!(
            out.ends_with("<|im_start|>assistant\n"),
            "generation prompt not appended:\n{out}"
      );
}

#[test]
fn qwen25_no_generation_prompt_omits_trailing_assistant() {
      let tmpl = qwen_template();
      let msgs = vec![Msg::new("user", "Hello")];
      let out = render_template(&tmpl, &msgs, false, "", "<|endoftext|>").expect("render");
      assert!(
            !out.ends_with("<|im_start|>assistant\n"),
            "generation prompt appended when not requested:\n{out}"
      );
}
