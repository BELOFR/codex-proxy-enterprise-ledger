//! 同一次请求快照的汇总、期间支出与十进制费用分摊

use crate::data::*;
use chrono::{DateTime, Utc};
use gateway_plugin_sdk::call::services::usage::{UsageCost, UsageCostSource, UsageRecordFact};
use rust_decimal::{Decimal, RoundingStrategy, prelude::ToPrimitive};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Money {
    pub currency: String,
    pub amount: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cost {
    pub currency: String,
    pub provider_reported: String,
    pub calculated: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    pub requests: u64,
    pub completed: u64,
    pub total_tokens: String,
    pub unknown_tokens: u64,
    pub unknown_costs: u64,
    pub usage_costs: Vec<Cost>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub name: String,
    pub metrics: Metrics,
    pub allocations: Vec<Money>,
    pub budgets: Vec<Money>,
    pub budget_status: Vec<BudgetStatus>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetStatus {
    pub currency: String,
    pub budget: String,
    pub allocated: String,
    pub remaining: String,
    pub usage_percent: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub id: String,
    pub started_at_ms: i64,
    pub time_display: String,
    pub key_id: String,
    pub person: String,
    pub department: String,
    pub project: String,
    pub provider: String,
    pub account_id: Option<String>,
    pub model: String,
    pub outcome: String,
    pub request_counted: bool,
    pub usage_counted: bool,
    pub total_tokens: Option<String>,
    pub input_tokens: Option<String>,
    pub output_tokens: Option<String>,
    pub cached_tokens: Option<String>,
    pub cost: Option<String>,
    pub cost_currency: Option<String>,
    pub cost_source: Option<String>,
    pub pool_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Allocation {
    pub payment_id: String,
    pub pool_id: String,
    pub request_id: Option<String>,
    pub person: String,
    pub department: String,
    pub project: String,
    pub currency: String,
    pub amount: String,
    pub basis: Basis,
    pub weight: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Expense {
    pub payment_id: String,
    pub pool_id: String,
    pub currency: String,
    pub base: String,
    pub allocated: String,
    pub unallocated: String,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub query: Query,
    pub time_zone: String,
    pub generated_at_ms: i64,
    pub source_anchor_ms: i64,
    pub complete: bool,
    pub metrics: Metrics,
    pub departments: Vec<Group>,
    pub people: Vec<Group>,
    pub projects: Vec<Group>,
    pub models: Vec<Group>,
    pub records: Vec<Record>,
    pub allocations: Vec<Allocation>,
    pub expenses: Vec<Expense>,
    pub payments: Vec<Payment>,
    pub cash: Vec<Money>,
    pub allocation_base: Vec<Money>,
    pub allocated: Vec<Money>,
    pub unallocated: Vec<Money>,
    pub warnings: Vec<String>,
}

pub fn plus(target: &mut BTreeMap<String, Decimal>, key: &str, value: Decimal) -> Result<()> {
    let current = target.entry(key.to_owned()).or_default();
    *current = current.checked_add(value).ok_or("金额汇总越界")?;
    Ok(())
}

fn money(values: BTreeMap<String, Decimal>) -> Vec<Money> {
    values
        .into_iter()
        .map(|(currency, amount)| Money {
            currency,
            amount: amount.to_string(),
        })
        .collect()
}

fn metrics<'a>(records: impl Iterator<Item = &'a Record>) -> Result<Metrics> {
    let mut result = Metrics::default();
    let mut tokens = 0_u64;
    let mut costs: BTreeMap<String, (Decimal, Decimal)> = BTreeMap::new();
    for r in records {
        result.requests += u64::from(r.request_counted);
        if !r.usage_counted {
            continue;
        }
        result.completed += 1;
        if let Some(v) = &r.total_tokens {
            tokens = tokens
                .checked_add(v.parse::<u64>().map_err(|_| "Token 无效")?)
                .ok_or("Token 汇总越界")?;
        } else {
            result.unknown_tokens += 1;
        }
        match (&r.cost, &r.cost_currency, &r.cost_source) {
            (Some(v), Some(ccy), Some(source)) => {
                let value = decimal(v)?;
                let item = costs.entry(ccy.clone()).or_default();
                let part = if source == "provider_reported" {
                    &mut item.0
                } else {
                    &mut item.1
                };
                *part = part.checked_add(value).ok_or("费用汇总越界")?;
            }
            _ => result.unknown_costs += 1,
        }
    }
    result.total_tokens = tokens.to_string();
    result.usage_costs = costs
        .into_iter()
        .map(|(currency, (reported, calculated))| Cost {
            currency,
            provider_reported: reported.to_string(),
            calculated: calculated.to_string(),
        })
        .collect();
    Ok(result)
}

fn matches(pool: &Pool, r: &UsageRecordFact) -> bool {
    !(pool.basis == Basis::Manual
        && pool.account_ids.is_empty()
        && pool.key_ids.is_empty()
        && pool.provider.is_none())
        && (pool.account_ids.is_empty()
            || r.data
                .account_ref
                .as_ref()
                .is_some_and(|a| pool.account_ids.contains(a)))
        && (pool.key_ids.is_empty() || pool.key_ids.contains(&r.data.client_key_ref))
        && pool
            .provider
            .as_ref()
            .is_none_or(|p| r.data.provider.as_ref() == Some(p))
}

fn record(settings: &Settings, period: &Period, fact: &UsageRecordFact) -> Result<Record> {
    let assigned = settings.assignment(
        &fact.data.client_key_ref,
        fact.data.started_at_ms,
        period.tz,
    )?;
    let pools = settings
        .pools
        .iter()
        .filter(|p| matches(p, fact))
        .collect::<Vec<_>>();
    if pools.len() > 1 {
        return Err(format!(
            "请求 {} 匹配多个费用池，请收窄账号、Key 或 Provider 条件",
            fact.id
        ));
    }
    let (cost, currency, source) = match &fact.data.cost {
        UsageCost::Unavailable => (None, None, None),
        UsageCost::Known {
            source,
            amount,
            currency,
        } => {
            if decimal(amount)? < Decimal::ZERO {
                return Err("原生请求费用不能为负".into());
            }
            (
                Some(amount.clone()),
                Some(currency.clone()),
                Some(
                    match source {
                        UsageCostSource::ProviderReported => "provider_reported",
                        UsageCostSource::Calculated => "calculated",
                    }
                    .into(),
                ),
            )
        }
    };
    let local = DateTime::<Utc>::from_timestamp_millis(fact.data.started_at_ms)
        .ok_or("请求时间无效")?
        .with_timezone(&period.tz);
    Ok(Record {
        id: fact.id.clone(),
        started_at_ms: fact.data.started_at_ms,
        time_display: local.format("%Y-%m-%d %H:%M:%S %:z").to_string(),
        key_id: fact.data.client_key_ref.clone(),
        person: assigned.map_or(UNASSIGNED, |a| &a.person).into(),
        department: assigned.map_or(UNASSIGNED, |a| &a.department).into(),
        project: assigned.map_or(UNASSIGNED, |a| &a.project).into(),
        provider: fact.data.provider.clone().unwrap_or_default(),
        account_id: fact.data.account_ref.clone(),
        model: fact.data.model.clone().unwrap_or_else(|| UNASSIGNED.into()),
        outcome: serde_json::to_value(fact.data.outcome)
            .map_err(|_| "结果编码失败")?
            .as_str()
            .unwrap_or("unknown")
            .into(),
        request_counted: fact.request_counted,
        usage_counted: fact.usage_counted,
        total_tokens: fact.data.tokens.total_tokens.map(|v| v.0.to_string()),
        input_tokens: fact.data.tokens.input_tokens.map(|v| v.0.to_string()),
        output_tokens: fact.data.tokens.output_tokens.map(|v| v.0.to_string()),
        cached_tokens: fact
            .data
            .tokens
            .cached_input_tokens
            .map(|v| v.0.to_string()),
        cost,
        cost_currency: currency,
        cost_source: source,
        pool_id: pools.first().map(|p| p.id.clone()),
    })
}

fn groups(
    report: &Report,
    settings: &Settings,
    period: &Period,
    dimension: u8,
) -> Result<Vec<Group>> {
    let r_name = |r: &Record| match dimension {
        0 => r.department.clone(),
        1 => r.person.clone(),
        2 => r.project.clone(),
        _ => r.model.clone(),
    };
    let a_name = |a: &Allocation| match dimension {
        0 => a.department.clone(),
        1 => a.person.clone(),
        _ => a.project.clone(),
    };
    let mut names = report
        .records
        .iter()
        .filter(|r| r.request_counted || r.usage_counted)
        .map(r_name)
        .collect::<BTreeSet<_>>();
    if dimension < 3 {
        names.extend(report.allocations.iter().map(a_name));
    }
    for b in &settings.budgets {
        if ((dimension == 0 && b.scope == BudgetScope::Department)
            || (dimension == 2 && b.scope == BudgetScope::Project))
            && prorate(decimal(&b.amount)?, &b.start_date, &b.end_date, period)? != Decimal::ZERO
        {
            names.insert(b.name.clone());
        }
    }
    let mut rows = vec![];
    for name in names {
        let mut allocated = BTreeMap::new();
        let mut budgets = BTreeMap::new();
        if dimension < 3 {
            for a in &report.allocations {
                if a_name(a) == name {
                    plus(&mut allocated, &a.currency, decimal(&a.amount)?)?;
                }
            }
        }
        for b in &settings.budgets {
            if b.name == name
                && ((dimension == 0 && b.scope == BudgetScope::Department)
                    || (dimension == 2 && b.scope == BudgetScope::Project))
            {
                plus(
                    &mut budgets,
                    &b.currency,
                    prorate(decimal(&b.amount)?, &b.start_date, &b.end_date, period)?
                        .round_dp(scale(&b.currency)),
                )?;
            }
        }
        let mut budget_status = vec![];
        for (currency, budget) in &budgets {
            let used = allocated.get(currency).copied().unwrap_or_default();
            let percent = if *budget == Decimal::ZERO {
                None
            } else {
                Some(
                    used.checked_mul(Decimal::ONE_HUNDRED)
                        .and_then(|v| v.checked_div(*budget))
                        .ok_or("预算比例越界")?
                        .round_dp(1)
                        .to_string(),
                )
            };
            budget_status.push(BudgetStatus {
                currency: currency.clone(),
                budget: budget.to_string(),
                allocated: used.to_string(),
                remaining: (*budget - used).to_string(),
                usage_percent: percent,
            });
        }
        rows.push(Group {
            name: name.clone(),
            metrics: metrics(report.records.iter().filter(|r| r_name(r) == name))?,
            allocations: money(allocated),
            budgets: money(budgets),
            budget_status,
        });
    }
    Ok(rows)
}

struct Weight<'a> {
    request: Option<&'a Record>,
    person: &'a str,
    department: &'a str,
    project: &'a str,
    value: Decimal,
}

fn split_amount(
    base: Decimal,
    weights: &[Weight<'_>],
    sum: Decimal,
    currency: &str,
) -> Result<Vec<Decimal>> {
    let factor = Decimal::from(10_u64.pow(scale(currency)));
    let units = base.abs().checked_mul(factor).ok_or("分摊金额越界")?;
    let mut values = vec![];
    let mut remainders = vec![];
    let mut used = Decimal::ZERO;
    for (i, w) in weights.iter().enumerate() {
        let exact = units
            .checked_mul(w.value)
            .and_then(|v| v.checked_div(sum))
            .ok_or("分摊权重越界")?;
        let whole = exact.round_dp_with_strategy(0, RoundingStrategy::ToZero);
        used = used.checked_add(whole).ok_or("分摊总量越界")?;
        values.push(whole);
        remainders.push((i, exact - whole));
    }
    let remaining = (units - used).to_usize().ok_or("分摊尾差越界")?;
    if remaining > weights.len() {
        return Err("分摊尾差超出行数".into());
    }
    // 按最大余数及稳定行序分配最小币种单位，避免小金额被舍入成负分摊
    remainders.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (i, _) in remainders.into_iter().take(remaining) {
        values[i] += Decimal::ONE;
    }
    Ok(values
        .into_iter()
        .map(|v| {
            if base.is_sign_negative() {
                -v / factor
            } else {
                v / factor
            }
        })
        .collect())
}

fn weights<'a>(
    pool: &'a Pool,
    records: &'a [Record],
    payment: &Payment,
    period: &Period,
) -> Result<std::result::Result<Vec<Weight<'a>>, String>> {
    if pool.basis == Basis::Manual {
        return Ok(Ok(pool
            .shares
            .iter()
            .map(|s| {
                Ok(Weight {
                    request: None,
                    person: &s.person,
                    department: &s.department,
                    project: &s.project,
                    value: decimal(&s.percent)?,
                })
            })
            .collect::<Result<_>>()?));
    }
    let (start, end) = interval(&payment.start_date, &payment.end_date)?;
    let mut values = vec![];
    let mut currencies = BTreeSet::new();
    for r in records {
        let day = DateTime::<Utc>::from_timestamp_millis(r.started_at_ms)
            .ok_or("请求日期无效")?
            .with_timezone(&period.tz)
            .date_naive();
        if !r.usage_counted || r.pool_id.as_ref() != Some(&pool.id) || day < start || day >= end {
            continue;
        }
        let value = match pool.basis {
            Basis::Tokens => match &r.total_tokens {
                Some(v) => decimal(v)?,
                None => return Ok(Err("池内存在未知 Token".into())),
            },
            Basis::Cost => match (&r.cost, &r.cost_currency) {
                (Some(v), Some(c)) => {
                    currencies.insert(c.clone());
                    decimal(v)?
                }
                _ => return Ok(Err("池内存在未知费用".into())),
            },
            Basis::Manual => unreachable!(),
        };
        if value > Decimal::ZERO {
            values.push(Weight {
                request: Some(r),
                person: &r.person,
                department: &r.department,
                project: &r.project,
                value,
            });
        }
    }
    if currencies.len() > 1 {
        return Ok(Err("金额权重包含多种币种，请拆分费用池".into()));
    }
    if values.is_empty() {
        return Ok(Err("没有合格用量或总权重为零".into()));
    }
    values.sort_by_key(|w| w.request.map(|r| r.id.as_str()));
    Ok(Ok(values))
}

pub fn build(
    settings: &Settings,
    query: Query,
    facts: Vec<UsageRecordFact>,
    anchor: i64,
) -> Result<Report> {
    settings.validate()?;
    let period = query.period(&settings.time_zone)?;
    if facts.len() > MAX_RECORDS {
        return Err("请求超过50000条，请缩小查询范围".into());
    }
    let mut ids = BTreeSet::new();
    let mut records = vec![];
    for fact in &facts {
        if !ids.insert(&fact.id) {
            return Err("扫描返回重复请求，报表未生成".into());
        }
        if fact.data.started_at_ms < period.start_ms || fact.data.started_at_ms >= period.end_ms {
            return Err("扫描返回范围外记录".into());
        }
        records.push(record(settings, &period, fact)?);
    }
    let mut report = Report {
        query,
        time_zone: settings.time_zone.clone(),
        generated_at_ms: Utc::now().timestamp_millis(),
        source_anchor_ms: anchor,
        complete: true,
        metrics: metrics(records.iter())?,
        departments: vec![],
        people: vec![],
        projects: vec![],
        models: vec![],
        records,
        allocations: vec![],
        expenses: vec![],
        payments: vec![],
        cash: vec![],
        allocation_base: vec![],
        allocated: vec![],
        unallocated: vec![],
        warnings: vec![
            "请求明细统计仅覆盖宿主仍保留的原生请求，不包含无明细的外部聚合贡献".into(),
            "用量金额与实际付款分别列示，成功交付用量不等于 Key 结算账本或供应商结算单".into(),
        ],
    };
    let mut cash = BTreeMap::new();
    let mut bases = BTreeMap::new();
    let mut allocated = BTreeMap::new();
    let mut pending = BTreeMap::new();
    for payment in &settings.payments {
        let paid = date(&payment.paid_date)?;
        if period.start <= paid && paid < period.end {
            plus(&mut cash, &payment.currency, decimal(&payment.paid_amount)?)?;
            report.payments.push(payment.clone());
        }
        let base = prorate(
            decimal(&payment.allocation_base)?,
            &payment.start_date,
            &payment.end_date,
            &period,
        )?
        .round_dp(scale(&payment.currency));
        if base == Decimal::ZERO {
            continue;
        }
        plus(&mut bases, &payment.currency, base)?;
        let pool = settings
            .pools
            .iter()
            .find(|p| p.id == payment.pool_id)
            .ok_or("费用池不存在")?;
        let resolved = weights(pool, &report.records, payment, &period)?;
        let reason = match resolved {
            Err(reason) => Some(reason),
            Ok(weights) => {
                let sum = weights.iter().try_fold(Decimal::ZERO, |a, w| {
                    a.checked_add(w.value).ok_or("权重总量越界")
                })?;
                if sum == Decimal::ZERO {
                    Some("总权重为零".into())
                } else {
                    let amounts = split_amount(base, &weights, sum, &payment.currency)?;
                    let mut used = Decimal::ZERO;
                    for (i, w) in weights.iter().enumerate() {
                        let amount = amounts[i];
                        used = used.checked_add(amount).ok_or("分摊合计越界")?;
                        report.allocations.push(Allocation {
                            payment_id: payment.id.clone(),
                            pool_id: pool.id.clone(),
                            request_id: w.request.map(|r| r.id.clone()),
                            person: w.person.into(),
                            department: w.department.into(),
                            project: w.project.into(),
                            currency: payment.currency.clone(),
                            amount: amount.to_string(),
                            basis: pool.basis,
                            weight: w.value.to_string(),
                        });
                    }
                    plus(&mut allocated, &payment.currency, used)?;
                    None
                }
            }
        };
        let remainder = if reason.is_some() {
            base
        } else {
            Decimal::ZERO
        };
        plus(&mut pending, &payment.currency, remainder)?;
        report.expenses.push(Expense {
            payment_id: payment.id.clone(),
            pool_id: pool.id.clone(),
            currency: payment.currency.clone(),
            base: base.to_string(),
            allocated: (base - remainder).to_string(),
            unallocated: remainder.to_string(),
            reason,
        });
    }
    report.cash = money(cash);
    report.allocation_base = money(bases);
    report.allocated = money(allocated);
    report.unallocated = money(pending);
    report.departments = groups(&report, settings, &period, 0)?;
    report.people = groups(&report, settings, &period, 1)?;
    report.projects = groups(&report, settings, &period, 2)?;
    report.models = groups(&report, settings, &period, 3)?;
    if report.metrics.unknown_costs > 0 {
        report.warnings.push(format!(
            "{} 条完整交付请求的费用未知",
            report.metrics.unknown_costs
        ));
    }
    if report
        .records
        .iter()
        .any(|r| r.request_counted && r.department == UNASSIGNED)
    {
        report
            .warnings
            .push("存在未归属 Key，请补齐请求当时的组织归属".into());
    }
    Ok(report)
}
