import tailwindcss from "@tailwindcss/vite"
import vue from "@vitejs/plugin-vue"
import { fileURLToPath, URL } from "node:url"
import Icons from "unplugin-icons/vite"
import { defineConfig } from "vite"

// https://vite.dev/config/
export default defineConfig({
    plugins: [vue(), tailwindcss(), Icons({ compiler: "vue3" })],
    resolve: {
        alias: {
            "@": fileURLToPath(new URL("./src", import.meta.url)),
        },
    },
    server: {
        proxy: {
            "/api": "http://localhost:4242",
        },
    },
    build: {
        outDir: "dist",
    },
})
