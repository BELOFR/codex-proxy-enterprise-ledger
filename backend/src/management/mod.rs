//! 管理路由、版本检查、月结与短期导出缓存

mod response;
mod routes;

use crate::{data::Settings, report::Report};
use gateway_plugin_sdk::{
    call::management::*,
    client::{AuthorError, ComposedPlugin, PluginBuilder, methods},
};
pub(crate) use response::ApiError;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Instant,
};

pub(crate) struct CachedReport {
    pub report: Arc<Report>,
    pub settings: Arc<Settings>,
    pub state_version: Option<u64>,
    pub expires: Instant,
}

pub(crate) struct CachedFile {
    pub bytes: Arc<Vec<u8>>,
    pub expires: Instant,
}

pub(crate) struct Runtime {
    pub reports: Mutex<BTreeMap<String, Arc<CachedReport>>>,
    pub files: Mutex<BTreeMap<String, CachedFile>>,
    pub work: Arc<tokio::sync::Semaphore>,
}

pub fn plugin() -> std::result::Result<ComposedPlugin, AuthorError> {
    let runtime = Arc::new(Runtime {
        reports: Mutex::default(),
        files: Mutex::default(),
        work: Arc::new(tokio::sync::Semaphore::new(1)),
    });
    let callback_runtime = Arc::clone(&runtime);
    PluginBuilder::from_json(include_bytes!("../../../plugin.json"))?
        .management(registration(), move |call| {
            let runtime = Arc::clone(&runtime);
            async move { routes::handle(&runtime, call).await }
        })?
        .on(methods::MANAGEMENT_CALLBACK, move |call| {
            let runtime = Arc::clone(&callback_runtime);
            async move { routes::download(&runtime, call) }
        })?
        .build()
}

fn registration() -> ManagementRegistration {
    let get = |path: &str| ManagementRoute {
        method: "GET".into(),
        path: path.into(),
        request_content_types: vec![],
        response_content_types: vec!["application/json".into()],
    };
    let post = |path: &str| ManagementRoute {
        method: "POST".into(),
        path: path.into(),
        request_content_types: vec!["application/json".into()],
        response_content_types: vec!["application/json".into()],
    };
    ManagementRegistration {
        routes: vec![
            get("api/state"),
            get("api/keys"),
            post("api/state"),
            post("api/report"),
            post("api/export"),
            post("api/close"),
            post("api/open"),
            post("api/delete-close"),
            post("api/import-payments"),
        ],
        resources: [
            "web/index.html",
            "web/app.js",
            "web/app.css",
            "web/icon.svg",
        ]
        .into_iter()
        .map(|path| ManagementResource {
            path: path.into(),
            public: false,
        })
        .collect(),
        pages: vec![ManagementPage {
            id: "ledger".into(),
            title: "企业用量与费用台账".into(),
            description: Some("统计用量、核对支出与分摊，导出 Excel".into()),
            entry: "web/index.html".into(),
            icon: Some("web/icon.svg".into()),
        }],
        callbacks: vec![ManagementCallback {
            path: "download".into(),
            response_content_types: vec![crate::export::MIME.into(), "application/json".into()],
        }],
    }
}
