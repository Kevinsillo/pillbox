import { ref } from 'vue'
import { defineStore } from 'pinia'

export const useUiStore = defineStore('ui', () => {
    const sidebarOpen = ref(true)
    const selectedProject = ref<string | null>(null)

    function toggleSidebar(): void {
        sidebarOpen.value = !sidebarOpen.value
    }

    function setProject(project: string | null): void {
        selectedProject.value = project
    }

    return {
        sidebarOpen,
        selectedProject,
        toggleSidebar,
        setProject,
    }
})
