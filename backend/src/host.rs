//! 公开 SDK 的状态、Key 摘要与完整用量扫描

use crate::data::{MAX_RECORDS, MAX_STATE_BYTES, Query, Result, Settings};
use crate::report::Report;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use gateway_plugin_sdk::{
    call::{host::*, services::usage::*},
    client::{HostClient, SessionError},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::io::{Read, Write};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompressedState {
    encoding: String,
    payload: String,
}

fn pack(state: &LedgerState) -> Result<serde_json::Value> {
    let raw = serde_json::to_vec(state).map_err(|_| "台账编码失败")?;
    if raw.len() > MAX_STATE_BYTES {
        return Err("台账解压后超过8 MiB，请缩小月结范围或另行归档".into());
    }
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw).map_err(|_| "台账压缩失败")?;
    let compressed = encoder.finish().map_err(|_| "台账压缩失败")?;
    let value = serde_json::to_value(CompressedState {
        encoding: "gzip-base64".into(),
        payload: STANDARD.encode(compressed),
    })
    .map_err(|_| "台账封装失败")?;
    if serde_json::to_vec(&value)
        .map_err(|_| "台账封装失败")?
        .len()
        > 256 * 1024
    {
        return Err("压缩台账超过宿主单条256 KiB限制，请缩小月结范围或保留 Excel 归档".into());
    }
    Ok(value)
}

fn unpack(value: serde_json::Value) -> Result<LedgerState> {
    let record: CompressedState =
        serde_json::from_value(value).map_err(|_| "已保存台账封装无效")?;
    if record.encoding != "gzip-base64" || record.payload.len() > 256 * 1024 {
        return Err("已保存台账编码不受支持".into());
    }
    let bytes = STANDARD
        .decode(record.payload)
        .map_err(|_| "台账编码无效")?;
    let mut reader = GzDecoder::new(bytes.as_slice()).take((MAX_STATE_BYTES + 1) as u64);
    let mut raw = Vec::new();
    reader.read_to_end(&mut raw).map_err(|_| "台账解压失败")?;
    if raw.len() > MAX_STATE_BYTES {
        return Err("台账解压后超过8 MiB".into());
    }
    serde_json::from_slice(&raw).map_err(|_| "已保存台账格式无效".into())
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StoredReport {
    pub id: String,
    pub label: String,
    pub reason: String,
    pub closed_at_ms: i64,
    pub settings: Settings,
    pub report: Report,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LedgerState {
    pub settings: Settings,
    pub closed_reports: Vec<StoredReport>,
}

async fn callback<I: Serialize, O: DeserializeOwned>(
    host: &HostClient,
    method: &str,
    input: &I,
) -> Result<O> {
    let response = host
        .call(
            method,
            serde_json::to_value(input).map_err(|_| "宿主参数编码失败")?,
            vec![],
        )
        .await
        .map_err(|e| safe_fault(e.into_plugin_fault()))?;
    serde_json::from_value(response.result).map_err(|_| "宿主返回的数据格式无效".into())
}

fn safe_fault(fault: gateway_plugin_sdk::PluginFault) -> String {
    if fault.code == gateway_plugin_sdk::ErrorCode::Conflict {
        "记录版本冲突，请重新加载后再保存".into()
    } else {
        format!("宿主操作未完成（{:?}）", fault.code)
    }
}

pub(crate) async fn load(host: &HostClient) -> Result<(Option<u64>, LedgerState)> {
    let response: StateGetResult = callback(
        host,
        "host.state.get",
        &StateGetRequest {
            namespace: "ledger".into(),
            key: "book".into(),
        },
    )
    .await?;
    match response.record {
        Some(record) => {
            if record.schema_version != 1 {
                return Err("已保存台账的格式不受支持".into());
            }
            let value = unpack(record.value)?;
            value.settings.validate()?;
            Ok((Some(record.version), value))
        }
        None => Ok((None, LedgerState::default())),
    }
}

pub(crate) async fn save(
    host: &HostClient,
    expected: Option<u64>,
    state: &LedgerState,
) -> Result<u64> {
    state.settings.validate()?;
    if state.closed_reports.len() > 24 {
        return Err("月结最多保存24个版本，请另行保存导出文件".into());
    }
    let response: StatePutResult = callback(
        host,
        "host.state.put",
        &StatePutRequest {
            namespace: "ledger".into(),
            key: "book".into(),
            value: pack(state)?,
            expected_version: expected,
        },
    )
    .await?;
    Ok(response.version)
}

pub(crate) async fn keys(host: &HostClient) -> Result<Vec<ClientKey>> {
    let mut result = vec![];
    let mut cursor = None;
    let mut seen = std::collections::BTreeSet::new();
    loop {
        let response: KeyListResult = callback(
            host,
            "host.keys.list",
            &KeyListRequest { cursor, limit: 200 },
        )
        .await?;
        result.extend(response.keys);
        if result.len() > 10_000 {
            return Err("Key 数量超过10000，请使用手工输入的稳定 Key ID".into());
        }
        cursor = response.next_cursor;
        let Some(next) = &cursor else {
            return Ok(result);
        };
        if !seen.insert(next.clone()) {
            return Err("Key 游标未前进".into());
        }
    }
}

pub(crate) async fn report(
    host: &HostClient,
    settings: &Settings,
    query: Query,
    permit: tokio::sync::OwnedSemaphorePermit,
) -> Result<Report> {
    let period = query.period(&settings.time_zone)?;
    let mut reader = host
        .scan_usage_records(ScanUsageRecords {
            filter: UsageReadFilter {
                records: UsageRecordFilter {
                    start_ms: period.start_ms,
                    end_ms: period.end_ms,
                    provider: None,
                    model: None,
                    account_ref: None,
                    client_key_ref: None,
                },
                routing_group_ref: None,
            },
            batch_size: Some(1024),
        })
        .await
        .map_err(safe_fault)?;
    let mut facts = vec![];
    let mut anchor = None;
    let mut complete = false;
    while let Some(batch) = reader.next_batch().await.map_err(safe_fault)? {
        if anchor.is_some_and(|v| v != batch.observation.generated_at_ms) {
            reader.close().await.map_err(safe_fault)?;
            return Err("扫描的数据锚点变化，报表未生成".into());
        }
        anchor = Some(batch.observation.generated_at_ms);
        for item in batch.items {
            match item {
                UsageReadItem::Record(fact) => facts.push(*fact),
                _ => {
                    reader.close().await.map_err(safe_fault)?;
                    return Err("扫描返回非明细数据".into());
                }
            }
        }
        if facts.len() > MAX_RECORDS {
            reader.close().await.map_err(safe_fault)?;
            return Err("请求超过50000条，请缩小日期范围".into());
        }
        complete = batch.complete;
        if complete {
            break;
        }
    }
    if !complete {
        return Err("扫描未完整结束，不能生成报表".into());
    }
    let anchor = anchor.ok_or("扫描没有提供数据锚点")?;
    let settings = settings.clone();
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        crate::report::build(&settings, query, facts, anchor)
    })
    .await
    .map_err(|_| "报表计算未完成")?
}

impl From<SessionError> for crate::management::ApiError {
    fn from(value: SessionError) -> Self {
        Self::host(safe_fault(value.into_plugin_fault()))
    }
}
