export function encodeWav(chunks, rate) {
  const frames = chunks.reduce((total, chunk) => total + chunk.length, 0)
  const samples = new Float32Array(frames)
  let offset = 0
  for (const chunk of chunks) {
    samples.set(chunk, offset)
    offset += chunk.length
  }
  const ratio = rate / 16000
  const length = Math.floor(samples.length / ratio)
  const wav = new ArrayBuffer(44 + length * 2)
  const view = new DataView(wav)
  const write = (at, word) => {
    for (let i = 0; i < word.length; i++) view.setUint8(at + i, word.charCodeAt(i))
  }
  write(0, 'RIFF')
  view.setUint32(4, wav.byteLength - 8, true)
  write(8, 'WAVEfmt ')
  view.setUint32(16, 16, true)
  view.setUint16(20, 1, true)
  view.setUint16(22, 1, true)
  view.setUint32(24, 16000, true)
  view.setUint32(28, 32000, true)
  view.setUint16(32, 2, true)
  view.setUint16(34, 16, true)
  write(36, 'data')
  view.setUint32(40, length * 2, true)
  for (let i = 0; i < length; i++) {
    const begin = Math.floor(i * ratio)
    const end = Math.max(begin + 1, Math.floor((i + 1) * ratio))
    let sum = 0
    for (let j = begin; j < end; j++) sum += samples[j] ?? 0
    const sample = Math.max(-1, Math.min(1, sum / (end - begin)))
    view.setInt16(44 + i * 2, sample < 0 ? sample * 32768 : sample * 32767, true)
  }
  return wav
}
