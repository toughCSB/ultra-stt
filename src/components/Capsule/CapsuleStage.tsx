import type { ReactNode } from 'react'
import { X } from 'lucide-react'
import { CapsuleBrand } from './CapsuleWorkIndicator'

interface CapsuleStageProps {
  status: ReactNode
  detail: ReactNode
  actions?: ReactNode
  timer?: ReactNode
  tone?: 'recording' | 'live' | 'error' | 'complete' | 'working'
}

export function CapsuleCancelButton({
  label,
  onCancel,
}: {
  label: string
  onCancel: () => Promise<unknown>
}) {
  return (
    <button
      type="button"
      aria-label={label}
      className="capsule-action capsule-action-cancel"
      onPointerDown={(event) => event.stopPropagation()}
      onPointerUp={(event) => event.stopPropagation()}
      onClick={(event) => {
        event.stopPropagation()
        void onCancel().catch((error) => console.error('Failed to cancel voice input:', error))
      }}
    >
      <X size={20} strokeWidth={2.2} />
    </button>
  )
}

export function CapsuleStage({
  status,
  detail,
  actions,
  timer,
  tone = 'working',
}: CapsuleStageProps) {
  return (
    <div className={`capsule-stage capsule-stage-${tone}`}>
      <CapsuleBrand size={64} />
      <div className="capsule-stage-main">
        <div className="capsule-stage-heading">
          <div className="capsule-stage-status">{status}</div>
          {timer && <div className="capsule-stage-timer">{timer}</div>}
        </div>
        <div className="capsule-stage-detail">{detail}</div>
      </div>
      {actions && <div className="capsule-stage-actions">{actions}</div>}
    </div>
  )
}
