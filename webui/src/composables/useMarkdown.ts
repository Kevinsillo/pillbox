import { Marked } from "marked"
import hljs from "highlight.js/lib/core"
import type { LanguageFn } from "highlight.js"

// Vite crea un chunk lazy por cada fichero — solo se descarga el que se necesita
const langModules = import.meta.glob<{ default: LanguageFn }>(
  "/node_modules/highlight.js/lib/languages/*.js"
)

// Aliases: lo que el usuario escribe → nombre canónico en highlight.js
const ALIASES: Record<string, string> = {
  sh: "bash",
  shell: "bash",
  zsh: "bash",
  js: "javascript",
  mjs: "javascript",
  cjs: "javascript",
  ts: "typescript",
  py: "python",
  rb: "ruby",
  yml: "yaml",
  toml: "ini",
  tf: "hcl",
}

const registered = new Set<string>()

async function loadLanguage(lang: string): Promise<void> {
  const canonical = ALIASES[lang] ?? lang
  if (registered.has(canonical)) return
  const key = `/node_modules/highlight.js/lib/languages/${canonical}.js`
  const importer = langModules[key]
  if (!importer) return
  const mod = await importer()
  hljs.registerLanguage(canonical, mod.default)
  if (lang !== canonical) hljs.registerLanguage(lang, mod.default)
  registered.add(canonical)
}

function extractLangs(markdown: string): string[] {
  const langs: string[] = []
  const re = /^```(\w+)/gm
  let m: RegExpExecArray | null
  while ((m = re.exec(markdown)) !== null) {
    if (m[1]) langs.push(m[1].toLowerCase())
  }
  return [...new Set(langs)]
}

const renderer = {
  code({ text, lang }: { text: string; lang?: string }) {
    const language = lang && hljs.getLanguage(lang) ? lang : "plaintext"
    const highlighted = hljs.highlight(text, { language }).value
    const label = lang ? `<span class="code-lang">${lang}</span>` : ""
    return `<div class="code-block">${label}<pre><code class="hljs language-${language}">${highlighted}</code></pre></div>`
  },
}

const marked = new Marked({ renderer })

export function useMarkdown() {
  async function parse(content: string): Promise<string> {
    const langs = extractLangs(content)
    await Promise.all(langs.map(loadLanguage))
    return marked.parse(content) as string
  }

  return { parse }
}
