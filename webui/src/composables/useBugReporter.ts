import { onMounted, ref } from "vue"
import { useRoute } from "vue-router"
import { metaApi, type AppInfo } from "@/core/infrastructure/repositories/MetaRepository"

/**
 * Builds and opens a pre-filled GitHub issue URL with the current app state.
 * - Loads `AppInfo` on mount (exposed so the caller can disable the button until ready).
 * - `reportBug()` opens a new tab with the issue template populated.
 */
export function useBugReporter() {
    const route = useRoute()
    const info = ref<AppInfo | null>(null)

    onMounted(async () => {
        try {
            info.value = await metaApi.info()
        } catch {}
    })

    const reportBug = () => {
        if (!info.value) return
        const { version, os, arch, family } = info.value
        const params = new URLSearchParams({
            template: "bug_report.yml",
            version: `v${version}`,
            platform: `${os}/${arch} (${family})`,
            browser: navigator.userAgent,
            page: route.fullPath,
            surface: "WebUI",
        })
        window.open(
            `https://github.com/kevinsillo/pillbox/issues/new?${params.toString()}`,
            "_blank",
            "noopener",
        )
    }

    return { info, reportBug }
}
