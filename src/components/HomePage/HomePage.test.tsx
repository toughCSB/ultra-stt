import React from 'react'
import { cleanup, fireEvent, render, screen, act } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import i18n from '../../i18n'
import { useAppStore, type HistoryEntry } from '../../stores/appStore'
import { HomePage } from './index'
import { setPolishStyle } from '../../lib/tauri'

vi.mock('../../lib/tauri', () => ({ setPolishStyle: vi.fn() }))

vi.mock('framer-motion', () => ({
  motion: new Proxy(
    {},
    {
      get:
        (_target, tag: string) =>
        ({ children, ...props }: React.HTMLAttributes<HTMLElement>) =>
          React.createElement(tag, props, children),
    },
  ),
}))

const entry: HistoryEntry = {
  id: 1,
  created_at: '2026-09-30T10:00:00',
  context_profile_id: 'unknown',
  context_label: 'Editor',
  context_icon_key: '',
  context_family: 'general',
  browser_access_status: 'not_applicable',
  provider_kind: 'local',
  raw_text: 'one',
  polished_text: 'one',
  language: 'ko',
  duration_ms: null,
  active_scene_id: null,
  active_scene_source: null,
  active_scene_name: null,
  active_scene_prompt_chars: null,
  active_scene_prompt_truncated: false,
  output_status: null,
  output_error: null,
}

describe('HomePage recent history activity', () => {
  beforeEach(async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date(2026, 8, 30, 12))
    await i18n.changeLanguage('ko')
    useAppStore.setState({
      history: [entry, { ...entry, id: 2 }, { ...entry, id: 3, created_at: '2026-09-28T09:00:00' }],
      lastPipelineTiming: null,
    })
  })

  afterEach(() => {
    cleanup()
    vi.useRealTimers()
    window.location.hash = ''
    useAppStore.setState(useAppStore.getInitialState())
  })

  it('shows 14 dated counts including empty days and opens the chosen History date', () => {
    render(<HomePage />)

    expect(screen.getAllByRole('button', { name: /2026-09-\d\d.*건/ })).toHaveLength(14)
    const today = screen.getByRole('button', { name: /2026-09-30.*2건/ })
    expect(today).toHaveAttribute('title', expect.stringContaining('2026-09-30'))
    expect(screen.getByRole('button', { name: /2026-09-29.*0건/ })).toBeInTheDocument()
    fireEvent.click(today)
    expect(window.location.hash).toBe('#/history?date=2026-09-30')
  })

  it('persists a one-click mode change and highlights the saved selection', async () => {
    vi.mocked(setPolishStyle).mockResolvedValue({ polish_style: 'original', polish_enabled: true })
    render(<HomePage />)
    expect(screen.getByRole('radio', { name: '깔끔하게' })).toHaveAttribute('aria-checked', 'true')
    await act(async () => fireEvent.click(screen.getByRole('radio', { name: '원문 · Original' })))
    expect(setPolishStyle).toHaveBeenCalledWith('original')
    expect(screen.getByRole('radio', { name: '원문 · Original' })).toHaveAttribute(
      'aria-checked',
      'true',
    )
    expect(useAppStore.getState().config.polish_enabled).toBe(true)
  })

  it('keeps the previous mode when persistence fails', async () => {
    vi.mocked(setPolishStyle).mockRejectedValue(new Error('Save failed'))
    render(<HomePage />)
    await act(async () => fireEvent.click(screen.getByRole('radio', { name: '최소' })))
    expect(screen.getByRole('alert')).toBeInTheDocument()
    expect(screen.getByRole('radio', { name: '깔끔하게' })).toHaveAttribute('aria-checked', 'true')
  })

  it('shows the most recent session timing as STT, LLM, and total seconds', () => {
    useAppStore.setState({ lastPipelineTiming: { stt_ms: 2400, llm_ms: 1100, total_ms: 3900 } })
    render(<HomePage />)

    expect(screen.getByText('2.4초')).toBeInTheDocument()
    expect(screen.getByText('1.1초')).toBeInTheDocument()
    expect(screen.getByText('3.9초')).toBeInTheDocument()
  })

  it('labels ultra-remote timing as raw recognition with separate local polish', () => {
    useAppStore.setState((state) => ({
      config: {
        ...state.config,
        stt_provider: 'custom-whisper',
        stt_custom_preset: 'ultra-remote',
      },
      lastPipelineTiming: { stt_ms: 2400, llm_ms: 1100, total_ms: 3900 },
    }))
    render(<HomePage />)

    expect(screen.getByText('원격 음성 인식')).toBeInTheDocument()
    expect(
      screen.getByText(
        '원격 서버는 음성을 원문 텍스트로 인식합니다. 다듬기가 켜져 있으면 이 컴퓨터에서 처리하고 시간을 따로 측정합니다.',
      ),
    ).toBeInTheDocument()
    expect(screen.getByText('1.1초')).toBeInTheDocument()
  })

  it('shows the saved API STT and Claude CLI route instead of pending legacy providers', () => {
    const base = useAppStore.getState().config
    useAppStore.setState({
      config: { ...base, stt_provider: 'custom-whisper', stt_custom_preset: 'ultra-remote' },
      savedConfig: {
        ...base,
        engine_mode: 'openai',
        desktop_polish_model: 'claude-sonnet-5-5',
        polish_enabled: false,
      },
      lastPipelineTiming: { stt_ms: 2400, llm_ms: 1100, total_ms: 3900 },
    })
    render(<HomePage />)

    expect(screen.getAllByText('GPT Transcribe')).toHaveLength(2)
    expect(screen.getAllByText('claude-sonnet-5-5 · Claude CLI')).toHaveLength(2)
    expect(screen.queryByText('원격 음성 인식')).toBeNull()
    expect(screen.queryByText(i18n.t('home.remoteTimingNote'))).toBeNull()
    expect(screen.getByText('인식')).toBeInTheDocument()
  })

  it('shows the saved existing remote route and pending Grok warning', () => {
    const base = useAppStore.getState().config
    useAppStore.setState({
      config: { ...base, engine_mode: 'openai' },
      savedConfig: {
        ...base,
        engine_mode: 'existing',
        stt_provider: 'custom-whisper',
        stt_custom_preset: 'ultra-remote',
        desktop_polish_model: 'grok-4.7',
      },
      lastPipelineTiming: { stt_ms: 2400, llm_ms: 1100, total_ms: 3900 },
    })
    render(<HomePage />)

    expect(screen.getAllByText('custom-whisper')).toHaveLength(2)
    expect(screen.getAllByText('grok-4.7 · Grok CLI · 지원 대기 중')).toHaveLength(2)
    expect(screen.getByText('원격 음성 인식')).toBeInTheDocument()
    expect(screen.getByText(i18n.t('home.remoteTimingNote'))).toBeInTheDocument()
  })

  it('shows the effective Codex CLI or OpenAI API route for a saved GPT model', () => {
    const base = useAppStore.getState().config
    useAppStore.setState({ savedConfig: { ...base, engine_mode: 'existing' } })
    render(<HomePage />)
    expect(screen.getAllByText('gpt-6-luna · Codex CLI')).toHaveLength(2)

    act(() => useAppStore.setState({ savedConfig: { ...base, engine_mode: 'openai' } }))
    expect(screen.getAllByText('gpt-6-luna · OpenAI API')).toHaveLength(2)
  })
})
