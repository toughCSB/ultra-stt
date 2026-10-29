import { AnimatePresence, motion } from 'framer-motion'
import { useTranslation } from 'react-i18next'
import { useAppStore } from '../../stores/appStore'
import { saveOnboardingCompleted, updateConfig as saveConfig } from '../../lib/tauri'
import { OnboardingLayout } from './OnboardingLayout'
import { WelcomeStep } from './WelcomeStep'
import { SttSetupStep } from './SttSetupStep'
import { LlmSetupStep } from './LlmSetupStep'
import { PermissionsStep } from './PermissionsStep'
import { QuickTestStep } from './QuickTestStep'
import { DoneStep } from './DoneStep'
import { slideRight } from '../../lib/animations'

const TOTAL_STEPS = 6

export function Onboarding() {
  const { t } = useTranslation()
  const step = useAppStore((s) => s.onboardingStep)
  const setStep = useAppStore((s) => s.setOnboardingStep)
  const setOnboardingCompleted = useAppStore((s) => s.setOnboardingCompleted)
  const config = useAppStore((s) => s.config)

  const canNext = (() => {
    switch (step) {
      case 0:
        return true // Welcome — always
      case 1:
      case 2:
        return true
      case 3:
      case 4:
      case 5:
        return true // Permissions, quick test and done are optional
      default:
        return false
    }
  })()

  const titles = [
    { title: t('onboarding.steps.welcome'), subtitle: t('onboarding.steps.welcomeSub') },
    {
      title: t('settings.engineSelection'),
      subtitle: t('settings.engine.onboardingNote'),
    },
    { title: t('onboarding.steps.aiPolish'), subtitle: t('onboarding.steps.aiPolishSub') },
    { title: t('onboarding.steps.permissions'), subtitle: t('onboarding.steps.permissionsSub') },
    { title: t('onboarding.steps.howItWorks'), subtitle: t('onboarding.steps.howItWorksSub') },
    { title: t('onboarding.steps.setupComplete'), subtitle: undefined },
  ]

  const persist = async () => {
    try {
      await saveConfig(config)
    } catch {
      // Best-effort save — continue navigation even if save fails
    }
  }

  const handleNext = async () => {
    if (step < TOTAL_STEPS - 1) {
      await persist()
      setStep(step + 1)
    } else {
      await persist()
      await saveOnboardingCompleted()
      setOnboardingCompleted(true)
    }
  }

  const handleBack = async () => {
    if (step > 0) {
      await persist()
      setStep(step - 1)
    }
  }

  const handleSkip = async () => {
    try {
      await saveConfig(config)
      await saveOnboardingCompleted()
    } catch {
      // Best-effort save — still let the user continue into the app.
    }
    setOnboardingCompleted(true)
  }

  return (
    <OnboardingLayout
      step={step}
      totalSteps={TOTAL_STEPS}
      title={titles[step].title}
      subtitle={titles[step].subtitle}
      canNext={canNext}
      canBack={step > 0}
      nextLabel={
        step === TOTAL_STEPS - 1 ? t('onboarding.steps.getStarted') : t('onboarding.layout.next')
      }
      onNext={handleNext}
      onBack={handleBack}
      onSkip={handleSkip}
    >
      <AnimatePresence mode="wait">
        <motion.div
          key={step}
          variants={slideRight}
          initial="initial"
          animate="animate"
          exit="exit"
          transition={{ duration: 0.2 }}
        >
          {step === 0 && <WelcomeStep />}
          {step === 1 && <SttSetupStep />}
          {step === 2 && <LlmSetupStep />}
          {step === 3 && <PermissionsStep />}
          {step === 4 && <QuickTestStep />}
          {step === 5 && <DoneStep />}
        </motion.div>
      </AnimatePresence>
    </OnboardingLayout>
  )
}
