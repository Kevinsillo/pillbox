import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
    {
        path: '/',
        component: () => import('@/layouts/MainLayout.vue'),
        children: [
            { path: '', component: () => import('@/views/DashboardView.vue') },
            { path: 'bottles', component: () => import('@/views/BottlesView.vue') },
            { path: 'bottles/:bottle_id', component: () => import('@/views/BottleDetailView.vue'), props: true },
            { path: 'bottles/:bottle_id/prescriptions/:rx_id', component: () => import('@/views/PrescriptionDetailView.vue'), props: true },
            { path: 'bottles/:bottle_id/prescriptions/:rx_id/pills/:pill_id', component: () => import('@/views/PillDetailView.vue'), props: true },
            { path: 'bottles/:bottle_id/prescriptions/:rx_id/pills/:pill_id/edit', component: () => import('@/views/PillEditView.vue'), props: true },
            { path: 'capsules', component: () => import('@/views/CapsulesView.vue') },
            { path: 'capsules/:id', component: () => import('@/views/CapsuleDetailView.vue'), props: true },
            { path: 'search', component: () => import('@/views/SearchView.vue') },
        ],
    },
    {
        path: '/:pathMatch(.*)*',
        component: () => import('@/views/NotFoundView.vue'),
    },
]

const router = createRouter({
    history: createWebHistory(),
    routes,
})

export default router
