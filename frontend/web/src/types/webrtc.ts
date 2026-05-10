import type { Instance } from 'simple-peer'

export interface WebRTCSignal {
  type: 'offer' | 'answer' | 'ice-candidate' | 'connect' | 'data'
  data: any
  senderId: string
  receiverId: string
}

export interface WebRTCConfig {
  initiator: boolean
  trickle?: boolean
  config?: RTCConfiguration
  reconnectAttempts?: number
  reconnectDelay?: number
}

export type ConnectionState =
  | 'new'
  | 'connecting'
  | 'connected'
  | 'disconnected'
  | 'failed'
  | 'closed'

export interface WebRTCError {
  type: 'connection' | 'signal' | 'ice' | 'data' | 'unknown'
  message: string
  error?: Error
}

export interface UseWebRTCReturn {
  peer: Instance | null
  signal: WebRTCSignal | null
  connect: (signalData: WebRTCSignal) => void
  disconnect: () => void
  error: WebRTCError | null
  connectionState: ConnectionState
  reconnect: () => void
  sendData: (data: any) => void
  isInitiator: boolean
}
