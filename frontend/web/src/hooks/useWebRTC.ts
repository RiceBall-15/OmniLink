import { useState, useEffect, useRef } from 'react'
import SimplePeer from 'simple-peer'
import type { Instance } from 'simple-peer'
import type { WebRTCConfig, WebRTCSignal, WebRTCError, ConnectionState, UseWebRTCReturn } from '../types/webrtc'

export function useWebRTC(config: WebRTCConfig): UseWebRTCReturn {
  const [peer, setPeer] = useState<Instance | null>(null)
  const [signal, setSignal] = useState<WebRTCSignal | null>(null)
  const [error, setError] = useState<WebRTCError | null>(null)
  const [connectionState, setConnectionState] = useState<ConnectionState>('new')

  const reconnectTimerRef = useRef<NodeJS.Timeout | null>(null)
  const reconnectAttemptsRef = useRef(0)

  const createPeer = () => {
    try {
      const peerInstance = new SimplePeer({
        initiator: config.initiator,
        trickle: config.trickle ?? true,
        config: config.config,
      })

      peerInstance.on('signal', (data) => {
        setSignal({
          type: 'offer',
          data,
          senderId: 'local',
          receiverId: 'remote',
        })
      })

      peerInstance.on('connect', () => {
        setConnectionState('connected')
        setError(null)
        reconnectAttemptsRef.current = 0
      })

      peerInstance.on('close', () => {
        setConnectionState('closed')
      })

      peerInstance.on('error', (err: Error) => {
        setError({
          type: 'connection',
          message: err.message,
          error: err,
        })
        setConnectionState('failed')
      })

      peerInstance.on('iceStateChange', (iceConnectionState) => {
        if (iceConnectionState === 'disconnected') {
          setConnectionState('disconnected')
        } else if (iceConnectionState === 'connected') {
          setConnectionState('connected')
        }
      })

      setPeer(peerInstance)
      return peerInstance
    } catch (err) {
      setError({
        type: 'connection',
        message: err instanceof Error ? err.message : 'Failed to create peer',
        error: err instanceof Error ? err : undefined,
      })
      return null
    }
  }

  useEffect(() => {
    const peerInstance = createPeer()

    return () => {
      if (peerInstance) {
        peerInstance.destroy()
      }
      if (reconnectTimerRef.current) {
        clearTimeout(reconnectTimerRef.current)
      }
    }
  }, [])

  const connect = (signalData: WebRTCSignal) => {
    if (peer && signalData.data) {
      try {
        peer.signal(signalData.data)
        setConnectionState('connecting')
      } catch (err) {
        setError({
          type: 'signal',
          message: err instanceof Error ? err.message : 'Failed to process signal',
          error: err instanceof Error ? err : undefined,
        })
      }
    }
  }

  const disconnect = () => {
    if (peer) {
      peer.destroy()
      setPeer(null)
      setConnectionState('closed')
    }
    if (reconnectTimerRef.current) {
      clearTimeout(reconnectTimerRef.current)
    }
  }

  const reconnect = () => {
    disconnect()
    reconnectAttemptsRef.current++

    if (reconnectAttemptsRef.current <= (config.reconnectAttempts || 3)) {
      reconnectTimerRef.current = setTimeout(() => {
        const newPeer = createPeer()
        if (newPeer) {
          setPeer(newPeer)
        }
      }, config.reconnectDelay || 3000)
    } else {
      setError({
        type: 'connection',
        message: `Max reconnection attempts (${config.reconnectAttempts || 3}) reached`,
      })
    }
  }

  const sendData = (data: any) => {
    if (peer) {
      try {
        peer.send(JSON.stringify(data))
      } catch (err) {
        setError({
          type: 'data',
          message: err instanceof Error ? err.message : 'Failed to send data',
          error: err instanceof Error ? err : undefined,
        })
      }
    }
  }

  return {
    peer,
    signal,
    connect,
    disconnect,
    error,
    connectionState,
    reconnect,
    sendData,
    isInitiator: config.initiator,
  }
}