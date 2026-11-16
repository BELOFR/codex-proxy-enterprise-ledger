//! 企业台账的领域、报表与插件入口

pub mod data;
pub mod export;
mod host;
mod management;
pub mod report;

pub use management::plugin;
