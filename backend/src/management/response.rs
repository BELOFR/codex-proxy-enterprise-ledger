//! 管理响应与中文错误的统一编码

use gateway_plugin_sdk::{
    ErrorCode, PluginFault, call::management::ManagementResponse, client::TypedReply,
};
use serde::Serialize;
use serde_json::json;

pub(crate) struct ApiError {
    pub status: u16,
    pub message: String,
}
pub(crate) type ApiResult = std::result::Result<TypedReply<ManagementResponse>, ApiError>;

impl ApiError {
    pub(crate) fn host(message: String) -> Self {
        Self {
            status: if message.contains("版本冲突") {
                409
            } else {
                502
            },
            message,
        }
    }
    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self {
            status: 400,
            message: message.into(),
        }
    }
    pub(crate) fn reply(self) -> std::result::Result<TypedReply<ManagementResponse>, PluginFault> {
        encode(self.status, &json!({"error":{"message":self.message}}))
    }
}
impl From<String> for ApiError {
    fn from(message: String) -> Self {
        Self::invalid(message)
    }
}

pub(crate) fn json(body: &impl Serialize) -> ApiResult {
    encode(200, body).map_err(|_| ApiError {
        status: 500,
        message: "接口响应编码失败".into(),
    })
}

fn encode(
    status: u16,
    body: &impl Serialize,
) -> std::result::Result<TypedReply<ManagementResponse>, PluginFault> {
    let payload = serde_json::to_vec(body)
        .map_err(|_| PluginFault::new(ErrorCode::Fault, "管理响应编码失败"))?;
    Ok(TypedReply::new(ManagementResponse {
        status,
        content_type: "application/json".into(),
        headers: vec![],
    })
    .with_payload(payload))
}
