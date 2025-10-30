import type {
  Candle,
  CreateFetchTaskRequest,
  CreateTaskResponse,
  ErrorResponse,
  Task,
  TaskEvent,
  Timeframe,
  GetSourceResponse,
  GetSourceQuery,
  SaveSourceQuery,
  DeleteSourceQuery,
  MoveSourceQuery,
  AddStrategyRequest
} from '@/types'

const API_BASE_URL = 'http://localhost:3001'

class ApiError extends Error {
  constructor(public error: string, message: string, public status: number) {
    super(message)
    this.name = 'ApiError'
  }
}

async function fetchAPI<T>(endpoint: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${endpoint}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  })

  if (!response.ok) {
    const error: ErrorResponse = await response.json()
    throw new ApiError(error.error, error.message, response.status)
  }

  return response.json()
}

export const api = {
  health: {
    check: () => fetchAPI<string>('/health'),
  },

  exchanges: {
    list: () => fetchAPI<string[]>('/exchanges'),
  },

  symbols: {
    list: (exchange: string) => fetchAPI<string[]>(`/symbols?exchange=${encodeURIComponent(exchange)}`),
  },

  timeframes: {
    list: (exchange: string) => fetchAPI<Record<Timeframe, string>>(`/timeframes?exchange=${encodeURIComponent(exchange)}`),
  },

  tasks: {
    getAll: () => fetchAPI<Task[]>('/tasks'),

    getById: (id: string) => fetchAPI<Task>(`/tasks/${id}`),

    createFetch: (request: CreateFetchTaskRequest) =>
      fetchAPI<CreateTaskResponse>('/tasks/fetch', {
        method: 'POST',
        body: JSON.stringify(request),
      }),

    stream: (onEvent: (event: TaskEvent) => void, onError?: (error: Error) => void) => {
      const eventSource = new EventSource(`${API_BASE_URL}/tasks/stream`)

      eventSource.onmessage = (event) => {
        try {
          const taskEvent: TaskEvent = JSON.parse(event.data)
          onEvent(taskEvent)
        } catch (error) {
          console.error('Failed to parse task event:', error)
        }
      }

      eventSource.onerror = (error) => {
        console.error('SSE connection error:', error)
        onError?.(new Error('SSE connection failed'))
      }

      return () => {
        eventSource.close()
      }
    },
  },

  candles: {
    get: (params: {
      exchange: string
      symbol: string
      timeframe: Timeframe
      start?: number
      end?: number
    }) => {
      const query = new URLSearchParams({
        exchange: params.exchange,
        symbol: params.symbol,
        timeframe: params.timeframe,
        ...(params.start && { start: params.start.toString() }),
        ...(params.end && { end: params.end.toString() }),
      })
      return fetchAPI<Candle[]>(`/candles?${query}`)
    },
  },

  source: {
    get: (query: GetSourceQuery) =>
      fetchAPI<GetSourceResponse>(`/strategy/source/get?path=${encodeURIComponent(query.path)}`),

    save: (query: SaveSourceQuery, content: string) =>
      fetchAPI<void>(`/strategy/source/save?path=${encodeURIComponent(query.path)}`, {
        method: 'POST',
        body: JSON.stringify(content),
      }),

    delete: (query: DeleteSourceQuery) =>
      fetchAPI<void>(`/strategy/source/delete?path=${encodeURIComponent(query.path)}`),

    move: (query: MoveSourceQuery) =>
      fetchAPI<void>(`/strategy/source/move?old_path=${encodeURIComponent(query.old_path)}&new_path=${encodeURIComponent(query.new_path)}`),
  },

  strategy: {
    add: (request: AddStrategyRequest) =>
      fetchAPI<void>('/strategy/add', {
        method: 'POST',
        body: JSON.stringify(request),
      }),
  },
}

export { ApiError }
