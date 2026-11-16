<script setup lang="ts">
import type { Collection, Group, Query, Report, Row, Settings, State } from './api'
import { defineTableColumns, ZButton, ZCard, ZConfigProvider, ZDialog, ZFormItem, ZInput, ZTable, ZTextarea } from '@codex-proxy/ui'
import { en, zhHans } from '@codex-proxy/ui/locale'
import { computed, onMounted, ref, toRaw } from 'vue'
import { count, host, money, request } from './api'
import AppIcon from './components/AppIcon.vue'
import EntityEditor from './components/EntityEditor.vue'
import GroupTable from './components/GroupTable.vue'
import { fields, titles } from './fields'
import { displayName, language, message, setLanguage, tr } from './i18n'

type Tab = 'report' | Collection | 'closed' | 'settings' | 'guide'
type Dimension = 'departments' | 'people' | 'projects' | 'models'
const tab = ref<Tab>('report')
const dimension = ref<Dimension>('departments')
const tabs = computed<{ key: Tab, label: string, icon: string }[]>(() => [
  { key: 'report', label: tr('统计总览', 'Overview'), icon: 'chart' },
  { key: 'assignments', label: tr('人员归属', 'People & ownership'), icon: 'users' },
  { key: 'pools', label: tr('费用分摊', 'Cost pools'), icon: 'pools' },
  { key: 'payments', label: tr('实际支出', 'Payments'), icon: 'payments' },
  { key: 'budgets', label: tr('预算', 'Budgets'), icon: 'budget' },
  { key: 'closed', label: tr('月结归档', 'Period archives'), icon: 'archive' },
  { key: 'settings', label: tr('时区与导入', 'Settings & import'), icon: 'settings' },
  { key: 'guide', label: tr('使用指南', 'Getting started'), icon: 'book' },
])
const descriptions = computed<Record<Tab, string>>(() => ({
  report: tr('把每一次调用，变成看得清的账。', 'Every request. Every cost. Clearly accounted for.'),
  assignments: tr('按请求发生时的有效归属，连接员工、部门与项目。', 'Connect each Key to the people, departments and projects it served.'),
  pools: tr('为共享账号建立清晰、可核对的分摊规则。', 'Define how shared account costs are allocated and reconciled.'),
  payments: tr('记录真正发生的付款，保留凭证与归属期间。', 'Record actual cash movements, vouchers and expense periods.'),
  budgets: tr('在同一期间、同一币种下，查看支出与预算。', 'Compare allocated spend and budgets for the same period and currency.'),
  closed: tr('保存当时的数据与规则，让历史有据可查。', 'Freeze the data and rules behind each reporting period.'),
  settings: tr('设置报表时区，批量导入付款与管理配置。', 'Set the reporting time zone, import payments and manage configuration.'),
  guide: tr('从安装完成，到导出第一份企业台账。', 'From a newly installed plugin to your first company ledger.'),
}))
const dimensions = computed<{ key: Dimension, label: string }[]>(() => [
  { key: 'departments', label: tr('部门统计', 'Departments') },
  { key: 'people', label: tr('人员统计', 'People') },
  { key: 'projects', label: tr('项目统计', 'Projects') },
  { key: 'models', label: tr('模型统计', 'Models') },
])
const state = ref<State | null>(null)
const report = ref<Report | null>(null)
const busy = ref(false)
const error = ref('')
const notice = ref<[string, string] | null>(null)
const closedView = ref(false)
const fileId = ref('')
const downloadUrl = ref('')
const csv = ref('')
const zone = ref('Asia/Shanghai')
const label = ref('')
const reason = ref('')
const deleteDialog = ref(false)
const deleteId = ref('')
const keys = ref<{ id: string, name: string, enabled: boolean }[]>([])
const now = new Date()
const localDate = (d: Date) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
const query = ref<Query>({ startDate: localDate(new Date(now.getFullYear(), now.getMonth(), 1)), endDate: localDate(now) })
const collection = computed(() => ['assignments', 'pools', 'payments', 'budgets'].includes(tab.value) ? tab.value as Collection : null)
const jsonSettings = ref('')
const steps = computed<{ tab: Tab, title: string, detail: string, done: boolean }[]>(() => [
  { tab: 'assignments', title: tr('关联人员', 'Assign people'), detail: tr('读取 Key ID，登记员工、部门、项目及有效期。共享 Key 无法识别个人时保留未归属。', 'Read the stable Key IDs and assign employees, departments, projects and effective dates. Leave shared, unidentified Keys unassigned.'), done: Boolean(state.value?.settings.assignments.length) },
  { tab: 'pools', title: tr('建立费用池', 'Create cost pools'), detail: tr('按账号、Key 或 Provider 选择范围，再选择 Token、金额或人工比例。', 'Match accounts, Keys or a Provider, then choose token, cost or manual allocation weights.'), done: Boolean(state.value?.settings.pools.length) },
  { tab: 'payments', title: tr('登记实际付款', 'Record payments'), detail: tr('选择费用池，填入币种、实付金额、整期分摊基数和归属期间。充值基数为零。', 'Choose a pool, currency, paid amount, full-period allocation base and dates. A recharge has a zero allocation base.'), done: Boolean(state.value?.settings.payments.length) },
  { tab: 'budgets', title: tr('设置预算（可选）', 'Add budgets (optional)'), detail: tr('按部门或项目登记同币种预算，名称需与人员归属完全一致。', 'Add a department or project budget in the same currency. Names must match the ownership records exactly.'), done: Boolean(state.value?.settings.budgets.length) },
  { tab: 'report', title: tr('查询、导出与月结', 'Query, export & close'), detail: tr('选择日期查询，核对未归属和未分摊；生成 Excel、取得链接并下载，最后保存月结。', 'Query a period and review unassigned usage and unallocated costs. Generate Excel, get a download link, then save a period archive.'), done: Boolean(state.value?.closedReports.length) },
])
const activeTitle = computed(() => tabs.value.find(item => item.key === tab.value)?.label)
const activeGroupTitle = computed(() => dimensions.value.find(item => item.key === dimension.value)?.label ?? '')
function tell(zh: string, english: string) {
  notice.value = [zh, english]
}

async function action(run: () => Promise<void>) {
  if (busy.value)
    return
  busy.value = true
  error.value = ''
  notice.value = null
  try {
    await run()
  }
  catch (e) { error.value = e instanceof Error ? e.message : tr('操作未完成', 'The operation did not complete.') }
  finally { busy.value = false }
}
async function load() {
  state.value = await request<State>('api/state')
  zone.value = state.value.settings.timeZone
  jsonSettings.value = JSON.stringify(state.value.settings, null, 2)
}
async function saveSettings(settings: Settings) {
  await request('api/state', { expectedVersion: state.value?.version ?? null, settings })
  await load()
  report.value = null
  fileId.value = ''
  downloadUrl.value = ''
  tell('已保存，重新查询可查看更新后的统计', 'Saved. Run the report again to see the updated totals.')
}
function saveRow(row: Row) {
  void action(async () => {
    if (!state.value || !collection.value)
      return
    const settings = structuredClone(toRaw(state.value.settings))
    const rows = settings[collection.value]
    const i = rows.findIndex(v => v.id === row.id)
    if (i < 0)
      rows.push(row)
    else rows[i] = row
    await saveSettings(settings)
  })
}
function removeRow(id: string) {
  void action(async () => {
    if (!state.value || !collection.value)
      return
    const settings = structuredClone(toRaw(state.value.settings))
    settings[collection.value] = settings[collection.value].filter(v => v.id !== id)
    await saveSettings(settings)
  })
}
function runReport() {
  void action(async () => {
    report.value = await request<Report>('api/report', query.value)
    closedView.value = false
    fileId.value = ''
    downloadUrl.value = ''
    label.value = `${query.value.startDate} / ${query.value.endDate}`
  })
}
function prepareExport() {
  void action(async () => {
    if (!report.value)
      return
    const file = await request<{ fileId: string }>('api/export', { reportId: report.value.id })
    fileId.value = file.fileId
    downloadUrl.value = ''
    tell('Excel 已生成，点击取得下载链接', 'Excel is ready. Click Get download link to continue.')
  })
}
function createLink() {
  void action(async () => {
    const ticket = await host().callbackTicket({ path: 'download', ttlSeconds: 120 })
    const url = new URL(ticket.callbackUrl)
    url.searchParams.set('id', fileId.value)
    downloadUrl.value = url.toString()
    tell('链接两分钟内有效且仅可使用一次', 'This link is valid for two minutes and can be used once.')
  })
}
function openDownload() {
  if (!downloadUrl.value)
    return
  if (new URL(downloadUrl.value).protocol !== 'https:') {
    tell('当前部署使用 HTTP，请复制下方一次性地址并在新标签页打开', 'This host uses HTTP. Copy the one-time address below and open it in a new tab.')
    return
  }
  void host().openExternal(downloadUrl.value).catch((e: unknown) => {
    error.value = e instanceof Error ? e.message : tr('无法打开下载地址', 'The download address could not be opened.')
  })
}
function closeReport() {
  void action(async () => {
    if (!report.value)
      return
    await request('api/close', { reportId: report.value.id, label: label.value, reason: reason.value })
    await load()
    closedView.value = true
    tell('月结快照已保存，配置变化不会改写此版本', 'Archive saved. Later configuration changes will not rewrite this version.')
  })
}
function openClosed(id: string) {
  void action(async () => {
    report.value = await request<Report>('api/open', { reportId: id })
    closedView.value = true
    tab.value = 'report'
    fileId.value = ''
    downloadUrl.value = ''
  })
}
function deleteClosed() {
  void action(async () => {
    await request('api/delete-close', { reportId: deleteId.value, expectedVersion: state.value?.version ?? null })
    await load()
    deleteDialog.value = false
    tell('已移除此月结版本，付款和配置仍保留', 'Archive removed. Payments and the current configuration are preserved.')
  })
}
function importPayments() {
  void action(async () => {
    const result = await request<{ imported: number }>('api/import-payments', { expectedVersion: state.value?.version ?? null, csv: csv.value })
    await load()
    report.value = null
    fileId.value = ''
    downloadUrl.value = ''
    csv.value = ''
    tell(`已导入 ${result.imported} 条付款`, `Imported ${result.imported} payment records.`)
  })
}
async function readCsv(event: Event) {
  const file = (event.target as HTMLInputElement).files?.[0]
  if (!file)
    return
  if (file.size > 512 * 1024) {
    error.value = tr('CSV 超过512 KiB', 'The CSV exceeds 512 KiB.')
    return
  }
  try {
    csv.value = await file.text()
  }
  catch { error.value = tr('无法读取文件', 'The file could not be read.') }
}
function distribution(groups: Group[]) {
  const sorted = groups.filter(g => g.metrics.requests > 0).sort((a, b) => b.metrics.requests - a.metrics.requests || a.name.localeCompare(b.name))
  const total = sorted.reduce((sum, g) => sum + g.metrics.requests, 0)
  const rows = sorted.slice(0, 4).map(g => ({ name: displayName(g.name), value: g.metrics.requests }))
  if (sorted.length > 4)
    rows.push({ name: tr('其他', 'Other'), value: sorted.slice(4).reduce((sum, g) => sum + g.metrics.requests, 0) })
  return rows.map((row, index) => ({ ...row, id: index, percent: total ? row.value / total * 100 : 0 }))
}
const charts = computed(() => report.value
  ? [
      { title: tr('部门用量分布', 'Requests by department'), icon: 'users', rows: distribution(report.value.departments) },
      { title: tr('模型用量分布', 'Requests by model'), icon: 'pools', rows: distribution(report.value.models) },
    ]
  : [])
const paymentTemplate = 'id,paidDate,supplier,voucher,kind,poolId,currency,paidAmount,allocationBase,startDate,endDate,originalPaymentId,notes'
const expenseRows = computed(() => report.value?.expenses.map((e, i) => ({ id: `${e.paymentId}-${i}`, payment: e.paymentId, pool: e.poolId, base: `${e.base} ${e.currency}`, allocated: `${e.allocated} ${e.currency}`, pending: `${e.unallocated} ${e.currency}`, reason: e.reason ? message(e.reason) : tr('已分摊', 'Allocated') })) ?? [])
const expenseColumns = computed(() => defineTableColumns<typeof expenseRows.value[number]>([{ key: 'payment', label: tr('付款ID', 'Payment ID'), minWidth: 150 }, { key: 'pool', label: tr('费用池', 'Cost pool'), minWidth: 150 }, { key: 'base', label: tr('期间基数', 'Period base'), minWidth: 150 }, { key: 'allocated', label: tr('已分摊', 'Allocated'), minWidth: 150 }, { key: 'pending', label: tr('未分摊', 'Unallocated'), minWidth: 150 }, { key: 'reason', label: tr('处理状态', 'Status'), minWidth: 240 }]))
const recordRows = computed(() => report.value?.records.map(r => ({ id: String(r.id), time: String(r.timeDisplay), person: displayName(String(r.person)), department: displayName(String(r.department)), model: String(r.model), tokens: r.totalTokens === null ? tr('未知', 'Unknown') : count(String(r.totalTokens)), cost: r.cost === null ? tr('未知', 'Unknown') : `${String(r.cost)} ${String(r.costCurrency)}` })) ?? [])
const recordColumns = computed(() => defineTableColumns<typeof recordRows.value[number]>([{ key: 'id', label: tr('请求ID', 'Request ID'), minWidth: 180 }, { key: 'time', label: tr('时间', 'Time'), minWidth: 240 }, { key: 'person', label: tr('员工', 'Employee'), minWidth: 120 }, { key: 'department', label: tr('部门', 'Department'), minWidth: 120 }, { key: 'model', label: tr('模型', 'Model'), minWidth: 160 }, { key: 'tokens', label: tr('原始总Token', 'Raw tokens'), minWidth: 140 }, { key: 'cost', label: tr('原始用量金额', 'Raw usage cost'), minWidth: 170 }]))
onMounted(() => {
  setLanguage(language.value)
  void action(load)
})
</script>

<template>
  <ZConfigProvider :locale="language === 'zh-CN' ? zhHans : en">
    <div class="ledger-shell">
      <aside class="ledger-sidebar">
        <div class="ledger-brand">
          <span class="brand-mark"><AppIcon :size="28" /></span><div><strong>Enterprise Ledger</strong><small>BY BELOFR</small></div>
        </div>
        <div class="nav-caption">
          {{ tr('企业用量与费用台账', 'USAGE & COST WORKSPACE') }}
        </div>
        <nav class="ledger-nav" :aria-label="tr('台账功能', 'Ledger navigation')">
          <button v-for="item in tabs" :key="item.key" type="button" :class="{ active: tab === item.key }" :aria-current="tab === item.key ? 'page' : undefined" @click="tab = item.key">
            <AppIcon :name="item.icon" :size="18" /><span>{{ item.label }}</span><span v-if="tab === item.key" class="nav-dot" />
          </button>
        </nav>
        <div class="sidebar-note">
          <AppIcon name="shield" :size="22" /><p>{{ tr('有据可查，按期归档。', 'Traceable records. Preserved periods.') }}</p><span>v0.2.0 · Apache-2.0</span>
        </div>
      </aside>
      <main class="ledger-content">
        <header class="ledger-header">
          <div>
            <div class="eyebrow">
              {{ tr('企业台账', 'ENTERPRISE LEDGER') }} <span>/</span> {{ tr('工作空间', 'WORKSPACE') }}
            </div><h1>{{ activeTitle }}</h1><p>{{ descriptions[tab] }}</p>
          </div>
          <div class="header-actions">
            <div class="language-switch" role="group" aria-label="Language / 语言">
              <AppIcon name="globe" :size="16" /><button type="button" :aria-pressed="language === 'zh-CN'" @click="setLanguage('zh-CN')">
                中文
              </button><button type="button" :aria-pressed="language === 'en'" @click="setLanguage('en')">
                EN
              </button>
            </div><ZButton :loading="busy" @click="action(load)">
              <AppIcon name="refresh" :size="16" />{{ tr('重新加载配置', 'Reload settings') }}
            </ZButton>
          </div>
        </header>
        <div v-if="error" class="ledger-alert error" role="alert">
          {{ message(error) }}
        </div>
        <div v-if="notice" class="ledger-alert" role="status">
          <AppIcon name="check" :size="18" />{{ tr(...notice) }}
        </div>

        <template v-if="tab === 'report'">
          <section class="query-bar" :aria-label="tr('报表查询', 'Report query')">
            <div class="query-dates">
              <ZFormItem :label="tr('开始日期', 'Start date')">
                <ZInput v-model="query.startDate" type="date" :aria-label="tr('开始日期', 'Start date')" />
              </ZFormItem><span class="date-separator">—</span><ZFormItem :label="tr('结束日期（含）', 'End date (inclusive)')">
                <ZInput v-model="query.endDate" type="date" :aria-label="tr('结束日期（含）', 'End date (inclusive)')" />
              </ZFormItem>
            </div>
            <div class="query-actions">
              <ZButton type="primary" :loading="busy" @click="runReport">
                {{ tr('查询统计', 'Run report') }}<AppIcon name="arrow" :size="16" />
              </ZButton><ZButton :disabled="busy || !report" @click="prepareExport">
                <AppIcon name="download" :size="16" />{{ tr('生成 Excel', 'Generate Excel') }}
              </ZButton><ZButton v-if="fileId" :disabled="busy" @click="createLink">
                {{ tr('取得下载链接', 'Get download link') }}
              </ZButton><ZButton v-if="downloadUrl" :disabled="busy" @click="openDownload">
                {{ tr('打开下载', 'Open download') }}
              </ZButton>
            </div>
            <div v-if="downloadUrl" class="download-field">
              <ZInput :model-value="downloadUrl" readonly :aria-label="tr('一次性下载地址', 'One-time download address')" @focus="($event.target as HTMLInputElement).select()" /><small>{{ tr('HTTP 部署请复制链接，在新标签页打开。', 'For HTTP hosts, copy the address and open it in a new tab.') }}</small>
            </div>
          </section>
          <template v-if="report">
            <div class="snapshot-line">
              <span class="live-dot" /><span>{{ closedView ? tr('已保存的月结快照', 'Saved period snapshot') : tr('当前请求快照', 'Current request snapshot') }}</span><span>{{ report.query.startDate }} — {{ report.query.endDate }}</span><span class="timezone"><AppIcon name="clock" :size="14" />{{ report.timeZone }}</span>
            </div>
            <div class="metric-grid">
              <section class="metric-card featured">
                <div class="metric-label">
                  {{ tr('调用数', 'Requests') }}<AppIcon name="chart" :size="19" />
                </div><strong>{{ count(report.metrics.requests) }}</strong><small>{{ tr('完整交付', 'Delivered') }} {{ count(report.metrics.completed) }}</small><div class="metric-rule" />
              </section>
              <section class="metric-card">
                <div class="metric-label">
                  {{ tr('已知总 Token', 'Known tokens') }}<AppIcon name="pools" :size="19" />
                </div><strong>{{ count(report.metrics.totalTokens) }}</strong><small>{{ tr(`${report.metrics.unknownTokens} 条 Token 未知`, `${report.metrics.unknownTokens} records with unknown tokens`) }}</small>
              </section>
              <section class="metric-card">
                <div class="metric-label">
                  {{ tr('实际付款净额', 'Net cash paid') }}<AppIcon name="payments" :size="19" />
                </div><strong class="money-value">{{ money(report.cash) }}</strong><small>{{ tr('按付款日期 · 各币种分别统计', 'By payment date · currencies kept separate') }}</small>
              </section>
              <section class="metric-card">
                <div class="metric-label">
                  {{ tr('未分摊金额', 'Unallocated') }}<AppIcon name="budget" :size="19" />
                </div><strong class="money-value">{{ money(report.unallocated) }}</strong><small>{{ tr(`费用未知 ${report.metrics.unknownCosts} 条`, `${report.metrics.unknownCosts} records with unknown costs`) }}</small>
              </section>
            </div>
            <div class="chart-grid">
              <section v-for="chart in charts" :key="chart.icon" class="ledger-panel chart-panel">
                <div class="section-heading">
                  <h2><AppIcon :name="chart.icon" :size="18" />{{ chart.title }}</h2><span class="unit-chip">{{ tr('调用数占比', 'Share of requests') }}</span>
                </div><div v-if="!chart.rows.length" class="chart-empty">
                  {{ tr('当前范围暂无数据', 'No data in this period.') }}
                </div><ol v-else class="distribution-list">
                  <li v-for="row in chart.rows" :key="row.id">
                    <div><span>{{ row.name }}</span><strong>{{ count(row.value) }} <small>{{ row.percent.toFixed(1) }}%</small></strong></div><div class="bar-track" role="img" :aria-label="`${row.name}: ${count(row.value)}, ${row.percent.toFixed(1)}%`">
                      <span :style="{ width: `${row.percent}%` }" />
                    </div>
                  </li>
                </ol>
              </section>
            </div>
            <section class="ledger-panel">
              <div class="section-heading">
                <h2>{{ tr('归属明细', 'Ownership breakdown') }}</h2><span class="unit-chip">{{ tr('同一请求快照', 'One consistent snapshot') }}</span>
              </div><div class="dimension-tabs" role="group" :aria-label="tr('统计维度', 'Reporting dimension')">
                <button v-for="item in dimensions" :key="item.key" type="button" :aria-pressed="dimension === item.key" @click="dimension = item.key">
                  {{ item.label }}
                </button>
              </div><GroupTable :title="activeGroupTitle" :groups="report[dimension]" />
            </section>
            <ZCard :title="tr('支出分摊核对', 'Allocation reconciliation')" padding="none">
              <ZTable :columns="expenseColumns" :data="expenseRows" row-key="id" :empty-text="tr('当前范围没有分摊基数', 'No allocation base in this period.')" />
            </ZCard>
            <details class="ledger-panel disclosure">
              <summary>{{ tr('费用口径与数据范围', 'Cost definitions & data coverage') }}</summary><div class="cost-facts">
                <div v-for="c in report.metrics.usageCosts" :key="c.currency">
                  {{ c.currency }} · {{ tr('上游报告', 'Provider reported') }} {{ c.providerReported }} / {{ tr('本地估算', 'Locally estimated') }} {{ c.calculated }}
                </div><div>{{ tr('期间分摊基数', 'Period allocation base') }}: {{ money(report.allocationBase) }}</div><div>{{ tr('已分摊支出', 'Allocated spend') }}: {{ money(report.allocated) }}</div>
              </div><ul>
                <li v-for="warning in report.warnings" :key="warning">
                  {{ message(warning) }}
                </li>
              </ul>
            </details>
            <details class="ledger-panel disclosure">
              <summary>{{ tr(`请求明细 · 共 ${report.recordCount} 条，预览前100条，Excel 导出全部`, `Request details · ${report.recordCount} total, 100-row preview, full Excel export`) }}</summary><ZTable :columns="recordColumns" :data="recordRows" row-key="id" />
            </details>
            <ZCard v-if="!closedView" :title="tr('保存月结版本', 'Close & archive this period')" :description="tr('保存本次报表与当时的规则，后续修改不会覆盖历史。', 'Save this report and its rules together. Later edits preserve this version.')">
              <div class="grid gap-3 sm:grid-cols-2">
                <ZFormItem :label="tr('名称', 'Name')">
                  <ZInput v-model="label" :aria-label="tr('月结名称', 'Archive name')" />
                </ZFormItem><ZFormItem :label="tr('调整说明（重复期间必填）', 'Adjustment reason (required for repeated periods)')">
                  <ZInput v-model="reason" :aria-label="tr('调整说明', 'Adjustment reason')" />
                </ZFormItem>
              </div><ZButton :disabled="busy" @click="closeReport">
                <AppIcon name="archive" :size="16" />{{ tr('保存月结快照', 'Save period snapshot') }}
              </ZButton>
            </ZCard>
          </template>
          <section v-else class="ledger-panel welcome-panel">
            <div class="welcome-art">
              <AppIcon :size="54" />
            </div><div><span class="eyebrow">{{ tr('从这里开始', 'A CLEARER VIEW STARTS HERE') }}</span><h2>{{ tr('第一份企业台账，从清晰的归属开始。', 'Give every request a place in your ledger.') }}</h2><p>{{ tr('先关联员工、建立费用池并登记付款，再选择日期查询。只有用量数据时，也可以直接生成统计。', 'Assign people, create cost pools and record payments, then query a period. You can also report usage before recording any payments.') }}</p></div><div class="setup-links">
              <button v-for="(step, i) in steps.slice(0, 3)" :key="step.tab" type="button" @click="tab = step.tab">
                <span>{{ step.done ? '✓' : `0${i + 1}` }}</span>{{ step.title }}<AppIcon name="arrow" :size="16" />
              </button>
            </div>
          </section>
        </template>

        <template v-if="collection && state">
          <div class="context-note">
            <AppIcon name="book" :size="18" /><p>{{ steps.find(step => step.tab === collection)?.detail }}</p><button type="button" @click="tab = 'guide'">
              {{ tr('查看流程', 'View workflow') }} →
            </button>
          </div>
          <EntityEditor :key="collection" :title="titles[collection]" :rows="state.settings[collection]" :fields="fields[collection]" :busy="busy" @save="saveRow" @remove="removeRow" @error="error = $event" />
          <ZCard v-if="collection === 'assignments'" :title="tr('Key ID 查询', 'Find stable Key IDs')" :description="tr('只读取 ID、名称和启用状态，不读取 Key 明文。', 'Reads IDs, names and enabled status. Key secrets are not read.')">
            <ZButton :loading="busy" @click="action(async () => { keys = await request<typeof keys>('api/keys') })">
              {{ tr('读取 Key 列表', 'Read Key list') }}
            </ZButton><div v-for="key in keys" :key="key.id" class="key-row">
              <span>{{ key.name }}</span><code>{{ key.id }}</code><span>{{ key.enabled ? tr('已启用', 'Enabled') : tr('已停用', 'Disabled') }}</span>
            </div>
          </ZCard>
        </template>
        <template v-if="tab === 'closed' && state">
          <ZCard :title="tr('已保存的月结版本', 'Saved period archives')">
            <div v-if="!state.closedReports.length" class="empty-state">
              <AppIcon name="archive" :size="36" /><p>{{ tr('暂无月结，在统计总览查询后保存快照', 'No archives yet. Run a report, then save a period snapshot.') }}</p>
            </div><div v-for="closed in state.closedReports" :key="closed.id" class="archive-row">
              <AppIcon name="archive" :size="24" /><div><strong>{{ closed.label }}</strong><small>{{ closed.query.startDate }} — {{ closed.query.endDate }} · {{ count(closed.records) }} {{ tr('条记录', 'records') }} · {{ closed.reason || tr('首次保存', 'First close') }}</small></div><ZButton :disabled="busy" @click="openClosed(closed.id)">
                {{ tr('查看与导出', 'View & export') }}
              </ZButton><ZButton :disabled="busy" variant="text" @click="deleteId = closed.id; deleteDialog = true">
                {{ tr('移除归档', 'Remove archive') }}
              </ZButton>
            </div>
          </ZCard>
        </template>
        <template v-if="tab === 'settings' && state">
          <ZCard :title="tr('报表时区', 'Report time zone')">
            <ZFormItem label="IANA time zone">
              <ZInput v-model="zone" :aria-label="tr('报表时区', 'Report time zone')" placeholder="Asia/Shanghai" />
            </ZFormItem><ZButton :disabled="busy" @click="action(() => saveSettings({ ...state!.settings, timeZone: zone }))">
              {{ tr('保存时区', 'Save time zone') }}
            </ZButton>
          </ZCard>
          <ZCard :title="tr('批量导入付款 CSV', 'Import payments from CSV')" :description="tr('UTF-8 编码，按付款ID去重，任何一行无效时整批不写入。先建立 CSV 中引用的费用池。', 'Use UTF-8. Duplicate payment IDs or invalid rows reject the entire batch. Create the referenced pools first.')">
            <ZInput :model-value="paymentTemplate" readonly :aria-label="tr('CSV 表头模板', 'CSV header template')" /><label for="payments-csv" class="file-picker">{{ tr('选择 CSV 文件', 'Choose a CSV file') }}<input id="payments-csv" type="file" accept=".csv,text/csv" @change="readCsv"></label><ZTextarea v-model="csv" :aria-label="tr('付款 CSV 内容', 'Payment CSV content')" :rows="6" /><ZButton class="mt-3" :disabled="busy || !csv.trim()" @click="importPayments">
              {{ tr('校验并导入', 'Validate & import') }}
            </ZButton>
          </ZCard>
          <ZCard :title="tr('配置 JSON', 'Configuration JSON')" :description="tr('包含人员归属、费用池、付款和预算，保存前执行相同校验，月结快照不会被覆盖。', 'Includes ownership, pools, payments and budgets. The same validation applies; existing archives are preserved.')">
            <ZTextarea v-model="jsonSettings" :aria-label="tr('配置 JSON', 'Configuration JSON')" :rows="12" /><ZButton class="mt-3" :disabled="busy" @click="action(() => saveSettings(JSON.parse(jsonSettings) as Settings))">
              {{ tr('校验并保存配置', 'Validate & save configuration') }}
            </ZButton>
          </ZCard>
        </template>
        <template v-if="tab === 'guide'">
          <section class="guide-hero ledger-panel">
            <span class="eyebrow">{{ tr('五步完成第一份报表', 'YOUR FIRST REPORT IN FIVE STEPS') }}</span><h2>{{ tr('从用量到费用，每一步都有依据。', 'A clear path from usage to costs.') }}</h2><p>{{ tr('页面右上角可随时切换中英文。切换语言不会更改人员名称、财务数据或未保存的表单。', 'Switch Chinese and English at the top right. Language changes preserve user names, financial data and unsaved form drafts.') }}</p>
          </section>
          <ol class="workflow-list">
            <li v-for="(step, i) in steps" :key="step.tab">
              <span class="step-number" :class="{ done: step.done }">{{ step.done ? '✓' : `0${i + 1}` }}</span><div><h3>{{ step.title }}</h3><p>{{ step.detail }}</p></div><ZButton @click="tab = step.tab">
                {{ tr('前往', 'Open') }}<AppIcon name="arrow" :size="16" />
              </ZButton>
            </li>
          </ol>
          <ZCard :title="tr('日期与下载，先看这两点', 'Two details that matter')">
            <p>{{ tr('查询结束日期包含当天；归属、付款期间和预算的结束日期不包含当天。例：10月整月查询 10-01 至 10-31，付款归属期间填 10-01 至 11-01。', 'Report end dates include that day. Ownership, payment periods and budgets use an exclusive end date: query Oct 1–31, but enter Oct 1–Nov 1 for an October payment period.') }}</p><p class="mt-3">
              {{ tr('生成 Excel → 取得下载链接 → 打开下载。HTTP 部署请复制链接到新标签页；两分钟内有效，仅可使用一次。Excel 保持七张中文工作表。', 'Generate Excel → Get download link → Open download. For HTTP hosts, copy the address into a new tab. Links last two minutes and work once. Excel keeps its seven Chinese worksheets.') }}
            </p>
          </ZCard>
          <ZCard :title="tr('导入与权限说明', 'Installation & access')">
            <p>{{ tr('在 Codex Proxy RS 的插件管理中上传 Linux x64 的 tar.gz 包，完成校验并确认信任后安装。源码 ZIP 不能作为插件包上传。', 'Upload the Linux x64 tar.gz package in Codex Proxy RS plugin management, validate it and confirm trust before installing. Source ZIP files are not plugin packages.') }}</p><p class="mt-3">
              {{ tr('当前通过宿主管理员入口使用。没有独立财务、业务或老板账号；付款来自人工或 CSV，不会自动读取银行流水。完整图文指南在 GitHub 仓库 README 中。', 'Access uses the host administrator account. Separate finance or manager accounts are not provided. Payments come from manual entries or CSV, not bank feeds. Full illustrated guides are in the GitHub README.') }}
            </p>
          </ZCard>
        </template>
        <footer class="ledger-footer">
          <span>BELOFR / Enterprise Ledger</span><span>{{ tr('用量 · 费用 · 归档', 'USAGE · COSTS · ARCHIVES') }}</span>
        </footer>
        <ZDialog v-model="deleteDialog" :title="tr('移除月结归档', 'Remove period archive')" :description="tr('请先导出并另行保存 Excel，移除后无法恢复该版本，付款和当前配置仍会保留', 'Export and keep the Excel file first. This archive cannot be restored after removal; payments and the current configuration remain.')">
          <template #footer>
            <ZButton :disabled="busy" @click="deleteDialog = false">
              {{ tr('取消', 'Cancel') }}
            </ZButton><ZButton type="danger" :loading="busy" @click="deleteClosed">
              {{ tr('已另行保存，确认移除', 'I have a copy. Remove archive') }}
            </ZButton>
          </template>
        </ZDialog>
      </main>
    </div>
  </ZConfigProvider>
</template>
