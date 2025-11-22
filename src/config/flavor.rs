fn flavor(environmet: &str) -> &str {
    match environmet {
        "dev" => "Development",
        "prod" => "Prodcution",
        _ => "Unknown",
    }
}

pub fn load_config() -> Result<String, Box<dyn std::error::Error>> {
    let environmet = std::env::var("FLAVOR").unwrap_or_else(|_| "dev".to_string());
    let flavor = flavor(&environmet);

    println!("🚀 Server running on {} mode", flavor);
    let config = format!("{}.toml", environmet);

    Ok(config)
}
