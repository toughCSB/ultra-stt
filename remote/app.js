import { encodeWav } from './audio.js'

const elements = Object.fromEntries(
  ['record', 'stop', 'cancel', 'copy', 'share', 'status', 'result', 'raw', 'polished'].map(
    (id) => [id, document.getElementById(id)],
  ),
)

let stream
let context
let source
let processor
let socket
let chunks = []
let frames = 0
let rate = 48000
let state = 'idle'

function showState(next, message) {
  state = next
  document.body.dataset.state = next
  elements.status.textContent = message
  elements.record.disabled = next !== 'idle' && next !== 'done' && next !== 'error'
  elements.stop.disabled = next !== 'recording'
  elements.cancel.disabled = next !== 'recording' && next !== 'processing'
}

async function releaseMicrophone() {
  processor?.disconnect()
  source?.disconnect()
  stream?.getTracks().forEach((track) => track.stop())
  if (context && context.state !== 'closed') await context.close()
  processor = undefined
  source = undefined
  stream = undefined
  context = undefined
}

async function start() {
  elements.result.hidden = true
  chunks = []
  frames = 0
  showState('permission', '마이크 권한을 기다리고 있습니다.')
  try {
    stream = await navigator.mediaDevices.getUserMedia({
      audio: { channelCount: 1, echoCancellation: true, noiseSuppression: true },
    })
    context = new AudioContext()
    rate = context.sampleRate
    await context.audioWorklet.addModule('./capture-worklet.js')
    source = context.createMediaStreamSource(stream)
    processor = new AudioWorkletNode(context, 'ultra-capture')
    processor.port.onmessage = ({ data }) => {
      if (state !== 'recording') return
      chunks.push(data)
      frames += data.length
      if (frames >= rate * 120) stop()
    }
    source.connect(processor)
    processor.connect(context.destination)
    await context.resume()
    showState('recording', '녹음 중입니다. 다 말한 뒤 완료하고 정리를 누르세요.')
  } catch (error) {
    await releaseMicrophone()
    showState(
      'error',
      error?.name === 'NotAllowedError'
        ? '마이크 권한을 허용한 뒤 다시 시도하세요.'
        : '마이크를 사용할 수 없습니다. 기기와 브라우저 설정을 확인하세요.',
    )
  }
}

async function stop() {
  if (state !== 'recording') return
  showState('processing', '서버에서 음성을 인식하고 문장을 정리하고 있습니다.')
  await releaseMicrophone()
  if (frames < rate / 4) {
    showState('error', '음성이 너무 짧습니다. 다시 녹음해 주세요.')
    return
  }
  const wav = encodeWav(chunks, rate)
  chunks = []
  frames = 0
  socket = new WebSocket(`wss://${location.host}/ws`)
  socket.addEventListener('open', () => {
    if (state === 'processing') socket.send(wav)
  })
  socket.addEventListener('message', ({ data }) => {
    if (state !== 'processing') return
    try {
      const result = JSON.parse(data)
      if (result.type !== 'result' || !result.text) {
        showState('error', result.message || '문장을 정리하지 못했습니다. 다시 시도하세요.')
        return
      }
      elements.raw.textContent = result.raw
      elements.polished.textContent = result.text
      elements.result.hidden = false
      showState('done', '문장 정리가 완료됐습니다. 복사하거나 다른 앱으로 공유하세요.')
      socket.close()
    } catch {
      showState('error', '서버 응답을 읽지 못했습니다. 다시 시도하세요.')
    }
  })
  socket.addEventListener('close', () => {
    if (state === 'processing') {
      showState('error', '서버에 연결할 수 없습니다. 서버 PC와 Tailscale 연결을 확인하세요.')
    }
  })
}

async function cancel() {
  if (state === 'processing' && socket?.readyState === WebSocket.OPEN) socket.send('cancel')
  socket?.close()
  showState('idle', '이번 녹음을 취소했습니다. 다시 시작할 수 있습니다.')
  await releaseMicrophone()
  chunks = []
  frames = 0
}

async function copy() {
  try {
    await navigator.clipboard.writeText(elements.polished.textContent)
    showState('done', '문장을 복사했습니다. 원하는 앱에 붙여 넣으세요.')
  } catch {
    showState('done', '복사 권한을 허용한 뒤 다시 누르세요.')
  }
}

elements.record.addEventListener('click', start)
elements.stop.addEventListener('click', stop)
elements.cancel.addEventListener('click', cancel)
elements.copy.addEventListener('click', copy)
elements.share.addEventListener('click', async () => {
  if (navigator.share) {
    try {
      await navigator.share({ text: elements.polished.textContent })
    } catch (error) {
      if (error?.name !== 'AbortError') showState('done', '공유할 수 없습니다. 문장 복사를 이용하세요.')
    }
  } else {
    await copy()
  }
})
