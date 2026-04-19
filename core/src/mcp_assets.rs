use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../mcp/dist"]
#[include = "index.js"]
pub struct McpAssets;
