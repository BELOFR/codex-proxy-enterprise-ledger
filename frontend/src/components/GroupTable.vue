<script setup lang="ts">
import type { Group } from '../api'
import { defineTableColumns, ZCard, ZTable } from '@codex-proxy/ui'
import { computed } from 'vue'
import { count, money } from '../api'

const props = defineProps<{ title: string, groups: Group[] }>()
const rows = computed(() => props.groups.map(g => ({
  name: g.name,
  requests: count(g.metrics.requests),
  tokens: count(g.metrics.totalTokens),
  coverage: `${g.metrics.unknownCosts} 条费用未知`,
  costs: g.metrics.usageCosts.map(c => `${c.currency} 上报 ${c.providerReported} / 估算 ${c.calculated}`).join('；') || '暂无已知金额',
  allocated: money(g.allocations),
  budget: money(g.budgets),
  budgetStatus: g.budgetStatus.map(b => `${b.remaining} ${b.currency} / ${b.usagePercent === null ? '无比例' : `${b.usagePercent}%`}`).join(' / ') || '未登记',
})))
const columns = defineTableColumns<typeof rows.value[number]>([
  { key: 'name', label: '归属', minWidth: 120 },
  { key: 'requests', label: '调用数', width: 100 },
  { key: 'tokens', label: '已知总 Token', minWidth: 140 },
  { key: 'costs', label: '用量金额', minWidth: 300 },
  { key: 'coverage', label: '费用覆盖', minWidth: 140 },
  { key: 'allocated', label: '已分摊支出', minWidth: 180 },
  { key: 'budget', label: '期间预算', minWidth: 160 },
  { key: 'budgetStatus', label: '预算余额 / 使用率', minWidth: 220 },
])
</script>

<template>
  <ZCard :title="title" padding="none">
    <ZTable :columns="columns" :data="rows" row-key="name" empty-text="当前范围暂无数据" />
  </ZCard>
</template>
