import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
    {
        path: '/',
        component: () => import('@/layouts/MainLayout.vue'),
        redirect: '/sessions',
        children: [
            {
                path: 'sessions',
                component: () => import('@/views/SessionsListView.vue'),
            },
            {
                path: 'session/:id',
                component: () => import('@/views/SessionDetailView.vue'),
                props: true,
            },
            {
                path: 'session/:sessionId/observation/:id',
                component: () => import('@/views/ObservationDetailView.vue'),
                props: true,
            },
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
