import { useCallback } from 'react'
import { useAppStore } from '../stores/appStore'
import { startRecording as startSession, stopRecording as stopSession } from '../lib/tauri'

export function useRecording() {
  const pipelineState = useAppStore((s) => s.pipelineState)
  const resetRecording = useAppStore((s) => s.resetRecording)

  const startRecording = useCallback(async () => {
    resetRecording()
    await startSession()
  }, [resetRecording])

  const stopRecording = useCallback(async () => {
    await stopSession()
  }, [])

  const isPreparing = pipelineState === 'preparing'
  const isRecording = pipelineState === 'recording'
  const isProcessing =
    pipelineState === 'preparing' ||
    pipelineState === 'transcribing' ||
    pipelineState === 'polishing'
  const isIdle = pipelineState === 'idle'

  return {
    startRecording,
    stopRecording,
    isPreparing,
    isRecording,
    isProcessing,
    isIdle,
    pipelineState,
  }
}
