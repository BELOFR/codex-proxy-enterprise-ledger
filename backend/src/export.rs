//! 原生 Excel 导出，只写入台账白名单字段及安全文本

use crate::data::{Result, Settings};
use crate::report::{Group, Money, Report};
use rust_xlsxwriter::{Color, Format, Workbook, Worksheet, XlsxError};

pub const MIME: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

fn error(e: XlsxError) -> String {
    format!("Excel 生成失败：{e}")
}
fn init<'a>(
    book: &'a mut Workbook,
    name: &str,
    note: &str,
    columns: u16,
) -> Result<&'a mut Worksheet> {
    let sheet = book.add_worksheet();
    sheet.set_name(name).map_err(error)?;
    sheet.set_screen_gridlines(false);
    sheet
        .set_column_range_width(0, columns - 1, 18)
        .map_err(error)?;
    sheet.set_column_width(0, 26).map_err(error)?;
    sheet
        .write_string_with_format(
            1,
            0,
            name,
            &Format::new()
                .set_font_size(15)
                .set_bold()
                .set_font_color(Color::RGB(0x172D46)),
        )
        .map_err(error)?;
    sheet.write_string(2, 0, note).map_err(error)?;
    sheet.set_freeze_panes(6, 1).map_err(error)?;
    Ok(sheet)
}

fn header(sheet: &mut Worksheet, row: u32, values: &[&str]) -> Result<()> {
    let style = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x172D46))
        .set_font_color(Color::White);
    for (c, v) in values.iter().enumerate() {
        sheet
            .write_string_with_format(row, c as u16, *v, &style)
            .map_err(error)?;
    }
    sheet.set_row_height(row, 26).map_err(error)?;
    Ok(())
}

fn text(sheet: &mut Worksheet, row: u32, col: u16, v: &str) -> Result<()> {
    // 显式字符串写入，用户提供的 =、+、-、@ 开头文本不作为公式执行
    sheet.write_string(row, col, v).map_err(error)?;
    Ok(())
}

fn number(sheet: &mut Worksheet, row: u32, col: u16, value: &str, format: &str) -> Result<()> {
    let significant = value.chars().filter(char::is_ascii_digit).count();
    if significant > 15 {
        return text(sheet, row, col, value);
    }
    let n = value.parse::<f64>().map_err(|_| "导出数值无效")?;
    if !n.is_finite() {
        return Err("导出数值越界".into());
    }
    sheet
        .write_number_with_format(row, col, n, &Format::new().set_num_format(format))
        .map_err(error)?;
    Ok(())
}

fn monies(sheet: &mut Worksheet, row: &mut u32, label: &str, values: &[Money]) -> Result<()> {
    if values.is_empty() {
        text(sheet, *row, 0, label)?;
        text(sheet, *row, 1, "无已登记金额")?;
        *row += 1;
    }
    for value in values {
        text(sheet, *row, 0, label)?;
        text(sheet, *row, 1, &value.currency)?;
        number(sheet, *row, 2, &value.amount, "#,##0.00;(#,##0.00)")?;
        *row += 1;
    }
    Ok(())
}

fn group_table(sheet: &mut Worksheet, start: u32, label: &str, groups: &[Group]) -> Result<u32> {
    text(sheet, start, 0, label)?;
    header(
        sheet,
        start + 1,
        &[
            "归属",
            "调用数",
            "完整交付数",
            "已知总Token",
            "未知Token条数",
            "未知费用条数",
            "用量币种",
            "上游报告金额",
            "本地估算金额",
            "分摊币种",
            "已分摊支出",
            "预算币种",
            "期间预算",
            "预算余额",
            "预算使用率%",
        ],
    )?;
    let mut row = start + 2;
    for g in groups {
        let lines = g
            .metrics
            .usage_costs
            .len()
            .max(g.allocations.len())
            .max(g.budgets.len())
            .max(1);
        for i in 0..lines {
            text(sheet, row, 0, &g.name)?;
            if i == 0 {
                for (col, value) in [
                    (1, g.metrics.requests.to_string()),
                    (2, g.metrics.completed.to_string()),
                    (3, g.metrics.total_tokens.clone()),
                    (4, g.metrics.unknown_tokens.to_string()),
                    (5, g.metrics.unknown_costs.to_string()),
                ] {
                    number(sheet, row, col, &value, "#,##0")?;
                }
            }
            if let Some(c) = g.metrics.usage_costs.get(i) {
                text(sheet, row, 6, &c.currency)?;
                number(sheet, row, 7, &c.provider_reported, "0.000000")?;
                number(sheet, row, 8, &c.calculated, "0.000000")?;
            }
            if let Some(a) = g.allocations.get(i) {
                text(sheet, row, 9, &a.currency)?;
                number(sheet, row, 10, &a.amount, "#,##0.00;(#,##0.00)")?;
            }
            if let Some(b) = g.budgets.get(i) {
                text(sheet, row, 11, &b.currency)?;
                number(sheet, row, 12, &b.amount, "#,##0.00")?;
            }
            if let Some(status) = g.budget_status.get(i) {
                number(sheet, row, 13, &status.remaining, "#,##0.00;(#,##0.00)")?;
                if let Some(percent) = &status.usage_percent {
                    number(sheet, row, 14, percent, "0.0")?;
                }
            }
            row += 1;
        }
    }
    Ok(row + 2)
}

pub fn workbook(report: &Report, settings: &Settings) -> Result<Vec<u8>> {
    if !report.complete {
        return Err("扫描未完成，不能导出".into());
    }
    let mut book = Workbook::new();
    let note = format!(
        "{} 至 {}，时区 {}，数据锚点 {}，原生明细快照",
        report.query.start_date, report.query.end_date, report.time_zone, report.source_anchor_ms
    );
    {
        let s = init(&mut book, "统计总览", &note, 15)?;
        header(s, 5, &["指标", "币种或单位", "数值"])?;
        let mut row = 6;
        for (label, value) in [
            ("调用数", report.metrics.requests.to_string()),
            ("完整交付请求数", report.metrics.completed.to_string()),
            ("已知总Token", report.metrics.total_tokens.clone()),
            ("未知Token请求数", report.metrics.unknown_tokens.to_string()),
            ("未知费用请求数", report.metrics.unknown_costs.to_string()),
            ("原始请求行数", report.records.len().to_string()),
        ] {
            text(s, row, 0, label)?;
            number(s, row, 2, &value, "#,##0")?;
            row += 1;
        }
        for cost in &report.metrics.usage_costs {
            for (label, value) in [
                ("上游报告金额", &cost.provider_reported),
                ("本地估算金额", &cost.calculated),
            ] {
                text(s, row, 0, label)?;
                text(s, row, 1, &cost.currency)?;
                number(s, row, 2, value, "0.000000")?;
                row += 1;
            }
        }
        row += 1;
        monies(s, &mut row, "实际付款净额", &report.cash)?;
        monies(s, &mut row, "期间分摊基数", &report.allocation_base)?;
        monies(s, &mut row, "已分摊金额", &report.allocated)?;
        monies(s, &mut row, "未分摊金额", &report.unallocated)?;
        row += 2;
        for warning in &report.warnings {
            text(s, row, 0, warning)?;
            row += 1;
        }
        text(s, row + 1, 0, "模型分布")?;
        group_table(s, row + 2, "模型分布", &report.models)?;
    }
    {
        let s = init(&mut book, "部门人员", &note, 15)?;
        let row = group_table(s, 5, "部门", &report.departments)?;
        group_table(s, row, "人员", &report.people)?;
    }
    {
        let s = init(&mut book, "项目汇总", &note, 15)?;
        group_table(s, 5, "项目", &report.projects)?;
    }
    {
        let s = init(&mut book, "费用分摊", &note, 11)?;
        header(
            s,
            5,
            &[
                "付款ID",
                "费用池",
                "请求ID",
                "员工",
                "部门",
                "项目",
                "币种",
                "分摊金额",
                "分摊依据",
                "权重",
                "说明",
            ],
        )?;
        for (i, a) in report.allocations.iter().enumerate() {
            let row = i as u32 + 6;
            for (c, v) in [
                (0, a.payment_id.as_str()),
                (1, &a.pool_id),
                (2, a.request_id.as_deref().unwrap_or("人工分摊")),
                (3, &a.person),
                (4, &a.department),
                (5, &a.project),
                (6, &a.currency),
            ] {
                text(s, row, c, v)?;
            }
            number(s, row, 7, &a.amount, "#,##0.00;(#,##0.00)")?;
            text(s, row, 8, &format!("{:?}", a.basis))?;
            number(s, row, 9, &a.weight, "0.000000")?;
        }
        let mut row = report.allocations.len() as u32 + 9;
        header(
            s,
            row,
            &[
                "付款ID",
                "费用池",
                "币种",
                "期间基数",
                "已分摊",
                "未分摊",
                "未分摊原因",
            ],
        )?;
        row += 1;
        for e in &report.expenses {
            text(s, row, 0, &e.payment_id)?;
            text(s, row, 1, &e.pool_id)?;
            text(s, row, 2, &e.currency)?;
            for (c, v) in [(3, &e.base), (4, &e.allocated), (5, &e.unallocated)] {
                number(s, row, c, v, "0.00;(#,##0.00)")?;
            }
            text(s, row, 6, e.reason.as_deref().unwrap_or(""))?;
            row += 1;
        }
    }
    {
        let s = init(&mut book, "请求明细", &note, 21)?;
        header(
            s,
            5,
            &[
                "请求ID",
                "时间（含时区）",
                "Key ID",
                "员工",
                "部门",
                "项目",
                "Provider",
                "账号ID",
                "实际模型",
                "结果",
                "计入请求",
                "计入用量",
                "总Token",
                "输入Token",
                "输出Token",
                "缓存Token（含于输入）",
                "用量金额",
                "币种",
                "费用来源",
                "费用池",
                "说明",
            ],
        )?;
        s.set_column_width(1, 32).map_err(error)?;
        for (i, r) in report.records.iter().enumerate() {
            let row = i as u32 + 6;
            for (c, v) in [
                (0, r.id.as_str()),
                (1, &r.time_display),
                (2, &r.key_id),
                (3, &r.person),
                (4, &r.department),
                (5, &r.project),
                (6, &r.provider),
                (7, r.account_id.as_deref().unwrap_or("")),
                (8, &r.model),
                (9, &r.outcome),
            ] {
                text(s, row, c, v)?;
            }
            text(s, row, 10, if r.request_counted { "是" } else { "否" })?;
            text(s, row, 11, if r.usage_counted { "是" } else { "否" })?;
            for (c, v) in [
                (12, &r.total_tokens),
                (13, &r.input_tokens),
                (14, &r.output_tokens),
                (15, &r.cached_tokens),
            ] {
                if let Some(v) = v {
                    number(s, row, c, v, "#,##0")?;
                }
            }
            if let Some(v) = &r.cost {
                number(s, row, 16, v, "0.000000")?;
            }
            text(s, row, 17, r.cost_currency.as_deref().unwrap_or(""))?;
            text(s, row, 18, r.cost_source.as_deref().unwrap_or("未知"))?;
            text(s, row, 19, r.pool_id.as_deref().unwrap_or("未关联"))?;
        }
        s.autofilter(5, 0, report.records.len() as u32 + 5, 20)
            .map_err(error)?;
    }
    {
        let s = init(&mut book, "实际支出", &note, 13)?;
        header(
            s,
            5,
            &[
                "付款ID",
                "付款日期",
                "供应商",
                "类别",
                "费用池",
                "币种",
                "实付金额",
                "整期分摊基数",
                "归属开始",
                "归属结束（不含）",
                "凭证编号",
                "原付款ID",
                "备注",
            ],
        )?;
        let mut row = 6;
        for p in &settings.payments {
            if !report.payments.iter().any(|v| v.id == p.id)
                && !report.expenses.iter().any(|v| v.payment_id == p.id)
            {
                continue;
            }
            for (c, v) in [
                (0, p.id.as_str()),
                (1, &p.paid_date),
                (2, &p.supplier),
                (3, &format!("{:?}", p.kind)),
                (4, &p.pool_id),
                (5, &p.currency),
                (8, &p.start_date),
                (9, &p.end_date),
                (10, &p.voucher),
                (11, p.original_payment_id.as_deref().unwrap_or("")),
                (12, &p.notes),
            ] {
                text(s, row, c, v)?;
            }
            number(s, row, 6, &p.paid_amount, "#,##0.00;(#,##0.00)")?;
            number(s, row, 7, &p.allocation_base, "#,##0.00;(#,##0.00)")?;
            row += 1;
        }
        text(
            s,
            row + 2,
            0,
            "此表包括付款日在范围内或归属期间有分摊的凭证，现金流汇总按付款日筛选，期间分摊见费用分摊表",
        )?;
    }
    {
        let s = init(&mut book, "归属与规则", &note, 9)?;
        header(
            s,
            5,
            &[
                "归属ID",
                "Key ID",
                "员工",
                "部门",
                "项目",
                "生效开始",
                "生效结束（不含）",
            ],
        )?;
        let mut row = 6;
        for a in &settings.assignments {
            for (c, v) in [
                (0, a.id.as_str()),
                (1, &a.key_id),
                (2, &a.person),
                (3, &a.department),
                (4, &a.project),
                (5, &a.start_date),
                (6, a.end_date.as_deref().unwrap_or("无截止日期")),
            ] {
                text(s, row, c, v)?;
            }
            row += 1;
        }
        row += 2;
        header(
            s,
            row,
            &[
                "费用池ID",
                "名称",
                "依据",
                "账号ID列表",
                "Key ID列表",
                "Provider",
            ],
        )?;
        row += 1;
        for p in &settings.pools {
            for (c, v) in [
                (0, p.id.clone()),
                (1, p.name.clone()),
                (2, format!("{:?}", p.basis)),
                (3, p.account_ids.join(",")),
                (4, p.key_ids.join(",")),
                (5, p.provider.clone().unwrap_or_default()),
            ] {
                text(s, row, c, &v)?;
            }
            row += 1;
        }
        row += 2;
        header(s, row, &["人工费用池", "员工", "部门", "项目", "比例%"])?;
        row += 1;
        for p in &settings.pools {
            for share in &p.shares {
                for (c, v) in [
                    (0, p.id.as_str()),
                    (1, &share.person),
                    (2, &share.department),
                    (3, &share.project),
                ] {
                    text(s, row, c, v)?;
                }
                number(s, row, 4, &share.percent, "0.0000")?;
                row += 1;
            }
        }
        row += 2;
        header(
            s,
            row,
            &[
                "预算ID",
                "维度",
                "归属",
                "币种",
                "整期预算",
                "开始",
                "结束（不含）",
            ],
        )?;
        row += 1;
        for b in &settings.budgets {
            for (c, v) in [
                (0, b.id.as_str()),
                (1, &format!("{:?}", b.scope)),
                (2, &b.name),
                (3, &b.currency),
                (5, &b.start_date),
                (6, &b.end_date),
            ] {
                text(s, row, c, v)?;
            }
            number(s, row, 4, &b.amount, "#,##0.00")?;
            row += 1;
        }
        text(
            s,
            row + 2,
            0,
            "金额与Token未知时留空，输入已含缓存，不重复加总，超过15位的精确数值以文本保存",
        )?;
    }
    book.save_to_buffer().map_err(error)
}
