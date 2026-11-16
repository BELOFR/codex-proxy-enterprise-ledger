<script setup lang="ts">
import type { Collection, Query, Report, Row, Settings, State } from './api'
import { defineTableColumns, ZButton, ZCard, ZConfigProvider, ZDialog, ZFormItem, ZInput, ZTable, ZTextarea } from '@codex-proxy/ui'
import { zhHans } from '@codex-proxy/ui/locale'
import { computed, onMounted, ref, toRaw } from 'vue'
import { count, host, money, request } from './api'
import EntityEditor from './components/EntityEditor.vue'
import GroupTable from './components/GroupTable.vue'
import { fields, titles } from './fields'

type Tab = 'report' | Collection | 'closed' | 'settings'
const tab = ref<Tab>('report')
const tabs: { key: Tab, label: string }[] = [{ key: 'report', label: '统计总览' }, { key: 'assignments', label: '人员归属' }, { key: 'payments', label: '实际支出' }, { key: 'pools', label: '费用分摊' }, { key: 'budgets', label: '预算' }, { key: 'closed', label: '月结归档' }, { key: 'settings', label: '时区与导入' }]
const state = ref<State | null>(null)
const report = ref<Report | null>(null)
const busy = ref(false)
const error = ref('')
const notice = ref('')
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

async function action(run: () => Promise<void>) {
  if (busy.value)
    return
  busy.value = true
  error.value = ''
  notice.value = ''
  try {
    await run()
  }
  catch (e) { error.value = e instanceof Error ? e.message : '操作未完成' }
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
  notice.value = '已保存，重新查询可查看更新后的统计'
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
    label.value = `${query.value.startDate} 至 ${query.value.endDate}`
  })
}
function prepareExport() {
  void action(async () => {
    if (!report.value)
      return
    const file = await request<{ fileId: string }>('api/export', { reportId: report.value.id })
    fileId.value = file.fileId
    downloadUrl.value = ''
    notice.value = 'Excel 已生成，点击取得下载链接'
  })
}
function createLink() {
  void action(async () => {
    const ticket = await host().callbackTicket({ path: 'download', ttlSeconds: 120 })
    const url = new URL(ticket.callbackUrl)
    url.searchParams.set('id', fileId.value)
    downloadUrl.value = url.toString()
    notice.value = '链接两分钟内有效且仅可使用一次'
  })
}
// 调用宿主打开链接必须直接来自用户点击，不能在异步生成文件后自动跳转
function openDownload() {
  error.value = ''
  if (!downloadUrl.value)
    return
  if (new URL(downloadUrl.value).protocol !== 'https:') {
    notice.value = '当前部署使用 HTTP，请复制下方一次性地址并在新标签页打开'
    return
  }
  void host().openExternal(downloadUrl.value).catch((e: unknown) => {
    error.value = e instanceof Error ? e.message : '无法打开下载地址'
  })
}
function closeReport() {
  void action(async () => {
    if (!report.value)
      return
    await request('api/close', { reportId: report.value.id, label: label.value, reason: reason.value })
    await load()
    closedView.value = true
    notice.value = '月结快照已保存，配置变化不会改写此版本'
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
    notice.value = '已移除此月结版本，付款和配置仍保留'
  })
}
function importPayments() {
  void action(async () => {
    const result = await request<{ imported: number }>('api/import-payments', { expectedVersion: state.value?.version ?? null, csv: csv.value })
    await load()
    report.value = null
    csv.value = ''
    notice.value = `已导入 ${result.imported} 条付款`
  })
}
async function readCsv(event: Event) {
  const file = (event.target as HTMLInputElement).files?.[0]
  if (!file)
    return
  if (file.size > 512 * 1024) {
    error.value = 'CSV 超过512 KiB'
    return
  }
  csv.value = await file.text()
}
const paymentTemplate = 'id,paidDate,supplier,voucher,kind,poolId,currency,paidAmount,allocationBase,startDate,endDate,originalPaymentId,notes'
const expenseRows = computed(() => report.value?.expenses.map((e, i) => ({ id: `${e.paymentId}-${i}`, payment: e.paymentId, pool: e.poolId, base: `${e.base} ${e.currency}`, allocated: `${e.allocated} ${e.currency}`, pending: `${e.unallocated} ${e.currency}`, reason: e.reason ?? '已分摊' })) ?? [])
const expenseColumns = defineTableColumns<typeof expenseRows.value[number]>([{ key: 'payment', label: '付款ID', minWidth: 160 }, { key: 'pool', label: '费用池', minWidth: 160 }, { key: 'base', label: '期间基数', minWidth: 150 }, { key: 'allocated', label: '已分摊', minWidth: 150 }, { key: 'pending', label: '未分摊', minWidth: 150 }, { key: 'reason', label: '处理状态', minWidth: 240 }])
const recordRows = computed(() => report.value?.records.map(r => ({ id: String(r.id), time: String(r.timeDisplay), person: String(r.person), department: String(r.department), model: String(r.model), tokens: r.totalTokens === null ? '未知' : count(String(r.totalTokens)), cost: r.cost === null ? '未知' : `${String(r.cost)} ${String(r.costCurrency)}` })) ?? [])
const recordColumns = defineTableColumns<typeof recordRows.value[number]>([{ key: 'id', label: '请求ID', minWidth: 180 }, { key: 'time', label: '时间', minWidth: 240 }, { key: 'person', label: '员工', minWidth: 100 }, { key: 'department', label: '部门', minWidth: 100 }, { key: 'model', label: '模型', minWidth: 160 }, { key: 'tokens', label: '原始总Token', minWidth: 140 }, { key: 'cost', label: '原始用量金额', minWidth: 170 }])
onMounted(() => void action(load))
</script>

<template>
  <ZConfigProvider :locale="zhHans">
    <main class="flex min-w-0 flex-col gap-4 font-sans text-z-text">
      <nav class="flex flex-wrap gap-2" aria-label="台账功能">
        <ZButton v-for="item in tabs" :key="item.key" :type="tab === item.key ? 'primary' : 'default'" :variant="tab === item.key ? 'solid' : 'text'" @click="tab = item.key">
          {{ item.label }}
        </ZButton>
        <ZButton :loading="busy" @click="action(load)">
          重新加载配置
        </ZButton>
      </nav>
      <div v-if="error" class="rounded-z bg-z-error-container p-3 text-z-error-on-container" role="alert">
        {{ error }}
      </div>
      <div v-if="notice" class="rounded-z bg-z-info-container p-3 text-z-info-on-container" role="status">
        {{ notice }}
      </div>

      <template v-if="tab === 'report'">
        <ZCard>
          <div class="flex flex-wrap items-end gap-3">
            <ZFormItem label="开始日期">
              <ZInput v-model="query.startDate" type="date" aria-label="开始日期" />
            </ZFormItem>
            <ZFormItem label="结束日期（含）">
              <ZInput v-model="query.endDate" type="date" aria-label="结束日期（含）" />
            </ZFormItem>
            <ZButton type="primary" :loading="busy" @click="runReport">
              查询统计
            </ZButton>
            <ZButton :disabled="busy || !report" @click="prepareExport">
              生成 Excel
            </ZButton>
            <ZButton v-if="fileId" :disabled="busy" @click="createLink">
              取得下载链接
            </ZButton>
            <ZButton v-if="downloadUrl" :disabled="busy" @click="openDownload">
              打开下载
            </ZButton>
          </div>
          <ZInput v-if="downloadUrl" :model-value="downloadUrl" readonly aria-label="一次性下载地址" class="mt-3" @focus="($event.target as HTMLInputElement).select()" />
        </ZCard>
        <template v-if="report">
          <div class="text-z-sm text-z-text-secondary">
            {{ closedView ? '已保存的月结快照' : '当前请求快照' }} · {{ report.query.startDate }} 至 {{ report.query.endDate }} · {{ report.timeZone }}
          </div>
          <div class="grid grid-cols-2 gap-3 lg:grid-cols-4">
            <ZCard title="调用数">
              <strong class="text-xl tabular-nums">{{ count(report.metrics.requests) }}</strong><div class="text-z-xs text-z-text-secondary">
                完整交付 {{ count(report.metrics.completed) }}
              </div>
            </ZCard>
            <ZCard title="已知总 Token">
              <strong class="text-xl tabular-nums">{{ count(report.metrics.totalTokens) }}</strong><div class="text-z-xs text-z-text-secondary">
                {{ report.metrics.unknownTokens }} 条 Token 未知
              </div>
            </ZCard>
            <ZCard title="实际付款净额">
              <strong class="tabular-nums">{{ money(report.cash) }}</strong><div class="text-z-xs text-z-text-secondary">
                按付款日期统计
              </div>
            </ZCard>
            <ZCard title="未分摊金额">
              <strong class="tabular-nums">{{ money(report.unallocated) }}</strong><div class="text-z-xs text-z-text-secondary">
                费用未知 {{ report.metrics.unknownCosts }} 条
              </div>
            </ZCard>
          </div>
          <ZCard title="费用口径">
            <div class="grid gap-2 text-z-sm sm:grid-cols-2">
              <div v-for="c in report.metrics.usageCosts" :key="c.currency">
                {{ c.currency }}：上游报告 {{ c.providerReported }}，本地估算 {{ c.calculated }}
              </div>
              <div>期间分摊基数：{{ money(report.allocationBase) }}</div><div>已分摊支出：{{ money(report.allocated) }}</div>
            </div>
            <ul class="mt-3 space-y-1 text-z-xs text-z-text-secondary">
              <li v-for="warning in report.warnings" :key="warning">
                {{ warning }}
              </li>
            </ul>
          </ZCard>
          <GroupTable title="部门统计" :groups="report.departments" />
          <GroupTable title="人员统计" :groups="report.people" />
          <GroupTable title="项目统计" :groups="report.projects" />
          <GroupTable title="模型统计" :groups="report.models" />
          <ZCard title="支出分摊核对" padding="none">
            <ZTable :columns="expenseColumns" :data="expenseRows" row-key="id" empty-text="当前范围没有分摊基数" />
          </ZCard>
          <ZCard :title="`请求明细 · 共 ${report.recordCount} 条，预览前100条，Excel 导出全部`" padding="none">
            <ZTable :columns="recordColumns" :data="recordRows" row-key="id" />
          </ZCard>
          <ZCard v-if="!closedView" title="保存月结版本">
            <div class="grid gap-3 sm:grid-cols-2">
              <ZFormItem label="名称">
                <ZInput v-model="label" aria-label="月结名称" />
              </ZFormItem><ZFormItem label="调整说明（重复期间必填）">
                <ZInput v-model="reason" aria-label="调整说明" />
              </ZFormItem>
            </div>
            <ZButton :disabled="busy" @click="closeReport">
              保存月结快照
            </ZButton>
          </ZCard>
        </template>
        <ZCard v-else title="选择日期后查询统计" description="先登记人员归属、费用池和付款，可同时查看用量及实际支出" />
      </template>

      <template v-if="collection && state">
        <div v-if="collection === 'assignments'" class="text-z-sm text-z-text-secondary">
          使用稳定 Key ID 关联员工、部门和项目，调岗时新增有效期记录，历史月结保留原归属
        </div>
        <div v-if="collection === 'pools'" class="text-z-sm text-z-text-secondary">
          账号、Key 和 Provider 条件共同匹配，留空表示不限，请避免同一请求匹配多个费用池
        </div>
        <div v-if="collection === 'payments'" class="text-z-sm text-z-text-secondary">
          充值的分摊基数填零，消费结算另行登记，退款填负数并关联原付款
        </div>
        <EntityEditor :key="collection" :title="titles[collection]" :rows="state.settings[collection]" :fields="fields[collection]" :busy="busy" @save="saveRow" @remove="removeRow" @error="error = $event" />
        <ZCard v-if="collection === 'assignments'" title="Key ID 查询">
          <ZButton :loading="busy" @click="action(async () => { keys = await request<typeof keys>('api/keys') })">
            读取 Key 列表
          </ZButton>
          <div v-for="key in keys" :key="key.id" class="mt-2 text-z-sm">
            {{ key.name }} · {{ key.id }} · {{ key.enabled ? '已启用' : '已停用' }}
          </div>
        </ZCard>
      </template>

      <template v-if="tab === 'closed' && state">
        <ZCard title="已保存的月结版本">
          <div v-if="!state.closedReports.length" class="text-z-text-secondary">
            暂无月结，在统计总览查询后保存快照
          </div>
          <div v-for="closed in state.closedReports" :key="closed.id" class="flex flex-wrap items-center justify-between gap-3 py-3">
            <div>
              <div>{{ closed.label }}</div><div class="text-z-xs text-z-text-secondary">
                {{ closed.query.startDate }} 至 {{ closed.query.endDate }} · {{ closed.records }} 条记录 · {{ closed.reason || '首次保存' }}
              </div>
            </div>
            <ZButton :disabled="busy" @click="openClosed(closed.id)">
              查看与导出
            </ZButton>
            <ZButton :disabled="busy" variant="text" @click="deleteId = closed.id; deleteDialog = true">
              移除归档
            </ZButton>
          </div>
        </ZCard>
      </template>

      <template v-if="tab === 'settings' && state">
        <ZCard title="报表时区">
          <ZFormItem label="IANA 时区">
            <ZInput v-model="zone" aria-label="报表时区" placeholder="Asia/Shanghai" />
          </ZFormItem>
          <ZButton :disabled="busy" @click="action(() => saveSettings({ ...state!.settings, timeZone: zone }))">
            保存时区
          </ZButton>
        </ZCard>
        <ZCard title="批量导入付款 CSV" description="UTF-8 编码，按付款ID去重，任何一行无效时整批不写入">
          <ZInput :model-value="paymentTemplate" readonly aria-label="CSV 表头模板" />
          <label for="payments-csv" class="my-3 block text-z-sm">选择 CSV 文件<input id="payments-csv" type="file" accept=".csv,text/csv" class="ml-3" @change="readCsv"></label>
          <ZTextarea v-model="csv" aria-label="付款 CSV 内容" :rows="6" />
          <ZButton class="mt-3" :disabled="busy || !csv.trim()" @click="importPayments">
            校验并导入
          </ZButton>
        </ZCard>
        <ZCard title="配置 JSON" description="包含人员归属、费用池、付款和预算，保存前执行相同校验，月结快照不会被覆盖">
          <ZTextarea v-model="jsonSettings" aria-label="配置 JSON" :rows="12" />
          <ZButton class="mt-3" :disabled="busy" @click="action(() => saveSettings(JSON.parse(jsonSettings) as Settings))">
            校验并保存配置
          </ZButton>
        </ZCard>
      </template>
      <ZDialog v-model="deleteDialog" title="移除月结归档" description="请先导出并另行保存 Excel，移除后无法恢复该版本，付款和当前配置仍会保留">
        <template #footer>
          <ZButton :disabled="busy" @click="deleteDialog = false">
            取消
          </ZButton>
          <ZButton type="danger" :loading="busy" @click="deleteClosed">
            已另行保存，确认移除
          </ZButton>
        </template>
      </ZDialog>
    </main>
  </ZConfigProvider>
</template>
