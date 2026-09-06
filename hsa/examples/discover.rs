use recipe_hsa::Runtime;
use write::{block, probe};

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let runtime = Runtime::open_default()?;
	{
		let discovery = runtime.discover()?;
		let mut report = format!("system: {:#?}", discovery.system());
		for (index, agent) in discovery.agents().iter().enumerate() {
			report.push_str(&format!("\nagent {index}: {:#?}", agent.description()));
		}
		block(probe, &report);
	}
	runtime.close()?;
	Ok(())
}
