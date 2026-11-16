import fs from 'node:fs/promises'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'
import { createTheme, createThemeVars } from '@codex-proxy/ui/theme'

async function main() {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
  for (const scheme of ['light', 'dark']) {
    const theme = createTheme({ scheme })
    const css = `:root {color-scheme:${scheme};${Object.entries(createThemeVars(theme.tokens)).map(([key, value]) => `${key}:${value};`).join('')}}`
    await fs.mkdir(path.join(root, '.runtime'), { recursive: true })
    await fs.writeFile(path.join(root, '.runtime', `theme-${scheme}.css`), css)
  }
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
