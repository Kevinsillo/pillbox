export type ResponseStatus = 'success' | 'error'

export interface CustomResponse {
    status: ResponseStatus
    message: string
    data: any
}
