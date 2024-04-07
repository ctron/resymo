use resymo_agent::config::Config;

fn main() -> anyhow::Result<()> {
    let schema = schemars::schema_for!(Config);
    let path = "deploy/config/schema.json";

    let file = std::fs::File::create(path)?;
    serde_json::to_writer_pretty(file, &schema)?;

    println!("Wrote schema to: {path}");

    Ok(())
}
