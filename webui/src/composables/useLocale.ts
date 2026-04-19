import type { Component } from 'vue'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import IFlagEs from '~icons/flag/es-4x3'
import IFlagEn from '~icons/flag/gb-4x3'
import IFlagDe from '~icons/flag/de-4x3'
import IFlagIt from '~icons/flag/it-4x3'
import IFlagPt from '~icons/flag/pt-4x3'
import IFlagFr from '~icons/flag/fr-4x3'
import { availableLocales, detectSystemLanguage } from '@/core/infrastructure/i18n'
import { useLocalStorage } from './useLocalStorage'

const LOCALE_META: Record<string, { name: string; flag: Component }> = {
    es: { name: 'Español', flag: IFlagEs },
    en: { name: 'English', flag: IFlagEn },
    de: { name: 'Deutsch', flag: IFlagDe },
    it: { name: 'Italiano', flag: IFlagIt },
    pt: { name: 'Português', flag: IFlagPt },
    fr: { name: 'Français', flag: IFlagFr },
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
