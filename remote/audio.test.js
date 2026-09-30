import { expect, it } from 'vitest'
import { encodeWav } from './audio.js'

it('encodes resampled mono PCM16 with a matching canonical WAV length', () => {
  const wav = encodeWav([new Float32Array([0, 1]), new Float32Array([-1, 0.5])], 32000)
  const view = new DataView(wav)
  expect(wav.byteLength).toBe(48)
  expect(view.getUint32(4, true)).toBe(wav.byteLength - 8)
  expect(view.getUint16(20, true)).toBe(1)
  expect(view.getUint16(22, true)).toBe(1)
  expect(view.getUint32(24, true)).toBe(16000)
  expect(view.getUint16(34, true)).toBe(16)
  expect(view.getUint32(40, true)).toBe(4)
  expect(view.getInt16(44, true)).toBe(16383)
  expect(view.getInt16(46, true)).toBe(-8192)
})
