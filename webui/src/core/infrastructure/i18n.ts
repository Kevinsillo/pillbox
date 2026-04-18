import { createI18n } from "vue-i18n"

const messages = {} as Record<string, any>
const locales = import.meta.glob('@/locales/*.json', { eager: true })
for (const path in locales) {
    const locale = path.match(/([\w-]+)\.json$/)?.[1]
    if (locale) {
        messages[locale] = locales[path]
    }
}

export const availableLocales = Object.keys(messages)

export function detectSystemLanguage(): string {
    const stored = localStorage.getItem('pillbox:locale')
    if (stored && availableLocales.includes(stored)) return stored

    const browser = navigator.language.split('-')[0]
    if (availableLocales.includes(browser)) return browser

    return 'es'
}

export const i18n = createI18n({
    legacy: false,
    locale: detectSystemLanguage(),
    globalInjection: true,
    fallbackLocale: "es",
    availableLocales,
    messages,
})
