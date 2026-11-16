//! 以公开 SDK 启动插件会话，标准输出仅用于二进制协议

use gateway_plugin_sdk::{
    CallBudgets, Manifest, Stage,
    client::{PluginSession, SessionConfig, SessionError},
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut session = PluginSession::accept(
        tokio::io::stdin(),
        tokio::io::stdout(),
        SessionConfig::default(),
    )
    .await?;
    let manifest = Manifest::from_author_slice(include_bytes!("../../plugin.json"))?;
    if session.handshake().plugin_id != "belofr.enterprise-ledger"
        || session.handshake().contributes != manifest.contributes
        || !session
            .handshake()
            .configuration
            .as_object()
            .is_some_and(serde_json::Map::is_empty)
    {
        return Err(SessionError::Handshake.into());
    }
    session.set_call_budgets(CallBudgets::from([
        (Stage::Management, 600_000),
        (Stage::PublicManagement, 120_000),
    ]))?;
    session.run(enterprise_ledger::plugin()?).await?;
    Ok(())
}
