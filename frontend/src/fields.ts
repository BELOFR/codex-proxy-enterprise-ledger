import type { Collection } from './api'
import type { Field } from './components/EntityEditor.vue'
import { computed } from 'vue'
import { tr } from './i18n'

const field = (key: string, zh: string, en: string, extra: Partial<Field> = {}): Field => ({ key, label: tr(zh, en), ...extra })
export const titles = computed<Record<Collection, string>>(() => ({ assignments: tr('人员与部门归属', 'People & ownership'), pools: tr('费用池与分摊规则', 'Cost pools & allocation'), payments: tr('实际支出', 'Actual payments'), budgets: tr('部门与项目预算', 'Department & project budgets') }))
export const fields = computed<Record<Collection, Field[]>>(() => {
  const dates = [field('startDate', '归属开始（含）', 'Period start (inclusive)', { placeholder: 'YYYY-MM-DD' }), field('endDate', '归属结束（不含）', 'Period end (exclusive)', { placeholder: 'YYYY-MM-DD' })]
  const currency = field('currency', '币种', 'Currency', { default: 'CNY', options: ['CNY', 'USD', 'EUR', 'GBP', 'HKD', 'JPY'].map(value => ({ label: value, value })) })
  return {
    assignments: [field('id', '归属ID', 'Assignment ID'), field('keyId', 'Client Key ID', 'Client Key ID'), field('person', '员工', 'Employee'), field('department', '部门', 'Department'), field('project', '项目', 'Project'), dates[0]!, field('endDate', '生效结束（不含，可留空）', 'Effective end (exclusive, optional)', { kind: 'nullable', placeholder: 'YYYY-MM-DD' })],
    pools: [field('id', '费用池ID', 'Cost pool ID'), field('name', '名称', 'Name'), field('basis', '分摊依据', 'Allocation basis', { default: 'tokens', options: [{ label: tr('按总 Token', 'Total tokens'), value: 'tokens' }, { label: tr('按已知金额', 'Known usage cost'), value: 'cost' }, { label: tr('按人工比例', 'Manual percentages'), value: 'manual' }] }), field('provider', 'Provider（可留空）', 'Provider (optional)', { kind: 'nullable' }), field('accountIds', '账号ID（逗号分隔）', 'Account IDs (comma-separated)', { kind: 'list' }), field('keyIds', 'Key ID（逗号分隔）', 'Key IDs (comma-separated)', { kind: 'list' }), field('shares', '人工比例 JSON（其他规则填 []）', 'Manual shares JSON (otherwise [])', { kind: 'json', default: '[]', placeholder: '[{"person":"Alex","department":"Engineering","project":"Assistant","percent":"100"}]' })],
    payments: [field('id', '付款ID', 'Payment ID'), field('paidDate', '付款日期', 'Payment date', { placeholder: 'YYYY-MM-DD' }), field('supplier', '供应商', 'Supplier'), field('voucher', '凭证编号', 'Voucher number'), field('kind', '类别', 'Category', { default: 'subscription', options: [{ label: tr('订阅费', 'Subscription'), value: 'subscription' }, { label: tr('API 结算', 'API settlement'), value: 'api_settlement' }, { label: tr('预付充值', 'Prepaid recharge'), value: 'recharge' }, { label: tr('退款', 'Refund'), value: 'refund' }, { label: tr('其他', 'Other'), value: 'other' }] }), field('poolId', '费用池ID', 'Cost pool ID'), currency, field('paidAmount', '实付金额', 'Paid amount'), field('allocationBase', '整期分摊基数', 'Full-period allocation base'), ...dates, field('originalPaymentId', '退款关联的原付款ID', 'Original payment ID (refunds)', { kind: 'nullable' }), field('notes', '备注', 'Notes')],
    budgets: [field('id', '预算ID', 'Budget ID'), field('scope', '预算维度', 'Budget scope', { default: 'department', options: [{ label: tr('部门', 'Department'), value: 'department' }, { label: tr('项目', 'Project'), value: 'project' }] }), field('name', '部门或项目名称', 'Department or project name'), currency, field('amount', '整期预算', 'Full-period budget'), ...dates],
  }
})
