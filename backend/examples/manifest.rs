//! 输出 SDK 规范化清单，供离线协议验证使用

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest =
        gateway_plugin_sdk::Manifest::from_author_slice(include_bytes!("../../plugin.json"))?;
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    Ok(())
}
