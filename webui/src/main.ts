import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from '@/App.vue'
import router from '@/router'
import { i18n } from '@/core/infrastructure/i18n'
// Estilos globales
import 'element-plus/dist/index.css'
import '@/styles/element-plus.css'
import '@/styles/style.css'

const app = createApp(App)
app.use(createPinia())
app.use(router)
app.use(i18n)
app.mount('#app')
