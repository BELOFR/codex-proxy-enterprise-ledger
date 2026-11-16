import { language, tr } from '../i18n'

export interface HostBridge {
  request: (input: { method: string, path: string, contentType?: string, body?: string }) => Promise<{ status: number, contentType: string, body: ArrayBuffer }>
  callbackTicket: (input: { path: string, ttlSeconds: number }) => Promise<{ callbackUrl: string }>
  openExternal: (url: string) => Promise<void>
}

declare global { interface Window { codexProxyPlugin?: HostBridge } }

export interface Row { id: string, [key: string]: unknown }
export type Collection = 'assignments' | 'pools' | 'payments' | 'budgets'
export interface Settings { timeZone: string, assignments: Row[], pools: Row[], payments: Row[], budgets: Row[] }
export interface Query { startDate: string, endDate: string }
export interface Closed { id: string, label: string, reason: string, closedAtMs: number, query: Query, records: number }
export interface State { version: number | null, settings: Settings, closedReports: Closed[] }
export interface Money { currency: string, amount: string }
export interface Cost { currency: string, providerReported: string, calculated: string }
export interface Metrics { requests: number, completed: number, totalTokens: string, unknownTokens: number, unknownCosts: number, usageCosts: Cost[] }
export interface Group { name: string, metrics: Metrics, allocations: Money[], budgets: Money[], budgetStatus: { currency: string, remaining: string, usagePercent: string | null }[] }
export interface Report {
  id: string
  query: Query
  timeZone: string
  metrics: Metrics
  departments: Group[]
  people: Group[]
  projects: Group[]
  models: Group[]
  cash: Money[]
  allocationBase: Money[]
  allocated: Money[]
  unallocated: Money[]
  expenses: { paymentId: string, poolId: string, currency: string, base: string, allocated: string, unallocated: string, reason: string | null }[]
  warnings: string[]
  recordCount: number
  records: Record<string, unknown>[]
}

export function host(): HostBridge {
  if (!window.codexProxyPlugin)
    throw new Error(tr('请从 Codex Proxy 的插件管理页面打开台账', 'Open this ledger from the Codex Proxy plugin page.'))
  return window.codexProxyPlugin
}

export async function request<T>(path: string, data?: unknown): Promise<T> {
  const result = await host().request({
    path,
    method: data === undefined ? 'GET' : 'POST',
    ...(data === undefined ? {} : { contentType: 'application/json', body: JSON.stringify(data) }),
  })
  if (result.contentType.split(';')[0]?.trim() !== 'application/json')
    throw new Error(tr('插件返回了不支持的内容类型', 'The plugin returned an unsupported content type.'))
  let value: unknown
  try {
    value = JSON.parse(new TextDecoder().decode(result.body))
  }
  catch { throw new Error(tr('插件返回的数据无法解析', 'The plugin response could not be parsed.')) }
  if (result.status < 200 || result.status >= 300) {
    const error = value as { error?: { message?: string } }
    throw new Error(error.error?.message || `HTTP ${result.status}`)
  }
  return value as T
}

export function money(values: Money[]): string {
  return values.map(v => `${v.amount} ${v.currency}`).join(' / ') || tr('未登记', 'Not recorded')
}
export function count(value: string | number): string {
  try {
    return BigInt(value).toLocaleString(language.value === 'zh-CN' ? 'zh-CN' : 'en-US')
  }
  catch { return String(value) }
}
