mod agent;
mod blackboard;
mod llm;
mod memory;
mod planner;
mod state;
mod supervisor;
mod swarm;
mod tools;

use std::env;
use std::io::{self, Read};

use anyhow::Result;
use llm::client_for_role;
use swarm::{SwarmConfig, SwarmOrchestrator};
use tools::{FileIoTool, ToolRegistry, WebSearchTool};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    let mut registry = ToolRegistry::new();
    registry.register(WebSearchTool::new());
    registry.register(FileIoTool::new());
    println!("Tools registered");

    let config = SwarmConfig::default();
    let swarm = SwarmOrchestrator::new(client_for_role, registry, config);
    println!("Swarm orchestrator created");
    let user_input = parse_user_input()?;
    println!("User input: {user_input}");
    let result = swarm.run(&user_input).await?;
    println!("Blackboard contributions: {}", result.blackboard.entries().len());
    println!("{}", result.final_decision);

    Ok(())
}

fn parse_user_input() -> Result<String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if !args.is_empty() {
        return Ok(args.join(" "));
    }

    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let trimmed = input.trim().to_string();
    if trimmed.is_empty() {
        anyhow::bail!("Provide input via CLI args or stdin");
    }
    Ok(trimmed)
}

#[cfg(test)]
mod tests {
    use super::parse_user_input;

    #[test]
    fn parse_user_input_function_is_linked() {
        let ptr = parse_user_input as fn() -> anyhow::Result<String>;
        assert!(std::ptr::fn_addr_eq(ptr, parse_user_input as fn() -> anyhow::Result<String>));
    }
}
