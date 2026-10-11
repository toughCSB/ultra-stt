import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

export function verificationPlan(argumentsList = []) {
  let scope = 'all'
  let target
  for (let index = 0; index < argumentsList.length; index++) {
    const argument = argumentsList[index]
    if (['--frontend', '--native', '--security'].includes(argument) && scope === 'all') {
      scope = argument.slice(2)
    } else if (argument === '--target' && !target) {
      target = argumentsList[++index]
      if (!target || !/^[a-z0-9_]+(?:-[a-z0-9_]+)+$/.test(target)) {
        throw new Error('Expected a Rust target triple after --target')
      }
    } else {
      throw new Error(`Unknown or conflicting argument: ${argument}`)
    }
  }
  if (target && !['all', 'native'].includes(scope)) {
    throw new Error('--target is only valid for native checks')
  }
  const steps = []
  const npm = (...args) => steps.push({ command: 'npm', args })
  const cargo = (...args) => steps.push({ command: 'cargo', args })
  if (['all', 'frontend'].includes(scope)) {
    npm('run', 'format:check')
    npm('run', 'typecheck')
    npm('run', 'lint')
    npm('run', 'test:verification')
    npm('test')
  }
  if (['all', 'native'].includes(scope)) {
    const manifest = ['--manifest-path', 'src-tauri/Cargo.toml']
    const targetArguments = target ? ['--target', target] : []
    cargo('fmt', '--check', ...manifest)
    npm('run', 'build')
    cargo('clippy', '--locked', ...targetArguments, ...manifest, '--', '-D', 'warnings')
    cargo('build', '--locked', ...targetArguments, ...manifest, '--example', 'process_fixture')
    cargo('test', '--locked', ...targetArguments, ...manifest)
  }
  if (['all', 'security'].includes(scope)) npm('audit', '--audit-level=moderate')
  return steps
}

export function executePlan(steps, runner) {
  for (const step of steps) {
    const result = runner(step)
    if (result.error) throw result.error
    if (result.status !== 0) return result.status || 1
  }
  return 0
}

export function auditEnvironment(environment) {
  const sanitized = { ...environment }
  for (const key of Object.keys(sanitized)) {
    if (key.toLowerCase() === 'npm_config_allow_scripts') delete sanitized[key]
  }
  sanitized.npm_config_ignore_scripts = 'true'
  return sanitized
}

function nativeEnvironment(root) {
  const environment = { ...process.env }
  if (process.platform !== 'win32') return environment
  const cache = path.resolve(root, '../cache/cargo')
  if (!environment.CARGO_HOME && existsSync(cache)) environment.CARGO_HOME = cache
  const locator = path.join(
    environment['ProgramFiles(x86)'] || 'C:/Program Files (x86)',
    'Microsoft Visual Studio/Installer/vswhere.exe',
  )
  if (existsSync(locator)) {
    const installation = spawnSync(
      locator,
      ['-latest', '-products', '*', '-property', 'installationPath'],
      {
        encoding: 'utf8',
        windowsHide: true,
      },
    )
    if (installation.status === 0 && installation.stdout.trim()) {
      const cmake = path.join(
        installation.stdout.trim(),
        'Common7/IDE/CommonExtensions/Microsoft/CMake/CMake/bin',
      )
      const pathKey = Object.keys(environment).find((key) => key.toLowerCase() === 'path') || 'PATH'
      if (existsSync(cmake))
        environment[pathKey] = `${cmake}${path.delimiter}${environment[pathKey] || ''}`
    }
  }
  return environment
}

function main() {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
  const environment = nativeEnvironment(root)
  const npmPath = environment.npm_execpath
  if (!npmPath) throw new Error('Run this command with npm run verify')
  return executePlan(verificationPlan(process.argv.slice(2)), ({ command, args }) => {
    console.log(`\n[verify] ${command} ${args.join(' ')}`)
    return spawnSync(
      command === 'npm' ? process.execPath : command,
      command === 'npm' ? [npmPath, ...args] : args,
      {
        cwd: root,
        env: command === 'npm' && args[0] === 'audit' ? auditEnvironment(environment) : environment,
        stdio: 'inherit',
        windowsHide: true,
      },
    )
  })
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  try {
    process.exitCode = main()
  } catch (error) {
    console.error(`[verify] ${error.message}`)
    process.exitCode = 1
  }
}
