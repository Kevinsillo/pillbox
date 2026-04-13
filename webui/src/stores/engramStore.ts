import { ref } from 'vue'
import { defineStore } from 'pinia'
import { api } from '@/core/infrastructure/api'
import type { Session, Observation, ObservationDetail } from '@/types/engram'
import type { GetSessionsParams, GetObservationsParams } from '@/core/infrastructure/api'

export const useEngramStore = defineStore('engram', () => {
    const sessions = ref<Session[]>([])
    const currentSession = ref<Session | null>(null)
    const observations = ref<Observation[]>([])
    const currentObservation = ref<ObservationDetail | null>(null)
    const projects = ref<string[]>([])
    const loadingSessions = ref(false)
    const loadingSession = ref(false)
    const error = ref<string | null>(null)
    const sessionCache = new Map<string, { session: Session; observations: Observation[] }>()

    async function fetchSessions(params?: GetSessionsParams): Promise<void> {
        loadingSessions.value = true
        error.value = null
        try {
            const result = await api.getSessions(params)
            sessions.value = result.items
        } catch (e) {
            error.value = e instanceof Error ? e.message : 'Unknown error'
        } finally {
            loadingSessions.value = false
        }
    }

    async function fetchSession(id: string): Promise<void> {
        if (sessionCache.has(id)) {
            const cached = sessionCache.get(id)!
            currentSession.value = cached.session
            observations.value = cached.observations
            return
        }
        loadingSession.value = true
        error.value = null
        try {
            const result = await api.getSession(id)
            sessionCache.set(id, result)
            currentSession.value = result.session
            observations.value = result.observations
        } catch (e) {
            error.value = e instanceof Error ? e.message : 'Unknown error'
            currentSession.value = null
        } finally {
            loadingSession.value = false
        }
    }

    async function fetchObservations(params?: GetObservationsParams): Promise<void> {
        loadingSession.value = true
        error.value = null
        try {
            const result = await api.getObservations(params)
            observations.value = result.items
        } catch (e) {
            error.value = e instanceof Error ? e.message : 'Unknown error'
        } finally {
            loadingSession.value = false
        }
    }

    async function fetchObservation(id: number): Promise<void> {
        loadingSession.value = true
        error.value = null
        try {
            const result = await api.getObservation(id)
            currentObservation.value = result.observation
        } catch (e) {
            error.value = e instanceof Error ? e.message : 'Unknown error'
            currentObservation.value = null
        } finally {
            loadingSession.value = false
        }
    }

    async function fetchProjects(): Promise<void> {
        error.value = null
        try {
            const result = await api.getProjects()
            projects.value = result.projects
        } catch (e) {
            error.value = e instanceof Error ? e.message : 'Unknown error'
        }
    }

    return {
        sessions,
        currentSession,
        observations,
        currentObservation,
        projects,
        loadingSessions,
        loadingSession,
        error,
        fetchSessions,
        fetchSession,
        fetchObservations,
        fetchObservation,
        fetchProjects,
    }
})
