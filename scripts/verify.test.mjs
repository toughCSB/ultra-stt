import assert from 'node:assert/strict'
import { test } from 'node:test'
import { auditEnvironment, executePlan, verificationPlan } from './verify.mjs'

test('default verification includes formatting, type, lint, frontend and native tests, and audit', () => {
  const commands = verificationPlan().map((step) => `${step.command} ${step.args.join(' ')}`)
  for (const command of [
    'npm run format:check',
    'npm run typecheck',
    'npm run lint',
    'npm test',
    'npm audit --audit-level=moderate',
  ]) {
    assert.ok(commands.includes(command), command)
  }
  assert.ok(commands.some((command) => command.startsWith('cargo fmt --check')))
  assert.ok(
    commands.some(
      (command) => command.startsWith('cargo clippy --locked') && command.endsWith('-D warnings'),
    ),
  )
  assert.ok(commands.some((command) => command.startsWith('cargo test --locked')))
  assert.ok(
    commands.findIndex((command) => command.includes('--example process_fixture')) <
      commands.findIndex((command) => command.startsWith('cargo test')),
  )
})

test('CI native targets propagate to clippy, fixture build and tests but not rustfmt', () => {
  const plan = verificationPlan(['--native', '--target', 'aarch64-apple-darwin'])
  for (const step of plan.filter((step) => step.command === 'cargo')) {
    assert.equal(step.args.includes('aarch64-apple-darwin'), step.args[0] !== 'fmt')
  }
  assert.ok(!plan.some((step) => step.args[0] === 'audit'))
})

test('frontend scope includes the verification script regression tests', () => {
  const plan = verificationPlan(['--frontend'])
  assert.ok(plan.every((step) => step.command === 'npm'))
  assert.ok(plan.some((step) => step.args.includes('test:verification')))
})

test('invalid options fail instead of silently skipping checks', () => {
  for (const options of [
    ['--skip-tests'],
    ['--native', '--frontend'],
    ['--target'],
    ['--target', '--offline'],
    ['--frontend', '--target', 'x86_64-pc-windows-msvc'],
  ]) {
    assert.throws(() => verificationPlan(options))
  }
})

test('any failed check prevents later steps and returns its status', () => {
  const visited = []
  const result = executePlan(
    [{ command: 'first' }, { command: 'failed' }, { command: 'last' }],
    (step) => {
      visited.push(step.command)
      return { status: step.command === 'failed' ? 17 : 0 }
    },
  )
  assert.equal(result, 17)
  assert.deepEqual(visited, ['first', 'failed'])
})

test('missing tools and terminated commands cannot pass verification', () => {
  assert.throws(
    () => executePlan([{}], () => ({ error: new Error('missing tool') })),
    /missing tool/,
  )
  assert.equal(
    executePlan([{}], () => ({ status: null, signal: 'SIGTERM' })),
    1,
  )
})

test('audit does not inherit npm lifecycle allow-scripts as a CLI permission', () => {
  const original = { npm_config_allow_scripts: 'example', PATH: 'original-path' }
  const environment = auditEnvironment(original)
  assert.equal(environment.npm_config_allow_scripts, undefined)
  assert.equal(environment.npm_config_ignore_scripts, 'true')
  assert.equal(environment.PATH, 'original-path')
  assert.equal(original.npm_config_allow_scripts, 'example')
})
