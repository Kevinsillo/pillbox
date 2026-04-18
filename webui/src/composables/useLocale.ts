import type { Component } from 'vue'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import IFlagEs from '~icons/flag/es-4x3'
import { availableLocales, detectSystemLanguage } from '@/core/infrastructure/i18n'
import { useLocalStorage } from './useLocalStorage'

const LOCALE_META: Record<string, { name: string; flag: Component }> = {
    es: { name: 'Español', flag: IFlagEs },
}

export function useLocale() {
    const { locale } = useI18n({ useScope: 'global' })
    const stored = useLocalStorage<string>('pillbox:locale', detectSystemLanguage())

    const locales = computed(() =>
        availableLocales
            .filter(code => LOCALE_META[code])
            .map(code => ({ code, name: LOCALE_META[code].name, flag: LOCALE_META[code].flag }))
    )

    const currentLocale = computed(() => locale.value)

    function setLocale(code: string) {
        if (!availableLocales.includes(code)) return
        locale.value = code
        stored.value = code
    }

    return {
        locale,
        currentLocale,
        availableLocales: locales,
        setLocale,
    }
}
