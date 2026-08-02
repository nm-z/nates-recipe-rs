use recipe_hsa::Runtime;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let runtime = Runtime::open_default()?;
	{
		let discovery = runtime.discover()?;
		println!("system: {:#?}", discovery.system());
		for (index, agent) in discovery.agents().iter().enumerate() {
			println!("agent {index}: {:#?}", agent.description());
		}
	}
	runtime.close()?;
	Ok(())
}
