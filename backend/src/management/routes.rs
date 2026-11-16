//! 台账管理操作，快照与配置在同一 CAS 记录中提交

use super::{
    CachedFile, CachedReport, Runtime,
    response::{ApiError, ApiResult, json},
};
use crate::{
    data::*,
    host::{self, StoredReport},
};
use gateway_plugin_sdk::{
    PluginFault,
    call::{
        management::{ManagementRequest, ManagementResponse},
        middleware::MiddlewareHeader,
    },
    client::{TypedCall, TypedReply},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::json as value;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SaveSettings {
    expected_version: Option<u64>,
    settings: Settings,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReportId {
    report_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Close {
    report_id: String,
    label: String,
    reason: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Import {
    expected_version: Option<u64>,
    csv: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DeleteClose {
    report_id: String,
    expected_version: Option<u64>,
}

fn decode<T: DeserializeOwned>(
    call: &TypedCall<ManagementRequest>,
) -> std::result::Result<T, ApiError> {
    if call.payload.len() > 768 * 1024 {
        return Err(ApiError::invalid("请求正文超过768 KiB"));
    }
    serde_json::from_slice(&call.payload).map_err(|_| ApiError::invalid("请求 JSON 格式或字段无效"))
}

fn cached(runtime: &Runtime, id: &str) -> std::result::Result<Arc<CachedReport>, ApiError> {
    let mut map = runtime
        .reports
        .lock()
        .map_err(|_| ApiError::host("报表缓存不可用".into()))?;
    map.retain(|_, v| v.expires > Instant::now());
    map.get(id).cloned().ok_or_else(|| ApiError {
        status: 410,
        message: "报表已过期，请重新查询或打开月结".into(),
    })
}

fn insert(
    runtime: &Runtime,
    report: crate::report::Report,
    settings: Settings,
    version: Option<u64>,
) -> std::result::Result<String, ApiError> {
    let mut map = runtime
        .reports
        .lock()
        .map_err(|_| ApiError::host("报表缓存不可用".into()))?;
    map.retain(|_, v| v.expires > Instant::now());
    if map.len() >= 4 {
        let oldest = map
            .iter()
            .min_by_key(|(_, v)| v.expires)
            .map(|(id, _)| id.clone());
        if let Some(id) = oldest {
            map.remove(&id);
        }
    }
    let id = uuid::Uuid::new_v4().to_string();
    map.insert(
        id.clone(),
        Arc::new(CachedReport {
            report: Arc::new(report),
            settings: Arc::new(settings),
            state_version: version,
            expires: Instant::now() + Duration::from_secs(900),
        }),
    );
    Ok(id)
}

fn view(runtime: &Runtime, id: &str) -> ApiResult {
    let cache = cached(runtime, id)?;
    let r = &cache.report;
    json(
        &value!({"id":id,"query":r.query,"timeZone":r.time_zone,"generatedAtMs":r.generated_at_ms,"sourceAnchorMs":r.source_anchor_ms,"complete":r.complete,"metrics":r.metrics,"departments":r.departments,"people":r.people,"projects":r.projects,"models":r.models,"cash":r.cash,"allocationBase":r.allocation_base,"allocated":r.allocated,"unallocated":r.unallocated,"expenses":r.expenses,"warnings":r.warnings,"recordCount":r.records.len(),"records":r.records.iter().take(100).collect::<Vec<_>>(),"allocationCount":r.allocations.len()}),
    )
}

pub(crate) async fn handle(
    runtime: &Runtime,
    call: TypedCall<ManagementRequest>,
) -> std::result::Result<TypedReply<ManagementResponse>, PluginFault> {
    match route(runtime, call).await {
        Ok(reply) => Ok(reply),
        Err(error) => error.reply(),
    }
}

async fn route(runtime: &Runtime, call: TypedCall<ManagementRequest>) -> ApiResult {
    if !call.request.query.is_empty() {
        return Err(ApiError::invalid("管理接口不接受查询参数"));
    }
    if call.request.method == "GET" && !call.payload.is_empty() {
        return Err(ApiError::invalid("GET 请求不能包含正文"));
    }
    match (call.request.method.as_str(), call.request.path.as_str()) {
        ("GET", "api/state") => {
            let (version, state) = host::load(&call.host).await.map_err(ApiError::host)?;
            let closed=state.closed_reports.iter().map(|r|value!({"id":r.id,"label":r.label,"reason":r.reason,"closedAtMs":r.closed_at_ms,"query":r.report.query,"records":r.report.records.len()})).collect::<Vec<_>>();
            json(
                &value!({"version":version,"settings":state.settings,"closedReports":closed,"pluginVersion":env!("CARGO_PKG_VERSION"),"maximumRecords":MAX_RECORDS}),
            )
        }
        ("GET", "api/keys") => json(&host::keys(&call.host).await.map_err(ApiError::host)?),
        ("POST", "api/state") => {
            let input: SaveSettings = decode(&call)?;
            input.settings.validate()?;
            let (version, mut state) = host::load(&call.host).await.map_err(ApiError::host)?;
            if input.expected_version != version {
                return Err(ApiError {
                    status: 409,
                    message: "记录版本冲突，请重新加载后再保存".into(),
                });
            }
            state.settings = input.settings;
            let version = host::save(&call.host, version, &state)
                .await
                .map_err(ApiError::host)?;
            json(&value!({"version":version}))
        }
        ("POST", "api/import-payments") => {
            let input: Import = decode(&call)?;
            let (version, mut state) = host::load(&call.host).await.map_err(ApiError::host)?;
            if input.expected_version != version {
                return Err(ApiError {
                    status: 409,
                    message: "记录版本冲突，请重新加载后再导入".into(),
                });
            }
            let mut reader =
                csv::Reader::from_reader(input.csv.trim_start_matches('\u{feff}').as_bytes());
            let mut imported = 0;
            for row in reader.deserialize::<Payment>() {
                let payment = row.map_err(|e| ApiError::invalid(format!("CSV 行无法解析：{e}")))?;
                if state.settings.payments.iter().any(|p| p.id == payment.id) {
                    return Err(ApiError::invalid(format!(
                        "付款ID {} 已存在，整批未导入",
                        payment.id
                    )));
                }
                state.settings.payments.push(payment);
                imported += 1;
                if imported > 2000 {
                    return Err(ApiError::invalid("导入超过2000行"));
                }
            }
            if imported == 0 {
                return Err(ApiError::invalid("没有可导入的付款"));
            }
            state.settings.validate()?;
            let version = host::save(&call.host, version, &state)
                .await
                .map_err(ApiError::host)?;
            json(&value!({"version":version,"imported":imported}))
        }
        ("POST", "api/report") => {
            let query: Query = decode(&call)?;
            let permit = Arc::clone(&runtime.work)
                .try_acquire_owned()
                .map_err(|_| ApiError {
                    status: 429,
                    message: "另一个报表任务正在运行，请稍后重试".into(),
                })?;
            let (version, state) = host::load(&call.host).await.map_err(ApiError::host)?;
            let report = host::report(&call.host, &state.settings, query, permit)
                .await
                .map_err(ApiError::host)?;
            let id = insert(runtime, report, state.settings, version)?;
            view(runtime, &id)
        }
        ("POST", "api/export") => {
            let input: ReportId = decode(&call)?;
            let cache = cached(runtime, &input.report_id)?;
            let permit = Arc::clone(&runtime.work)
                .try_acquire_owned()
                .map_err(|_| ApiError {
                    status: 429,
                    message: "另一个报表任务正在运行".into(),
                })?;
            let export_cache = Arc::clone(&cache);
            let bytes = tokio::task::spawn_blocking(move || {
                let _permit = permit;
                crate::export::workbook(&export_cache.report, &export_cache.settings)
            })
            .await
            .map_err(|_| ApiError::host("Excel 生成任务未完成".into()))??;
            if bytes.len() > 16 * 1024 * 1024 {
                return Err(ApiError::invalid("Excel 超过16 MiB，请缩小查询范围"));
            }
            let size = bytes.len();
            let id = uuid::Uuid::new_v4().to_string();
            {
                let mut files = runtime
                    .files
                    .lock()
                    .map_err(|_| ApiError::host("导出缓存不可用".into()))?;
                files.retain(|_, f| f.expires > Instant::now());
                if files.len() >= 4 {
                    return Err(ApiError {
                        status: 429,
                        message: "待下载文件较多，请先下载已有文件或等待过期".into(),
                    });
                }
                files.insert(
                    id.clone(),
                    CachedFile {
                        bytes: Arc::new(bytes),
                        expires: Instant::now() + Duration::from_secs(600),
                    },
                );
            }
            json(
                &value!({"fileId":id,"bytes":size,"expiresInSeconds":600,"filename":format!("enterprise-ledger-{}-{}.xlsx",cache.report.query.start_date,cache.report.query.end_date)}),
            )
        }
        ("POST", "api/close") => {
            let input: Close = decode(&call)?;
            if input.label.trim().is_empty()
                || input.label.chars().count() > 128
                || input.reason.chars().count() > 512
            {
                return Err(ApiError::invalid("月结名称或调整说明无效"));
            }
            let cache = cached(runtime, &input.report_id)?;
            let (version, mut state) = host::load(&call.host).await.map_err(ApiError::host)?;
            if version != cache.state_version {
                return Err(ApiError {
                    status: 409,
                    message: "配置已变化，请重新查询后月结".into(),
                });
            }
            if state
                .closed_reports
                .iter()
                .any(|r| r.report.query == cache.report.query)
                && input.reason.trim().is_empty()
            {
                return Err(ApiError::invalid("同一期间再次月结须填写调整说明"));
            }
            let id = uuid::Uuid::new_v4().to_string();
            state.closed_reports.push(StoredReport {
                id: id.clone(),
                label: input.label,
                reason: input.reason,
                closed_at_ms: chrono::Utc::now().timestamp_millis(),
                settings: (*cache.settings).clone(),
                report: (*cache.report).clone(),
            });
            let version = host::save(&call.host, version, &state)
                .await
                .map_err(ApiError::host)?;
            json(&value!({"id":id,"version":version}))
        }
        ("POST", "api/delete-close") => {
            let input: DeleteClose = decode(&call)?;
            let (version, mut state) = host::load(&call.host).await.map_err(ApiError::host)?;
            if version != input.expected_version {
                return Err(ApiError {
                    status: 409,
                    message: "记录版本冲突，请重新加载后再移除".into(),
                });
            }
            if !state.closed_reports.iter().any(|r| r.id == input.report_id) {
                return Err(ApiError {
                    status: 404,
                    message: "月结版本不存在".into(),
                });
            }
            state.closed_reports.retain(|r| r.id != input.report_id);
            let version = host::save(&call.host, version, &state)
                .await
                .map_err(ApiError::host)?;
            json(&value!({"version":version}))
        }
        ("POST", "api/open") => {
            let input: ReportId = decode(&call)?;
            let (version, state) = host::load(&call.host).await.map_err(ApiError::host)?;
            let stored = state
                .closed_reports
                .into_iter()
                .find(|r| r.id == input.report_id)
                .ok_or_else(|| ApiError {
                    status: 404,
                    message: "月结版本不存在".into(),
                })?;
            let id = insert(runtime, stored.report, stored.settings, version)?;
            view(runtime, &id)
        }
        _ => Err(ApiError {
            status: 404,
            message: "插件接口不存在".into(),
        }),
    }
}

pub(crate) fn download(
    runtime: &Runtime,
    call: TypedCall<ManagementRequest>,
) -> std::result::Result<TypedReply<ManagementResponse>, PluginFault> {
    let reply = || -> ApiResult {
        if call.request.method != "GET" || call.request.path != "download" {
            return Err(ApiError::invalid("下载入口无效"));
        }
        let ids = url::form_urlencoded::parse(call.request.query.as_bytes())
            .filter(|(key, _)| key == "id")
            .map(|(_, value)| value.into_owned())
            .collect::<Vec<_>>();
        if ids.len() != 1 || uuid::Uuid::parse_str(&ids[0]).is_err() {
            return Err(ApiError::invalid("缺少有效文件ID"));
        }
        let file = {
            let mut files = runtime
                .files
                .lock()
                .map_err(|_| ApiError::host("导出缓存不可用".into()))?;
            files.retain(|_, f| f.expires > Instant::now());
            files.remove(&ids[0]).ok_or_else(|| ApiError {
                status: 410,
                message: "文件已下载或过期，请重新导出".into(),
            })?
        };
        Ok(TypedReply::new(ManagementResponse {
            status: 200,
            content_type: crate::export::MIME.into(),
            headers: vec![
                MiddlewareHeader {
                    name: "content-disposition".into(),
                    value: format!("attachment; filename=\"enterprise-ledger-{}.xlsx\"", ids[0])
                        .into_bytes(),
                },
                MiddlewareHeader {
                    name: "cache-control".into(),
                    value: b"no-store".to_vec(),
                },
                MiddlewareHeader {
                    name: "x-content-type-options".into(),
                    value: b"nosniff".to_vec(),
                },
            ],
        })
        .with_payload((*file.bytes).clone()))
    };
    match reply() {
        Ok(v) => Ok(v),
        Err(e) => e.reply(),
    }
}
