//! 组织归属、付款、费用池与预算的校验及日历范围

use chrono::{DateTime, Days, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::str::FromStr;

pub type Result<T> = std::result::Result<T, String>;
pub const MAX_STATE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_RECORDS: usize = 50_000;
pub const UNASSIGNED: &str = "未归属";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub time_zone: String,
    pub assignments: Vec<Assignment>,
    pub pools: Vec<Pool>,
    pub payments: Vec<Payment>,
    pub budgets: Vec<Budget>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            time_zone: "Asia/Shanghai".into(),
            assignments: vec![],
            pools: vec![],
            payments: vec![],
            budgets: vec![],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Assignment {
    pub id: String,
    pub key_id: String,
    pub person: String,
    pub department: String,
    pub project: String,
    pub start_date: String,
    pub end_date: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    Tokens,
    Cost,
    Manual,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Share {
    pub person: String,
    pub department: String,
    pub project: String,
    pub percent: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Pool {
    pub id: String,
    pub name: String,
    pub basis: Basis,
    #[serde(default)]
    pub account_ids: Vec<String>,
    #[serde(default)]
    pub key_ids: Vec<String>,
    pub provider: Option<String>,
    #[serde(default)]
    pub shares: Vec<Share>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentKind {
    Subscription,
    ApiSettlement,
    Recharge,
    Refund,
    Other,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Payment {
    pub id: String,
    pub paid_date: String,
    pub supplier: String,
    pub voucher: String,
    pub kind: PaymentKind,
    pub pool_id: String,
    pub currency: String,
    pub paid_amount: String,
    pub allocation_base: String,
    pub start_date: String,
    pub end_date: String,
    pub original_payment_id: Option<String>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetScope {
    Department,
    Project,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Budget {
    pub id: String,
    pub scope: BudgetScope,
    pub name: String,
    pub currency: String,
    pub amount: String,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Query {
    pub start_date: String,
    pub end_date: String,
}

#[derive(Clone)]
pub struct Period {
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub start_ms: i64,
    pub end_ms: i64,
    pub tz: Tz,
}

pub fn date(value: &str) -> Result<NaiveDate> {
    if value.len() != 10 {
        return Err("日期须使用 YYYY-MM-DD".into());
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| "日期无效".into())
}

impl Query {
    pub fn period(&self, zone: &str) -> Result<Period> {
        let start = date(&self.start_date)?;
        let end = date(&self.end_date)?
            .checked_add_days(Days::new(1))
            .ok_or("结束日期越界")?;
        if end <= start || (end - start).num_days() > 366 {
            return Err("查询范围须为 1 至 366 天".into());
        }
        let tz: Tz = zone.parse().map_err(|_| "报表时区无效")?;
        let at_midnight = |d: NaiveDate| -> Result<i64> {
            let local = d.and_hms_opt(0, 0, 0).ok_or("日历边界无效")?;
            tz.from_local_datetime(&local)
                .single()
                .map(|d| d.timestamp_millis())
                .ok_or_else(|| "此时区的日历边界不唯一，请调整日期".into())
        };
        Ok(Period {
            start,
            end,
            start_ms: at_midnight(start)?,
            end_ms: at_midnight(end)?,
            tz,
        })
    }
}

pub fn decimal(value: &str) -> Result<Decimal> {
    Decimal::from_str(value).map_err(|_| "金额或比例须为十进制数字".into())
}

pub fn scale(currency: &str) -> u32 {
    if currency == "JPY" { 0 } else { 2 }
}

fn currency(value: &str) -> Result<()> {
    if !["CNY", "USD", "EUR", "GBP", "HKD", "JPY"].contains(&value) {
        return Err("币种支持 CNY、USD、EUR、GBP、HKD、JPY".into());
    }
    Ok(())
}

fn text(value: &str, label: &str, max: usize) -> Result<()> {
    if value.trim().is_empty() || value.chars().count() > max || value.chars().any(char::is_control)
    {
        return Err(format!("{label}不能为空或超过 {max} 字"));
    }
    Ok(())
}

fn unique<'a>(values: impl Iterator<Item = &'a str>, label: &str) -> Result<()> {
    let mut seen = BTreeSet::new();
    for value in values {
        text(value, label, 128)?;
        if !seen.insert(value) {
            return Err(format!("{label}重复：{value}"));
        }
    }
    Ok(())
}

pub fn interval(start: &str, end: &str) -> Result<(NaiveDate, NaiveDate)> {
    let a = date(start)?;
    let b = date(end)?;
    if a >= b {
        return Err("归属开始须早于结束，结束日期不包含在期间内".into());
    }
    Ok((a, b))
}

pub fn prorate(amount: Decimal, start: &str, end: &str, period: &Period) -> Result<Decimal> {
    let (a, b) = interval(start, end)?;
    let days = (b.min(period.end) - a.max(period.start)).num_days().max(0);
    amount
        .checked_mul(Decimal::from(days))
        .and_then(|n| n.checked_div(Decimal::from((b - a).num_days())))
        .ok_or_else(|| "期间金额计算越界".into())
}

impl Settings {
    pub fn validate(&self) -> Result<()> {
        self.time_zone.parse::<Tz>().map_err(|_| "时区无效")?;
        if self.assignments.len() > 1000
            || self.pools.len() > 100
            || self.payments.len() > 2000
            || self.budgets.len() > 1000
        {
            return Err("配置超出容量限制".into());
        }
        unique(self.assignments.iter().map(|x| x.id.as_str()), "归属ID")?;
        unique(self.pools.iter().map(|x| x.id.as_str()), "费用池ID")?;
        unique(self.payments.iter().map(|x| x.id.as_str()), "付款ID")?;
        unique(self.budgets.iter().map(|x| x.id.as_str()), "预算ID")?;
        for (i, a) in self.assignments.iter().enumerate() {
            text(&a.key_id, "Key ID", 256)?;
            for (v, n) in [
                (&a.person, "员工"),
                (&a.department, "部门"),
                (&a.project, "项目"),
            ] {
                text(v, n, 128)?;
            }
            let start = date(&a.start_date)?;
            let end = a
                .end_date
                .as_deref()
                .map(date)
                .transpose()?
                .unwrap_or(NaiveDate::MAX);
            if start >= end {
                return Err("归属有效期无效".into());
            }
            for b in &self.assignments[..i] {
                let b_end = b
                    .end_date
                    .as_deref()
                    .map(date)
                    .transpose()?
                    .unwrap_or(NaiveDate::MAX);
                if a.key_id == b.key_id && start < b_end && date(&b.start_date)? < end {
                    return Err(format!("Key {} 的归属有效期重叠", a.key_id));
                }
            }
        }
        for pool in &self.pools {
            text(&pool.name, "费用池名称", 128)?;
            unique(pool.account_ids.iter().map(String::as_str), "账号ID")?;
            unique(pool.key_ids.iter().map(String::as_str), "Key ID")?;
            if pool
                .provider
                .as_ref()
                .is_some_and(|p| p.trim().is_empty() || p.len() > 64)
            {
                return Err("Provider 无效".into());
            }
            if pool.shares.len() > 1000 {
                return Err("人工分摊行数过多".into());
            }
            if pool.basis == Basis::Manual {
                let mut total = Decimal::ZERO;
                for share in &pool.shares {
                    for (v, n) in [
                        (&share.person, "员工"),
                        (&share.department, "部门"),
                        (&share.project, "项目"),
                    ] {
                        text(v, n, 128)?;
                    }
                    let p = decimal(&share.percent)?;
                    if p <= Decimal::ZERO || p > Decimal::ONE_HUNDRED {
                        return Err("人工分摊比例须大于零且不超过100".into());
                    }
                    total = total.checked_add(p).ok_or("分摊比例越界")?;
                }
                if total != Decimal::ONE_HUNDRED {
                    return Err("人工分摊比例合计须等于100".into());
                }
            } else if !pool.shares.is_empty() {
                return Err("非人工费用池不能配置人工比例".into());
            }
        }
        let mut vouchers = BTreeSet::new();
        for p in &self.payments {
            text(&p.supplier, "供应商", 128)?;
            date(&p.paid_date)?;
            interval(&p.start_date, &p.end_date)?;
            currency(&p.currency)?;
            if !self.pools.iter().any(|v| v.id == p.pool_id) {
                return Err(format!("付款 {} 的费用池不存在", p.id));
            }
            if p.voucher.len() > 128 || p.notes.len() > 1024 {
                return Err("凭证编号或备注过长".into());
            }
            if !p.voucher.is_empty() && !vouchers.insert((&p.supplier, &p.voucher)) {
                return Err("供应商及凭证编号重复，请检查是否重复导入".into());
            }
            let paid = decimal(&p.paid_amount)?;
            let base = decimal(&p.allocation_base)?;
            if paid.abs() > Decimal::from(1_000_000_000_000_i64)
                || base.abs() > Decimal::from(1_000_000_000_000_i64)
                || paid.scale() > scale(&p.currency)
                || base.scale() > scale(&p.currency)
            {
                return Err("金额超出上限或币种精度".into());
            }
            if p.kind == PaymentKind::Recharge && base != Decimal::ZERO {
                return Err("充值的本期分摊基数必须为零，消费结算另行登记".into());
            }
            if p.kind == PaymentKind::Refund {
                let original = self
                    .payments
                    .iter()
                    .find(|v| Some(&v.id) == p.original_payment_id.as_ref())
                    .ok_or("退款须关联原付款")?;
                if original.id == p.id
                    || original.kind == PaymentKind::Refund
                    || original.currency != p.currency
                    || original.pool_id != p.pool_id
                    || paid > Decimal::ZERO
                    || base > Decimal::ZERO
                {
                    return Err("退款的原付款、币种、费用池或正负号无效".into());
                }
            } else if paid < Decimal::ZERO
                || base < Decimal::ZERO
                || p.original_payment_id.is_some()
            {
                return Err("负金额及原付款关联仅用于退款".into());
            }
        }
        for b in &self.budgets {
            text(&b.name, "预算归属", 128)?;
            currency(&b.currency)?;
            interval(&b.start_date, &b.end_date)?;
            let amount = decimal(&b.amount)?;
            if amount < Decimal::ZERO || amount.scale() > scale(&b.currency) {
                return Err("预算须为非负金额并符合币种精度".into());
            }
        }
        if serde_json::to_vec(self).map_err(|_| "配置编码失败")?.len() > 512 * 1024 {
            return Err("配置超过512 KiB，请减少历史付款或归属条目".into());
        }
        Ok(())
    }

    pub fn assignment(&self, key: &str, at_ms: i64, tz: Tz) -> Result<Option<&Assignment>> {
        let at = DateTime::<Utc>::from_timestamp_millis(at_ms)
            .ok_or("请求时间无效")?
            .with_timezone(&tz)
            .date_naive();
        for a in &self.assignments {
            if a.key_id == key
                && date(&a.start_date)? <= at
                && a.end_date
                    .as_deref()
                    .map(date)
                    .transpose()?
                    .is_none_or(|end| at < end)
            {
                return Ok(Some(a));
            }
        }
        Ok(None)
    }
}
