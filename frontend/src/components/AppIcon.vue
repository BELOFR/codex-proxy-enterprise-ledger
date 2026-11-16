<script setup lang="ts">
withDefaults(defineProps<{ name?: string, size?: number }>(), { name: 'ledger', size: 20 })
const paths: Record<string, string> = {
  ledger: 'M5 3h11l4 4v14H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2Zm11 0v5h4M7 16v-3m4 3v-6m4 6v-4M5 21v-3h15',
  chart: 'M4 3v17h17M8 15v-4m5 4V7m5 8v-6',
  users: 'M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2m20 0v-2a4 4 0 0 0-3-3.87M9 3a4 4 0 1 0 0 8 4 4 0 0 0 0-8m8 .13a4 4 0 0 1 0 7.75',
  payments: 'M3 7h18v13H3V7Zm0 4h18M7 16h3M3 7l2-4h14l2 4',
  pools: 'm12 3 10 5-10 5L2 8l10-5ZM2 12l10 5 10-5M2 16l10 5 10-5',
  budget: 'M4 4h16v16H4V4Zm0 5h16M9 9v11m4-7h3m-3 4h3',
  archive: 'M3 3h18v5H3V3Zm2 5v13h14V8m-10 5h6',
  settings: 'M4 6h16M4 12h16M4 18h16M8 3v6m8 0v6M8 15v6',
  book: 'M12 5v16m0-16C8 2 4 3 2 4v15c4-2 7-1 10 2 3-3 6-4 10-2V4c-2-1-6-2-10 1Z',
  globe: 'M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0ZM3 12h18m-9-9c5 5 5 13 0 18-5-5-5-13 0-18Z',
  download: 'M12 3v12m-5-5 5 5 5-5M4 15v6h16v-6',
  arrow: 'M5 12h14m-5-5 5 5-5 5',
  check: 'm5 12 4 4L19 6',
  refresh: 'M20 7V3m0 4h-4m4 0a9 9 0 1 0 1 8',
  clock: 'M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9-5v5l3 2',
  shield: 'm12 3 8 3v6c0 5-8 9-8 9s-8-4-8-9V6l8-3Zm-4 9 3 3 5-6',
}
</script>

<template>
  <svg :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.65" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path :d="paths[name] || paths.ledger" /></svg>
</template>
