import { ref } from 'vue'

export type Language = 'zh-CN' | 'en'
function initial(): Language {
  try {
    const saved = localStorage.getItem('enterprise-ledger-language')
    if (saved === 'zh-CN' || saved === 'en')
      return saved
  }
  catch { /* Host sandbox may disable storage. */ }
  return navigator.language.toLowerCase().startsWith('zh') ? 'zh-CN' : 'en'
}
export const language = ref<Language>(initial())
export const tr = (zh: string, en: string) => language.value === 'zh-CN' ? zh : en
export function setLanguage(value: Language) {
  language.value = value
  document.documentElement.lang = value
  document.title = tr('企业用量与费用台账', 'Enterprise Ledger')
  try {
    localStorage.setItem('enterprise-ledger-language', value)
  }
  catch { /* The selector also works without storage in opaque-origin iframes. */ }
}
export const displayName = (value: string) => value === '未归属' ? tr(value, 'Unassigned') : value
const messages: Record<string, string> = {
  '请求明细统计仅覆盖宿主仍保留的原生请求，不包含无明细的外部聚合贡献': 'Only retained native requests are included. External aggregates without request records are excluded.',
  '用量金额与实际付款分别列示，成功交付用量不等于 Key 结算账本或供应商结算单': 'Usage costs and actual payments are shown separately. Delivered usage is not a Key billing ledger or supplier invoice.',
  '存在未归属 Key，请补齐请求当时的组织归属': 'Some Keys are unassigned. Add the ownership that applied when the requests started.',
  '人工分摊比例合计须等于100': 'Manual allocation percentages must total 100.',
  '池内存在未知 Token': 'This pool contains requests with unknown tokens.',
  '池内存在未知费用': 'This pool contains requests with unknown costs.',
  '金额权重包含多种币种，请拆分费用池': 'Cost weights include different currencies. Split this cost pool.',
  '没有合格用量或总权重为零': 'No eligible usage, or total weight is zero.',
  '日期须使用 YYYY-MM-DD': 'Use YYYY-MM-DD dates.',
  '日期无效': 'The date is invalid.',
  '查询范围须为 1 至 366 天': 'Choose a date range between 1 and 366 days.',
  '报表时区无效': 'Enter a valid reporting time zone.',
  '此时区的日历边界不唯一，请调整日期': 'This time zone has an ambiguous day boundary. Choose another date.',
  '金额或比例须为十进制数字': 'Amounts and percentages must be decimal numbers.',
  '币种支持 CNY、USD、EUR、GBP、HKD、JPY': 'Supported currencies: CNY, USD, EUR, GBP, HKD and JPY.',
  '归属开始须早于结束，结束日期不包含在期间内': 'The period start must precede its exclusive end date.',
  '配置超出容量限制': 'The configuration exceeds the supported record limits.',
  '归属有效期无效': 'The ownership effective period is invalid.',
  'Provider 无效': 'The Provider value is invalid.',
  '人工分摊行数过多': 'There are too many manual allocation rows.',
  '人工分摊比例须大于零且不超过100': 'Each manual percentage must be greater than zero and at most 100.',
  '非人工费用池不能配置人工比例': 'Manual shares are only valid for manual cost pools.',
  '凭证编号或备注过长': 'The voucher number or notes are too long.',
  '金额超出上限或币种精度': 'The amount exceeds the limit or the currency precision.',
  '充值的本期分摊基数必须为零，消费结算另行登记': 'Recharge allocation bases must be zero. Record consumption settlement separately.',
  '退款须关联原付款': 'A refund must reference its original payment.',
  '退款的原付款、币种、费用池或正负号无效': 'Check the original payment, currency, pool and negative amounts for this refund.',
  '负金额及原付款关联仅用于退款': 'Negative amounts and original-payment references are only allowed for refunds.',
  '预算须为非负金额并符合币种精度': 'Budgets must be nonnegative and respect currency precision.',
  '配置超过512 KiB，请减少历史付款或归属条目': 'The configuration exceeds 512 KiB. Reduce historical payment or ownership entries.',
  '台账解压后超过8 MiB，请缩小月结范围或另行归档': 'The decoded ledger exceeds 8 MiB. Shorten the reporting period or archive older exports separately.',
  '台账解压后超过8 MiB': 'The decoded ledger exceeds 8 MiB.',
  '已保存台账的格式不受支持': 'The saved ledger format is not supported.',
  'Key 数量超过10000，请使用手工输入的稳定 Key ID': 'There are more than 10,000 Keys. Enter the stable Key ID manually.',
  'Key 游标未前进': 'The host Key cursor did not advance.',
  '扫描的数据锚点变化，报表未生成': 'The data anchor changed during the scan. No report was generated.',
  '扫描返回非明细数据': 'The scan returned data other than request records.',
  '扫描没有提供数据锚点': 'The scan did not provide a data anchor.',
  '报表计算未完成': 'The report calculation did not finish.',
  '请求超过50000条，请缩小查询范围': 'The query exceeds 50,000 records. Choose a shorter date range.',
  '扫描返回重复请求，报表未生成': 'The scan returned duplicate requests. No report was generated.',
  '扫描返回范围外记录': 'The scan returned records outside the selected period.',
  '导入超过2000行': 'An import cannot exceed 2,000 rows.',
  'Excel 生成任务未完成': 'Excel generation did not finish.',
  'Excel 超过16 MiB，请缩小查询范围': 'The Excel file exceeds 16 MiB. Choose a shorter date range.',
  '时区无效': 'Enter a valid IANA time zone.',
  '记录版本冲突，请重新加载后再保存': 'The record changed. Reload the configuration before saving.',
  '记录版本冲突，请重新加载后再导入': 'The record changed. Reload the configuration before importing.',
  '记录版本冲突，请重新加载后再移除': 'The record changed. Reload the configuration before removing an archive.',
  '配置已变化，请重新查询后月结': 'The configuration changed. Run the report again before closing the period.',
  '同一期间再次月结须填写调整说明': 'Add an adjustment reason before closing the same period again.',
  '月结名称或调整说明无效': 'Enter a valid archive name and adjustment reason.',
  '没有可导入的付款': 'The CSV contains no payment records.',
  '供应商及凭证编号重复，请检查是否重复导入': 'The supplier and voucher number already exist. Check for a duplicate import.',
  '报表已过期，请重新查询或打开月结': 'The report expired. Run the query again or reopen an archive.',
  '文件已下载或过期，请重新导出': 'The file was downloaded or expired. Generate the export again.',
  '扫描未完整结束，不能生成报表': 'The scan did not finish. No partial report was generated.',
  '请求超过50000条，请缩小日期范围': 'The query exceeds 50,000 records. Choose a shorter date range.',
  '待下载文件较多，请先下载已有文件或等待过期': 'Too many exports are waiting. Download an existing file or wait for it to expire.',
  '另一个报表任务正在运行，请稍后重试': 'Another report is running. Try again shortly.',
  '另一个报表任务正在运行': 'Another report is running.',
  '月结版本不存在': 'This archive no longer exists.',
  '压缩台账超过宿主单条256 KiB限制，请缩小月结范围或保留 Excel 归档': 'The compressed ledger exceeds the host 256 KiB limit. Shorten the period or keep older archives as Excel files.',
  '月结最多保存24个版本，请另行保存导出文件': 'The ledger holds up to 24 archive versions. Keep older exports separately.',
}
const fieldNames: Record<string, string> = { 费用池名称: 'Cost pool name', 员工: 'Employee', 部门: 'Department', 项目: 'Project', 供应商: 'Supplier', 归属ID: 'Assignment ID', 付款ID: 'Payment ID', 费用池ID: 'Cost pool ID', 预算ID: 'Budget ID', 名称: 'Name', 账号ID: 'Account ID', 预算归属: 'Budget owner' }
export function message(value: string): string {
  if (language.value === 'zh-CN')
    return value
  if (messages[value])
    return messages[value]
  const missing = value.match(/^(\d+) 条完整交付请求的费用未知$/)
  if (missing)
    return `${missing[1]} delivered requests have unknown costs.`
  const required = value.match(/^(.+)不能为空$/)
  if (required)
    return `${fieldNames[required[1]!] ?? required[1]} is required.`
  const length = value.match(/^(.+)不能为空或超过 (\d+) 字$/)
  if (length)
    return `${fieldNames[length[1]!] ?? length[1]} is required and must be at most ${length[2]} characters.`
  const repeated = value.match(/^(.+)重复：(.+)$/)
  if (repeated)
    return `Duplicate ${fieldNames[repeated[1]!] ?? repeated[1]}: ${repeated[2]}`
  const overlap = value.match(/^Key (.+) 的归属有效期重叠$/)
  if (overlap)
    return `Ownership dates overlap for Key ${overlap[1]}.`
  const pool = value.match(/^付款 (.+) 的费用池不存在$/)
  if (pool)
    return `The cost pool for payment ${pool[1]} does not exist.`
  if (value.startsWith('CSV 行无法解析：'))
    return `CSV parse error: ${value.slice('CSV 行无法解析：'.length)}`
  const fault = value.match(/^宿主操作未完成（(.+)）$/)
  if (fault)
    return `The host operation did not finish (${fault[1]}).`
  const duplicate = value.match(/^付款ID (.+) 已存在，整批未导入$/)
  if (duplicate)
    return `Payment ID ${duplicate[1]} already exists. No rows were imported.`
  return value
}
