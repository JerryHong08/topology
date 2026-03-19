use crate::graph::Graph;

pub fn print_json(graph: &Graph) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(graph)?;
    println!("{json}");
    Ok(())
}
