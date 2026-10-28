import { describe, expect, it } from 'vitest'
import { errorMessage } from '../errors'

describe('errorMessage', () => {
  it('shows thrown errors and Rust command rejections the same way', () => {
    expect(errorMessage(new Error('save failed'))).toBe('save failed')
    expect(errorMessage('STT API key is not configured.')).toBe('STT API key is not configured.')
    expect(errorMessage(42)).toBe('42')
  })
})
