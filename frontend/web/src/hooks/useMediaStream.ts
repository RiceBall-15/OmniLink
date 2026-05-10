import { useState, useEffect, useRef } from 'react'

export type MediaType = 'audio' | 'video' | 'screen'

export interface MediaDevice {
  deviceId: string
  kind: 'audioinput' | 'videoinput'
  label: string
}

export interface UseMediaStreamReturn {
  stream: MediaStream | null
  error: string | null
  devices: MediaDevice[]
  startStream: (type: MediaType, deviceId?: string) => Promise<void>
  stopStream: () => void
  switchDevice: (type: MediaType, deviceId: string) => Promise<void>
  isLoading: boolean
}

export function useMediaStream(): UseMediaStreamReturn {
  const [stream, setStream] = useState<MediaStream | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [devices, setDevices] = useState<MediaDevice[]>([])
  const [isLoading, setIsLoading] = useState(false)

  const streamRef = useRef<MediaStream | null>(null)

  useEffect(() => {
    loadDevices()
    return () => {
      stopStream()
    }
  }, [])

  const loadDevices = async () => {
    try {
      const mediaDevices = await navigator.mediaDevices.enumerateDevices()
      const filteredDevices: MediaDevice[] = mediaDevices
        .filter((device) => device.kind === 'audioinput' || device.kind === 'videoinput')
        .map((device) => ({
          deviceId: device.deviceId,
          kind: device.kind as 'audioinput' | 'videoinput',
          label: device.label || `${device.kind} ${device.deviceId.slice(0, 8)}`,
        }))
      setDevices(filteredDevices)
    } catch (err) {
      setError('Failed to enumerate devices')
    }
  }

  const startStream = async (type: MediaType, deviceId?: string) => {
    setIsLoading(true)
    setError(null)

    try {
      const constraints: MediaStreamConstraints = {}

      if (type === 'audio' || type === 'video') {
        if (type === 'audio' || type === 'video') {
          constraints.audio = type === 'audio' ? true : undefined
          constraints.video = type === 'video' ? true : undefined

          if (deviceId) {
            if (type === 'audio') {
              constraints.audio = { deviceId: { exact: deviceId } }
            } else {
              constraints.video = { deviceId: { exact: deviceId } }
            }
          }
        }
      } else if (type === 'screen') {
        constraints.video = true
        constraints.audio = false
      }

      const mediaStream = await navigator.mediaDevices.getUserMedia(constraints)
      streamRef.current = mediaStream
      setStream(mediaStream)
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to get media stream'

      if (errorMessage.includes('Permission denied')) {
        setError('Permission denied. Please allow access to camera/microphone.')
      } else if (errorMessage.includes('NotFoundError')) {
        setError('No camera or microphone found.')
      } else {
        setError(errorMessage)
      }
    } finally {
      setIsLoading(false)
    }
  }

  const stopStream = () => {
    if (streamRef.current) {
      streamRef.current.getTracks().forEach((track) => track.stop())
      streamRef.current = null
      setStream(null)
    }
  }

  const switchDevice = async (type: MediaType, deviceId: string) => {
    const currentStream = streamRef.current
    const trackType = type === 'audio' ? 'audio' : 'video'

    if (currentStream) {
      const tracks = currentStream.getTracks()
      const trackToReplace = tracks.find((t) => t.kind === trackType)

      if (trackToReplace) {
        try {
          const constraints: MediaTrackConstraints = { deviceId: { exact: deviceId } }
          const newTrack = await navigator.mediaDevices.getUserMedia({
            [trackType]: constraints,
          })

          const newStreamTrack = newTrack.getTracks()[0]
          currentStream.removeTrack(trackToReplace)
          currentStream.addTrack(newStreamTrack)
          setStream(currentStream)
        } catch (err) {
          setError('Failed to switch device')
        }
      }
    }
  }

  return {
    stream,
    error,
    devices,
    startStream,
    stopStream,
    switchDevice,
    isLoading,
  }
}