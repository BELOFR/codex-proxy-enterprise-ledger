<script setup lang="ts">
import type { Row } from '../api'
import { defineTableColumns, ZButton, ZCard, ZFormItem, ZInput, ZSelect, ZTable, ZTextarea } from '@codex-proxy/ui'
import { computed, ref, watch } from 'vue'
import { tr } from '../i18n'

export interface Field {
  key: string
  label: string
  kind?: 'text' | 'list' | 'json' | 'nullable'
  options?: { label: string, value: string }[]
  placeholder?: string
  default?: string
}
const props = defineProps<{ title: string, rows: Row[], fields: Field[], busy: boolean }>()
const emit = defineEmits<{ save: [row: Row], remove: [id: string], error: [message: string] }>()
const selected = ref('')
const draft = ref<Record<string, string>>({})
const options = computed(() => [{ label: tr('新增记录', 'New record'), value: '' }, ...props.rows.map(row => ({ label: `${row.id} · ${row.person || row.name || row.supplier || ''}`, value: row.id }))])
const tableColumns = computed(() => defineTableColumns<Row>(props.fields.slice(0, 7).map(field => ({ key: field.key, label: field.label, minWidth: 130, formatter: value => Array.isArray(value) ? JSON.stringify(value) : String(value ?? '—') }))))

function reset() {
  const source = props.rows.find(row => row.id === selected.value)
  draft.value = Object.fromEntries(props.fields.map((field) => {
    const value = source?.[field.key]
    return [field.key, value === undefined || value === null
      ? field.default ?? (field.key === 'id' ? crypto.randomUUID() : '')
      : field.kind === 'list' ? (value as string[]).join(',') : field.kind === 'json' ? JSON.stringify(value, null, 2) : String(value)]
  }))
}

// Translating field labels must not discard an unsaved draft.
watch(() => [props.rows, selected.value], reset, { immediate: true })
function save() {
  try {
    const row = Object.fromEntries(props.fields.map((field) => {
      const value = draft.value[field.key]?.trim() ?? ''
      return [field.key, field.kind === 'list' ? value.split(',').map(s => s.trim()).filter(Boolean) : field.kind === 'json' ? JSON.parse(value || '[]') as unknown : field.kind === 'nullable' && !value ? null : value]
    })) as Row
    if (!row.id)
      throw new Error(tr('记录ID不能为空', 'Record ID is required.'))
    emit('save', row)
  }
  catch (error) { emit('error', error instanceof Error ? error.message : tr('表单无效', 'Invalid form.')) }
}
</script>

<template>
  <ZCard :title="title">
    <div class="mb-4 flex flex-wrap items-center gap-3">
      <ZSelect v-model="selected" :options="options" :aria-label="tr('选择记录', 'Select record')" class="w-full sm:w-96" :disabled="busy" />
      <ZButton :disabled="busy" @click="selected = ''; reset()">
        {{ tr('新增', 'New record') }}
      </ZButton>
    </div>
    <div class="grid grid-cols-1 gap-x-4 sm:grid-cols-2 lg:grid-cols-3">
      <ZFormItem v-for="field in fields" :key="field.key" :label="field.label" :class="{ 'sm:col-span-2 lg:col-span-3': field.kind === 'json' }">
        <ZSelect v-if="field.options" :model-value="draft[field.key]" :options="field.options" :aria-label="field.label" :disabled="busy" @update:model-value="draft[field.key] = String($event ?? '')" />
        <ZTextarea v-else-if="field.kind === 'json'" v-model="draft[field.key]" :placeholder="field.placeholder" :aria-label="field.label" :disabled="busy" :rows="5" />
        <ZInput v-else v-model="draft[field.key]" :type="field.key.endsWith('Date') ? 'date' : 'text'" :placeholder="field.placeholder" :aria-label="field.label" :disabled="busy || (field.key === 'id' && Boolean(selected))" />
      </ZFormItem>
      <div class="flex gap-2 sm:col-span-2 lg:col-span-3">
        <ZButton type="primary" :loading="busy" @click="save">
          {{ tr('保存记录', 'Save record') }}
        </ZButton>
        <ZButton v-if="selected" :disabled="busy" @click="emit('remove', selected)">
          {{ tr('删除所选记录', 'Delete selected record') }}
        </ZButton>
      </div>
    </div>
    <div class="mt-5">
      <ZTable :columns="tableColumns" :data="rows" row-key="id" :empty-text="tr('尚未登记，使用上方表单新增', 'No records yet. Add your first record above.')" />
    </div>
  </ZCard>
</template>
