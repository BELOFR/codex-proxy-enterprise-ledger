<script setup lang="ts">
import type { Group } from '../api'
import { defineTableColumns, ZCard, ZTable } from '@codex-proxy/ui'
import { computed } from 'vue'
import { count, money } from '../api'
import { displayName, tr } from '../i18n'

const props = defineProps<{ title: string, groups: Group[] }>()
const rows = computed(() => props.groups.map(g => ({
  id: g.name,
  name: displayName(g.name),
  requests: count(g.metrics.requests),
  tokens: count(g.metrics.totalTokens),
  coverage: tr(`${g.metrics.unknownCosts} 条费用未知`, `${g.metrics.unknownCosts} unknown costs`),
  costs: g.metrics.usageCosts.map(c => `${c.currency} ${tr('上报', 'reported')} ${c.providerReported} / ${tr('估算', 'estimated')} ${c.calculated}`).join(' / ') || tr('暂无已知金额', 'No known costs'),
  allocated: money(g.allocations),
  budget: money(g.budgets),
  budgetStatus: g.budgetStatus.map(b => `${b.remaining} ${b.currency} / ${b.usagePercent === null ? 'N/A' : `${b.usagePercent}%`}`).join(' / ') || tr('未登记', 'Not recorded'),
})))
const columns = computed(() => defineTableColumns<typeof rows.value[number]>([
  { key: 'name', label: tr('归属', 'Owner'), minWidth: 140 },
  { key: 'requests', label: tr('调用数', 'Requests'), width: 110 },
  { key: 'tokens', label: tr('已知总 Token', 'Known tokens'), minWidth: 140 },
  { key: 'allocated', label: tr('已分摊支出', 'Allocated spend'), minWidth: 180 },
  { key: 'budget', label: tr('期间预算', 'Period budget'), minWidth: 160 },
  { key: 'budgetStatus', label: tr('预算余额 / 使用率', 'Remaining / used'), minWidth: 220 },
  { key: 'costs', label: tr('用量金额', 'Usage cost'), minWidth: 300 },
  { key: 'coverage', label: tr('费用覆盖', 'Cost coverage'), minWidth: 150 },
]))
</script>

<template>
  <ZCard :title="title" padding="none">
    <ZTable :columns="columns" :data="rows" row-key="id" :empty-text="tr('当前范围暂无数据', 'No data in this period.')" />
  </ZCard>
</template>
