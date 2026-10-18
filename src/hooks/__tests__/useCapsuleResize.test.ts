import { createElement } from 'react'
import { act, cleanup, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  getCapsuleCenterPosition,
  getCapsuleFocusable,
  getCapsuleRecoveryPosition,
  getCapsuleShellSize,
  getCapsuleVisibility,
  getRecordingTargetMonitor,
  isCapsuleVisibleOnAnyMonitor,
  useCapsuleResize,
} from '../useCapsuleResize'
import { useAppStore } from '../../stores/appStore'

const windowApiMocks = vi.hoisted(() => ({
  getCurrentWindow: vi.fn(),
  availableMonitors: vi.fn(),
  currentMonitor: vi.fn(),
  primaryMonitor: vi.fn(),
  setFocusable: vi.fn(),
  setSize: vi.fn(),
  setPosition: vi.fn(),
  outerPosition: vi.fn(),
  outerSize: vi.fn(),
  scaleFactor: vi.fn(),
  show: vi.fn(),
  hide: vi.fn(),
  invoke: vi.fn(),
}))

vi.mock('@tauri-apps/api/core', () => ({ invoke: windowApiMocks.invoke }))

vi.mock('@tauri-apps/api/window', () => {
  class LogicalSize {
    constructor(
      public width: number,
      public height: number,
    ) {}
  }

  class PhysicalPosition {
    constructor(
      public x: number,
      public y: number,
    ) {}
  }

  class LogicalPosition extends PhysicalPosition {}

  return {
    getCurrentWindow: windowApiMocks.getCurrentWindow,
    availableMonitors: windowApiMocks.availableMonitors,
    currentMonitor: windowApiMocks.currentMonitor,
    primaryMonitor: windowApiMocks.primaryMonitor,
    LogicalSize,
    PhysicalPosition,
    LogicalPosition,
  }
})

const logicalCapsuleSize = { width: 444, height: 88 }

describe('capsule shell geometry', () => {
  it('uses one 560x112 shell for every recording and result state', () => {
    for (const state of [
      'preparing',
      'recording',
      'transcribing',
      'polishing',
      'outputting',
      'ask_recording',
      'ask_thinking',
    ] as const) {
      expect(getCapsuleShellSize(state, false, false, false)).toEqual({ width: 560, height: 112 })
    }
    expect(getCapsuleShellSize('idle', false, true, false)).toEqual({ width: 560, height: 112 })
    expect(getCapsuleShellSize('idle', false, false, true)).toEqual({ width: 560, height: 112 })
  })
})

describe('getCapsuleVisibility', () => {
  it('recovers a detached macOS screen without treating unlike Retina coordinates as overlap', () => {
    const retina = {
      position: { x: 0, y: 0 },
      size: { width: 3024, height: 1964 },
      scaleFactor: 2,
      workArea: { position: { x: 0, y: 48 }, size: { width: 3024, height: 1880 } },
    }
    expect(
      getCapsuleRecoveryPosition(
        { x: 1700, y: 300, width: 544, height: 120 },
        [retina],
        retina,
        { width: 544, height: 120 },
        1,
      ),
    ).toEqual({ x: 968, y: 868 })
  })
  it('hides idle capsule when auto-hide is enabled', () => {
    expect(
      getCapsuleVisibility({
        capsuleAutoHide: true,
        contextMenuOpen: false,
        capsuleExpanded: false,
        hasError: false,
        pipelineState: 'idle',
      }),
    ).toBe(false)
  })

  it('shows idle capsule when an error appears', () => {
    expect(
      getCapsuleVisibility({
        capsuleAutoHide: true,
        contextMenuOpen: false,
        capsuleExpanded: false,
        hasError: true,
        pipelineState: 'idle',
      }),
    ).toBe(true)
  })

  it('shows active capsule while recording', () => {
    expect(
      getCapsuleVisibility({
        capsuleAutoHide: true,
        contextMenuOpen: false,
        capsuleExpanded: false,
        hasError: false,
        pipelineState: 'recording',
      }),
    ).toBe(true)
  })

  it('keeps capsule visible while preparing even when auto-hide is enabled', () => {
    expect(
      getCapsuleVisibility({
        capsuleAutoHide: true,
        contextMenuOpen: false,
        capsuleExpanded: false,
        hasError: false,
        pipelineState: 'preparing',
      }),
    ).toBe(true)
  })

  it('keeps capsule visible while Ask is recording', () => {
    expect(
      getCapsuleVisibility({
        capsuleAutoHide: true,
        contextMenuOpen: false,
        capsuleExpanded: false,
        hasError: false,
        pipelineState: 'ask_recording',
      }),
    ).toBe(true)
  })

  it('shows idle capsule while the context menu is open', () => {
    expect(
      getCapsuleVisibility({
        capsuleAutoHide: true,
        contextMenuOpen: true,
        capsuleExpanded: false,
        hasError: false,
        pipelineState: 'idle',
      }),
    ).toBe(true)
  })

  it('keeps the capsule overlay from stealing keyboard output focus', () => {
    expect(getCapsuleFocusable()).toBe(false)
    expect(getCapsuleFocusable(true)).toBe(true)
  })

  it('keeps an idle mode chooser visible when automatic hiding is enabled', () => {
    expect(
      getCapsuleVisibility({
        capsuleAutoHide: true,
        contextMenuOpen: false,
        polishModeMenuOpen: true,
        capsuleExpanded: false,
        hasError: false,
        pipelineState: 'idle',
      }),
    ).toBe(true)
  })
})

describe('capsule monitor geometry', () => {
  it.each([
    {
      name: 'primary monitor',
      workArea: { position: { x: 0, y: 0 }, size: { width: 1920, height: 1040 } },
      expected: { x: 738, y: 476 },
    },
    {
      name: 'monitor to the right',
      workArea: { position: { x: 3440, y: 0 }, size: { width: 1920, height: 1040 } },
      expected: { x: 4178, y: 476 },
    },
    {
      name: 'monitor to the left',
      workArea: { position: { x: -1920, y: 0 }, size: { width: 1920, height: 1040 } },
      expected: { x: -1182, y: 476 },
    },
    {
      name: 'monitor above',
      workArea: { position: { x: 0, y: -1080 }, size: { width: 1920, height: 1040 } },
      expected: { x: 738, y: -604 },
    },
  ])('uses the global work-area origin for the $name', ({ workArea, expected }) => {
    expect(getCapsuleCenterPosition({ scaleFactor: 1, workArea }, logicalCapsuleSize)).toEqual(
      expected,
    )
  })

  it('uses physical pixels for mixed-DPI monitors', () => {
    expect(
      getCapsuleCenterPosition(
        {
          scaleFactor: 1.5,
          workArea: { position: { x: 1920, y: 100 }, size: { width: 2560, height: 1400 } },
        },
        logicalCapsuleSize,
      ),
    ).toEqual({ x: 2867, y: 734 })
  })

  it('supports a 200% scale factor without converting the global origin', () => {
    expect(
      getCapsuleCenterPosition(
        {
          scaleFactor: 2,
          workArea: { position: { x: -3840, y: -200 }, size: { width: 3840, height: 2080 } },
        },
        logicalCapsuleSize,
      ),
    ).toEqual({ x: -2364, y: 752 })
  })

  it('uses the work area rather than placing the capsule behind the taskbar', () => {
    expect(
      getCapsuleCenterPosition(
        {
          scaleFactor: 1,
          workArea: { position: { x: 0, y: 40 }, size: { width: 1920, height: 1000 } },
        },
        logicalCapsuleSize,
      ),
    ).toEqual({ x: 738, y: 496 })
  })

  it('preserves a capsule that is even partially visible on any monitor', () => {
    const monitorBounds = [
      { position: { x: 0, y: 0 }, size: { width: 1920, height: 1040 } },
      { position: { x: 1920, y: 0 }, size: { width: 1920, height: 1040 } },
    ]

    expect(
      isCapsuleVisibleOnAnyMonitor({ x: 3830, y: 900, width: 224, height: 60 }, monitorBounds),
    ).toBe(true)
  })

  it('reports a capsule as off-screen when it touches no monitor', () => {
    const monitorBounds = [
      { position: { x: -1920, y: 0 }, size: { width: 1920, height: 1040 } },
      { position: { x: 0, y: 0 }, size: { width: 1920, height: 1040 } },
    ]

    expect(
      isCapsuleVisibleOnAnyMonitor({ x: 2500, y: 900, width: 224, height: 60 }, monitorBounds),
    ).toBe(false)
  })

  it('does not recover a user-positioned capsule that remains partially visible', () => {
    const monitors = [
      {
        position: { x: 0, y: 0 },
        size: { width: 1920, height: 1080 },
        scaleFactor: 1,
        workArea: { position: { x: 0, y: 0 }, size: { width: 1920, height: 1040 } },
      },
      {
        position: { x: 1920, y: 0 },
        size: { width: 2560, height: 1440 },
        scaleFactor: 1.5,
        workArea: { position: { x: 1920, y: 0 }, size: { width: 2560, height: 1400 } },
      },
    ]

    expect(
      getCapsuleRecoveryPosition(
        { x: 4400, y: 1200, width: 336, height: 90 },
        monitors,
        monitors[0],
        logicalCapsuleSize,
      ),
    ).toBeNull()
  })

  it('does not recover a capsule visible only in a taskbar or Dock reserved area', () => {
    const monitors = [
      {
        position: { x: 0, y: 0 },
        size: { width: 1920, height: 1080 },
        scaleFactor: 1,
        workArea: { position: { x: 0, y: 0 }, size: { width: 1920, height: 1040 } },
      },
    ]

    expect(
      getCapsuleRecoveryPosition(
        { x: 848, y: 1050, width: 224, height: 60 },
        monitors,
        monitors[0],
        logicalCapsuleSize,
      ),
    ).toBeNull()
  })

  it('recovers a fully off-screen capsule to the primary work area', () => {
    const monitors = [
      {
        position: { x: 0, y: 0 },
        size: { width: 1920, height: 1080 },
        scaleFactor: 1,
        workArea: { position: { x: 0, y: 0 }, size: { width: 1920, height: 1040 } },
      },
      {
        position: { x: 1920, y: 0 },
        size: { width: 2560, height: 1440 },
        scaleFactor: 1.5,
        workArea: { position: { x: 1920, y: 0 }, size: { width: 2560, height: 1400 } },
      },
    ]

    expect(
      getCapsuleRecoveryPosition(
        { x: 5200, y: 1200, width: 336, height: 90 },
        monitors,
        monitors[0],
        logicalCapsuleSize,
      ),
    ).toEqual({ x: 738, y: 476 })
  })
})

function HookHarness() {
  const frame = useCapsuleResize()
  return createElement('div', { 'data-testid': 'capsule-frame', 'data-visible': frame.visible })
}

describe('useCapsuleResize async updates', () => {
  const monitor = {
    position: { x: 0, y: 0 },
    size: { width: 1920, height: 1080 },
    scaleFactor: 1,
    workArea: { position: { x: 0, y: 0 }, size: { width: 1920, height: 1040 } },
  }

  beforeEach(() => {
    useAppStore.setState((state) => ({
      pipelineState: 'idle',
      capsuleExpanded: false,
      pipelineError: null,
      contextMenuOpen: false,
      contextMenuReady: false,
      translationTargetMenuOpen: false,
      polishModeMenuOpen: false,
      config: { ...state.config, capsule_auto_hide: true },
    }))

    windowApiMocks.getCurrentWindow.mockReset().mockReturnValue({
      setFocusable: windowApiMocks.setFocusable,
      setSize: windowApiMocks.setSize,
      setPosition: windowApiMocks.setPosition,
      outerPosition: windowApiMocks.outerPosition,
      outerSize: windowApiMocks.outerSize,
      scaleFactor: windowApiMocks.scaleFactor,
      show: windowApiMocks.show,
      hide: windowApiMocks.hide,
    })
    windowApiMocks.availableMonitors.mockReset().mockResolvedValue([monitor])
    windowApiMocks.currentMonitor.mockReset().mockResolvedValue(monitor)
    windowApiMocks.primaryMonitor.mockReset().mockResolvedValue(monitor)
    windowApiMocks.setFocusable.mockReset().mockResolvedValue(undefined)
    windowApiMocks.setSize.mockReset().mockResolvedValue(undefined)
    windowApiMocks.setPosition.mockReset().mockResolvedValue(undefined)
    windowApiMocks.outerPosition.mockReset().mockResolvedValue({ x: 0, y: 0 })
    windowApiMocks.outerSize.mockReset().mockResolvedValue({ width: 60, height: 60 })
    windowApiMocks.scaleFactor.mockReset().mockResolvedValue(1)
    windowApiMocks.show.mockReset().mockResolvedValue(undefined)
    windowApiMocks.hide.mockReset().mockResolvedValue(undefined)
    windowApiMocks.invoke.mockReset().mockResolvedValue(undefined)
  })

  afterEach(() => {
    cleanup()
  })

  it('raises the capsule after showing it on every recording cycle without taking focus', async () => {
    render(createElement(HookHarness))
    await waitFor(() => expect(windowApiMocks.hide).toHaveBeenCalledTimes(1))
    expect(windowApiMocks.invoke).not.toHaveBeenCalled()

    for (let cycle = 0; cycle < 2; cycle++) {
      act(() => useAppStore.setState({ pipelineState: 'recording' }))
      await waitFor(() => expect(windowApiMocks.invoke).toHaveBeenLastCalledWith('raise_capsule'))
      expect(windowApiMocks.invoke).toHaveBeenLastCalledWith('raise_capsule')
      expect(windowApiMocks.show.mock.invocationCallOrder[cycle]).toBeLessThan(
        windowApiMocks.invoke.mock.invocationCallOrder[cycle * 2 + 1],
      )
      expect(windowApiMocks.setFocusable).toHaveBeenLastCalledWith(false)
      act(() => useAppStore.setState({ pipelineState: 'idle' }))
      await waitFor(() => expect(windowApiMocks.hide).toHaveBeenCalledTimes(cycle + 2))
    }
  })

  it('gives the error shell the same native window size as recording', async () => {
    useAppStore.setState({ pipelineError: 'Capture failed' })
    render(createElement(HookHarness))
    await waitFor(() => expect(windowApiMocks.show).toHaveBeenCalled())
    expect(windowApiMocks.setSize).toHaveBeenLastCalledWith(
      expect.objectContaining({ width: 584, height: 136 }),
    )
    expect(windowApiMocks.setPosition).toHaveBeenLastCalledWith(
      expect.objectContaining({ x: 668, y: 452 }),
    )
  })

  it('opens and closes mode selection without moving or resizing an active capsule', async () => {
    useAppStore.setState({ pipelineState: 'recording' })
    render(createElement(HookHarness))
    await waitFor(() =>
      expect(screen.getByTestId('capsule-frame')).toHaveAttribute('data-visible', 'true'),
    )
    windowApiMocks.setSize.mockClear()
    windowApiMocks.setPosition.mockClear()
    windowApiMocks.show.mockClear()
    act(() => useAppStore.setState({ polishModeMenuOpen: true }))
    await waitFor(() => expect(windowApiMocks.setFocusable).toHaveBeenLastCalledWith(true))
    await waitFor(() => expect(windowApiMocks.show).toHaveBeenCalledTimes(1))
    expect(windowApiMocks.setSize).not.toHaveBeenCalled()
    expect(windowApiMocks.setPosition).not.toHaveBeenCalled()
    act(() => useAppStore.setState({ polishModeMenuOpen: false }))
    await waitFor(() => expect(windowApiMocks.show).toHaveBeenCalledTimes(2))
    expect(windowApiMocks.setSize).not.toHaveBeenCalled()
    expect(windowApiMocks.setPosition).not.toHaveBeenCalled()
    expect(windowApiMocks.setFocusable).toHaveBeenLastCalledWith(false)
  })

  it('keeps a user-dragged position when mode selection is quickly opened and closed', async () => {
    useAppStore.setState({ pipelineState: 'recording' })
    render(createElement(HookHarness))
    await waitFor(() =>
      expect(screen.getByTestId('capsule-frame')).toHaveAttribute('data-visible', 'true'),
    )
    windowApiMocks.outerPosition.mockResolvedValue({ x: -1700, y: 500 })
    windowApiMocks.setPosition.mockClear()
    windowApiMocks.setSize.mockClear()
    windowApiMocks.show.mockClear()
    act(() => useAppStore.setState({ polishModeMenuOpen: true }))
    await waitFor(() => expect(windowApiMocks.show).toHaveBeenCalledTimes(1))
    act(() => useAppStore.setState({ polishModeMenuOpen: false }))
    await waitFor(() => expect(windowApiMocks.show).toHaveBeenCalledTimes(2))
    expect(windowApiMocks.setPosition).not.toHaveBeenCalled()
    expect(windowApiMocks.setSize).not.toHaveBeenCalled()
  })

  it('shows all mode choices in a full-size frame when opened from the idle icon', async () => {
    useAppStore.setState({ pipelineState: 'idle', polishModeMenuOpen: true })
    render(createElement(HookHarness))
    await waitFor(() =>
      expect(screen.getByTestId('capsule-frame')).toHaveAttribute('data-visible', 'true'),
    )
    expect(windowApiMocks.setSize).toHaveBeenLastCalledWith(
      expect.objectContaining({ width: 584, height: 136 }),
    )
  })

  it('starts the entrance only after native show and keeps the full frame until exit finishes', async () => {
    let finishShow: () => void = () => {}
    windowApiMocks.show.mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          finishShow = resolve
        }),
    )
    useAppStore.setState({ pipelineState: 'recording' })
    render(createElement(HookHarness))
    await waitFor(() => expect(windowApiMocks.show).toHaveBeenCalled())
    expect(screen.getByTestId('capsule-frame')).toHaveAttribute('data-visible', 'false')
    await act(async () => finishShow())
    await waitFor(() =>
      expect(screen.getByTestId('capsule-frame')).toHaveAttribute('data-visible', 'true'),
    )
    const sizeCalls = windowApiMocks.setSize.mock.calls.length
    act(() => useAppStore.setState({ pipelineState: 'idle' }))
    expect(screen.getByTestId('capsule-frame')).toHaveAttribute('data-visible', 'false')
    expect(windowApiMocks.hide).not.toHaveBeenCalled()
    expect(windowApiMocks.setSize).toHaveBeenCalledTimes(sizeCalls)
    await waitFor(() => expect(windowApiMocks.hide).toHaveBeenCalledTimes(1))
    expect(windowApiMocks.setSize).toHaveBeenCalledTimes(sizeCalls)
  })

  it('does not let an old exit hide a new recording', async () => {
    useAppStore.setState({ pipelineState: 'recording' })
    render(createElement(HookHarness))
    await waitFor(() =>
      expect(screen.getByTestId('capsule-frame')).toHaveAttribute('data-visible', 'true'),
    )
    await act(async () => {
      useAppStore.setState({ pipelineState: 'idle' })
      await Promise.resolve()
    })
    act(() => useAppStore.setState({ pipelineState: 'preparing' }))
    await waitFor(() => expect(windowApiMocks.show).toHaveBeenCalledTimes(2))
    await new Promise((resolve) => setTimeout(resolve, 180))
    expect(windowApiMocks.hide).not.toHaveBeenCalled()
    expect(screen.getByTestId('capsule-frame')).toHaveAttribute('data-visible', 'true')
  })

  it('reasserts stacking during recording and stops when the capsule becomes idle', async () => {
    useAppStore.setState({ pipelineState: 'recording' })
    render(createElement(HookHarness))
    await waitFor(() => expect(windowApiMocks.invoke).toHaveBeenCalled())
    const initialCalls = windowApiMocks.invoke.mock.calls.length
    await waitFor(
      () => expect(windowApiMocks.invoke.mock.calls.length).toBeGreaterThan(initialCalls),
      {
        timeout: 1800,
      },
    )
    act(() => useAppStore.setState({ pipelineState: 'idle' }))
    await waitFor(() => expect(windowApiMocks.hide).toHaveBeenCalled())
    const stoppedCalls = windowApiMocks.invoke.mock.calls.length
    await new Promise((resolve) => setTimeout(resolve, 1100))
    expect(windowApiMocks.invoke).toHaveBeenCalledTimes(stoppedCalls)
  })

  it('prevents a superseded async resize from overwriting the latest window state', async () => {
    render(createElement(HookHarness))

    await waitFor(() => {
      expect(windowApiMocks.hide).toHaveBeenCalledTimes(1)
    })

    windowApiMocks.currentMonitor.mockReset()
    windowApiMocks.setSize.mockClear()
    windowApiMocks.setPosition.mockClear()
    windowApiMocks.show.mockClear()
    windowApiMocks.hide.mockClear()

    let resolveFirstMonitor: (value: typeof monitor) => void = () => {}
    const delayedFirstMonitor = new Promise<typeof monitor>((resolve) => {
      resolveFirstMonitor = resolve
    })
    windowApiMocks.currentMonitor
      .mockReset()
      .mockImplementationOnce(() => delayedFirstMonitor)
      .mockResolvedValue(monitor)

    act(() => {
      useAppStore.setState({ contextMenuOpen: true })
    })

    await waitFor(() => {
      expect(windowApiMocks.currentMonitor).toHaveBeenCalledTimes(1)
    })

    act(() => {
      useAppStore.setState({ contextMenuOpen: false })
    })

    await waitFor(() => {
      expect(windowApiMocks.setPosition).toHaveBeenCalledTimes(1)
      expect(windowApiMocks.hide).toHaveBeenCalledTimes(1)
    })

    await act(async () => {
      resolveFirstMonitor(monitor)
      await delayedFirstMonitor
    })

    expect(windowApiMocks.setSize.mock.calls.map(([size]) => [size.width, size.height])).toEqual([
      [60, 60],
    ])
    expect(windowApiMocks.setPosition).toHaveBeenCalledTimes(1)
    expect(windowApiMocks.show).not.toHaveBeenCalled()
    expect(windowApiMocks.hide).toHaveBeenCalledTimes(1)
    expect(useAppStore.getState().contextMenuReady).toBe(false)

    windowApiMocks.outerPosition.mockResolvedValue({ x: 100, y: 100 })
    windowApiMocks.outerSize.mockResolvedValue(null)
    windowApiMocks.setPosition.mockClear()

    act(() => {
      useAppStore.setState({ pipelineState: 'recording' })
    })

    await waitFor(() => {
      expect(windowApiMocks.setPosition).toHaveBeenCalledWith(
        expect.objectContaining({ x: 668, y: 452 }),
      )
    })
  })

  it('starts every recording on the primary monitor but preserves a drag during that session', async () => {
    const secondary = {
      ...monitor,
      position: { x: -2560, y: 0 },
      scaleFactor: 1.5,
      workArea: { position: { x: -2560, y: 0 }, size: { width: 2560, height: 1400 } },
    }
    windowApiMocks.currentMonitor.mockResolvedValue(secondary)
    windowApiMocks.availableMonitors.mockResolvedValue([secondary, monitor])
    render(createElement(HookHarness))
    await waitFor(() => expect(windowApiMocks.hide).toHaveBeenCalledTimes(1))
    expect(windowApiMocks.setPosition).toHaveBeenLastCalledWith(
      expect.objectContaining({ x: 930, y: 490 }),
    )

    act(() => useAppStore.setState({ pipelineState: 'preparing' }))
    await waitFor(() => expect(windowApiMocks.show).toHaveBeenCalledTimes(1))
    expect(windowApiMocks.setSize).toHaveBeenLastCalledWith(
      expect.objectContaining({ width: 584, height: 136 }),
    )
    expect(windowApiMocks.setPosition).toHaveBeenLastCalledWith(
      expect.objectContaining({ x: 668, y: 452 }),
    )
    const positionCalls = windowApiMocks.setPosition.mock.invocationCallOrder
    const sizeCalls = windowApiMocks.setSize.mock.invocationCallOrder
    expect(positionCalls[positionCalls.length - 1]).toBeGreaterThan(sizeCalls[sizeCalls.length - 1])

    windowApiMocks.outerPosition.mockResolvedValue({ x: -1700, y: 500 })
    windowApiMocks.outerSize.mockResolvedValue({ width: 876, height: 204 })
    act(() => useAppStore.setState({ pipelineState: 'recording' }))
    await waitFor(() => expect(windowApiMocks.show).toHaveBeenCalledTimes(2))
    expect(windowApiMocks.setPosition).toHaveBeenLastCalledWith(
      expect.objectContaining({ x: -1700, y: 500 }),
    )
    act(() => useAppStore.setState({ pipelineState: 'transcribing' }))
    await waitFor(() => expect(windowApiMocks.show).toHaveBeenCalledTimes(3))
    expect(windowApiMocks.setPosition).toHaveBeenLastCalledWith(
      expect.objectContaining({ x: -1700, y: 500 }),
    )

    act(() => useAppStore.setState({ pipelineState: 'idle' }))
    await waitFor(() => expect(windowApiMocks.hide).toHaveBeenCalledTimes(2))
    act(() => useAppStore.setState({ pipelineState: 'ask_recording' }))
    await waitFor(() => expect(windowApiMocks.show).toHaveBeenCalledTimes(4))
    expect(windowApiMocks.setPosition).toHaveBeenLastCalledWith(
      expect.objectContaining({ x: 668, y: 452 }),
    )
    expect(windowApiMocks.setFocusable).toHaveBeenLastCalledWith(false)
  })

  it('keeps a pending reset when preparing is superseded before positioning finishes', async () => {
    render(createElement(HookHarness))
    await waitFor(() => expect(windowApiMocks.hide).toHaveBeenCalled())
    let finishPrimary: (value: typeof monitor) => void = () => {}
    windowApiMocks.primaryMonitor.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finishPrimary = resolve
        }),
    )
    windowApiMocks.primaryMonitor.mockClear()
    act(() => useAppStore.setState({ pipelineState: 'preparing' }))
    await waitFor(() => expect(windowApiMocks.primaryMonitor).toHaveBeenCalledTimes(1))
    act(() => useAppStore.setState({ pipelineState: 'recording' }))
    await waitFor(() => expect(windowApiMocks.show).toHaveBeenCalledTimes(1))
    expect(windowApiMocks.setPosition).toHaveBeenLastCalledWith(
      expect.objectContaining({ x: 668, y: 452 }),
    )
    const positions = windowApiMocks.setPosition.mock.calls.length
    await act(async () => finishPrimary(monitor))
    expect(windowApiMocks.setPosition).toHaveBeenCalledTimes(positions)
  })

  it('centers on the captured input monitor rather than primary or the old capsule display', async () => {
    const external = {
      ...monitor,
      position: { x: -2560, y: -300 },
      size: { width: 2560, height: 1440 },
      workArea: { position: { x: -2560, y: -280 }, size: { width: 2560, height: 1400 } },
    }
    windowApiMocks.availableMonitors.mockResolvedValue([monitor, external])
    windowApiMocks.invoke.mockImplementation(async (command: string) =>
      command === 'get_recording_monitor' ? external : undefined,
    )
    useAppStore.setState({ pipelineState: 'preparing' })
    render(createElement(HookHarness))
    await waitFor(() => expect(windowApiMocks.show).toHaveBeenCalled())
    expect(windowApiMocks.setPosition).toHaveBeenLastCalledWith(
      expect.objectContaining({ x: -1572, y: 352 }),
    )
    expect(windowApiMocks.primaryMonitor).not.toHaveBeenCalled()
  })

  it('uses logical macOS coordinates when moving from Retina to an external screen', async () => {
    vi.spyOn(window.navigator, 'platform', 'get').mockReturnValue('MacIntel')
    const external = {
      ...monitor,
      position: { x: -5120, y: 0 },
      size: { width: 5120, height: 2880 },
      scaleFactor: 2,
      workArea: { position: { x: -5120, y: 40 }, size: { width: 5120, height: 2800 } },
    }
    windowApiMocks.availableMonitors.mockResolvedValue([monitor, external])
    windowApiMocks.invoke.mockImplementation(async (command: string) =>
      command === 'get_recording_monitor' ? external : undefined,
    )
    useAppStore.setState({ pipelineState: 'recording' })
    render(createElement(HookHarness))
    await waitFor(() => expect(windowApiMocks.show).toHaveBeenCalled())
    expect(windowApiMocks.setPosition).toHaveBeenLastCalledWith(
      expect.objectContaining({ x: -1572, y: 652 }),
    )
    vi.restoreAllMocks()
  })

  it('rejects an unplugged captured display before falling back to the available primary', () => {
    expect(
      getRecordingTargetMonitor({ ...monitor, position: { x: -1920, y: 0 } }, [monitor]),
    ).toBeUndefined()
    expect(getRecordingTargetMonitor({ ...monitor }, [monitor])).toBe(monitor)
  })
})
