use std::fmt::Write as _;
use std::io::{BufRead, Read, Write};
use std::process::{Command, Stdio};

const REPOSITORY: &str = "nm-z/nates-recipe-rs";

fn quote(value: &str) -> String {
	let mut json = String::from("\"");
	for character in value.chars() {
		match character {
			'\"' => json.push_str("\\\""),
			'\\' => json.push_str("\\\\"),
			'\n' => json.push_str("\\n"),
			'\r' => json.push_str("\\r"),
			'\t' => json.push_str("\\t"),
			value if value <= '\u{1f}' => write!(json, "\\u{:04x}", value as u32).unwrap(),
			value => json.push(value),
		}
	}
	json.push('\"');
	json
}

fn command(binary: &str, arguments: &[&str], input: Option<&str>) -> Result<String, String> {
	let mut command = Command::new(binary);
	command.args(arguments).stdout(Stdio::piped()).stderr(Stdio::piped());
	if input.is_some() { command.stdin(Stdio::piped()); }
	let mut child = command.spawn().map_err(|error| error.to_string())?;
	if let Some(input) = input { child.stdin.take().unwrap().write_all(input.as_bytes()).map_err(|error| error.to_string())?; }
	let output = child.wait_with_output().map_err(|error| error.to_string())?;
	if output.status.success() { return String::from_utf8(output.stdout).map_err(|error| error.to_string()); }
	let error = String::from_utf8_lossy(&output.stderr).trim().to_owned();
	Err(if error.is_empty() { String::from_utf8_lossy(&output.stdout).trim().to_owned() } else { error })
}

fn value(request: &str, filter: &str) -> Result<String, String> {
	command("jq", &["-er", filter], Some(request)).map(|value| value.trim().to_owned())
}

fn content(text: Result<String, String>) -> String {
	let (text, error) = match text { Ok(text) => (text, false), Err(error) => (error, true) };
	format!(r#"{{"content":[{{"type":"text","text":{}}}],"isError":{error}}}"#, quote(&text))
}

fn issues(request: &str) -> Result<String, String> {
	let query = value(request, ".params.arguments.query")?;
	let limit = value(request, ".params.arguments.limit // 10")?.parse::<usize>().map_err(|error| error.to_string())?.clamp(1, 20).to_string();
	let mut arguments = vec!["issue", "list", "-R", REPOSITORY, "--state", "all", "--limit", &limit, "--json", "number,title,state,url"];
	if !query.trim().is_empty() { arguments.extend(["--search", &query]); }
	command("gh", &arguments, None)
}

fn issue(request: &str) -> Result<String, String> {
	let number = value(request, ".params.arguments.number")?;
	number.parse::<u64>().map_err(|error| error.to_string())?;
	command("gh", &["issue", "view", &number, "-R", REPOSITORY, "--json", "number,title,state,url,body,comments"], None)
}

fn response(request: &str) -> Option<String> {
	let method = value(request, ".method").ok()?;
	if method.starts_with("notifications/") { return None; }
	let id = value(request, ".id | tojson").unwrap_or_else(|_| "null".to_owned());
	let result = match method.as_str() {
		"initialize" => {
			let protocol = value(request, ".params.protocolVersion").unwrap_or_else(|_| "2025-06-18".to_owned());
			format!(r#"{{"protocolVersion":{},"capabilities":{{"tools":{{"listChanged":false}}}},"serverInfo":{{"name":"recipe-issue-reader","version":"1"}},"instructions":"Search the bounded GitHub issue catalog, then read every plausible match before deciding. Both tools are read-only."}}"#, quote(&protocol))
		}
		"ping" => "{}".to_owned(),
		"tools/list" => r#"{"tools":[{"name":"search_issues","description":"Search Recipe issues by GitHub issue query. Returns only issue number, title, state, and URL. Use read_issue for every plausible match.","inputSchema":{"type":"object","properties":{"query":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":20}},"required":["query"],"additionalProperties":false},"annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":true}},{"name":"read_issue","description":"Read one complete Recipe issue, including its body and every comment.","inputSchema":{"type":"object","properties":{"number":{"type":"integer","minimum":1}},"required":["number"],"additionalProperties":false},"annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":true}}]}"#.to_owned(),
		"tools/call" => match value(request, ".params.name").as_deref() {
			Ok("search_issues") => content(issues(request)),
			Ok("read_issue") => content(issue(request)),
			Ok(name) => content(Err(format!("unknown read-only tool {name:?}"))),
			Err(error) => content(Err(error.to_owned())),
		},
		_ => return Some(format!(r#"{{"jsonrpc":"2.0","id":{id},"error":{{"code":-32601,"message":"method not found"}}}}"#)),
	};
	Some(format!(r#"{{"jsonrpc":"2.0","id":{id},"result":{result}}}"#))
}

fn hook() {
	let mut input = String::new(); std::io::stdin().read_to_string(&mut input).unwrap();
	let tool = value(&input, ".toolCall.name").unwrap_or_default();
	let issue_tool = tool == "call_mcp_tool"
		&& value(&input, ".toolCall.args.ServerName").as_deref() == Ok("recipe_issues")
		&& matches!(value(&input, ".toolCall.args.ToolName").as_deref(), Ok("search_issues" | "read_issue"));
	let allowed = matches!(tool.as_str(), "view_file" | "grep_search" | "find_by_name" | "list_dir" | "finish") || issue_tool;
	println!(r#"{{"decision":"{}","reason":"Classifier tools are read-only"}}"#, if allowed { "allow" } else { "deny" });
}

fn main() {
	if std::env::args().nth(1).as_deref() == Some("hook") { hook(); return; }
	for line in std::io::stdin().lock().lines() {
		let line = line.unwrap();
		if let Some(response) = response(&line) { println!("{response}"); std::io::stdout().flush().unwrap(); }
	}
}
