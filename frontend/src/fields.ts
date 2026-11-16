import type { Collection } from './api'
import type { Field } from './components/EntityEditor.vue'

const field = (key: string, label: string, extra: Partial<Field> = {}): Field => ({ key, label, ...extra })
const dates = [field('startDate', '归属开始（含）', { placeholder: 'YYYY-MM-DD' }), field('endDate', '归属结束（不含）', { placeholder: 'YYYY-MM-DD' })]
const currency = field('currency', '币种', { default: 'CNY', options: ['CNY', 'USD', 'EUR', 'GBP', 'HKD', 'JPY'].map(value => ({ label: value, value })) })
export const titles: Record<Collection, string> = { assignments: '人员与部门归属', pools: '费用池与分摊规则', payments: '实际支出', budgets: '部门与项目预算' }
export const fields: Record<Collection, Field[]> = {
  assignments: [field('id', '归属ID'), field('keyId', 'Client Key ID'), field('person', '员工'), field('department', '部门'), field('project', '项目'), dates[0]!, field('endDate', '生效结束（不含，可留空）', { kind: 'nullable', placeholder: 'YYYY-MM-DD' })],
  pools: [field('id', '费用池ID'), field('name', '名称'), field('basis', '分摊依据', { default: 'tokens', options: [{ label: '按总 Token', value: 'tokens' }, { label: '按已知金额', value: 'cost' }, { label: '按人工比例', value: 'manual' }] }), field('provider', 'Provider（可留空）', { kind: 'nullable' }), field('accountIds', '账号ID（逗号分隔）', { kind: 'list' }), field('keyIds', 'Key ID（逗号分隔）', { kind: 'list' }), field('shares', '人工比例 JSON（其他规则填 []）', { kind: 'json', default: '[]', placeholder: '[{"person":"张三","department":"研发部","project":"项目A","percent":"100"}]' })],
  payments: [field('id', '付款ID'), field('paidDate', '付款日期', { placeholder: 'YYYY-MM-DD' }), field('supplier', '供应商'), field('voucher', '凭证编号'), field('kind', '类别', { default: 'subscription', options: [{ label: '订阅费', value: 'subscription' }, { label: 'API 结算', value: 'api_settlement' }, { label: '预付充值', value: 'recharge' }, { label: '退款', value: 'refund' }, { label: '其他', value: 'other' }] }), field('poolId', '费用池ID'), currency, field('paidAmount', '实付金额'), field('allocationBase', '整期分摊基数'), ...dates, field('originalPaymentId', '退款关联的原付款ID', { kind: 'nullable' }), field('notes', '备注')],
  budgets: [field('id', '预算ID'), field('scope', '预算维度', { default: 'department', options: [{ label: '部门', value: 'department' }, { label: '项目', value: 'project' }] }), field('name', '部门或项目名称'), currency, field('amount', '整期预算'), ...dates],
}
