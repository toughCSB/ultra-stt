import { execFileSync } from 'node:child_process'
import { chmodSync, existsSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const git = (args) =>
  execFileSync('git', args, { cwd: root, encoding: 'utf8', windowsHide: true }).trim()
let configured = ''
try {
  configured = git(['config', '--get', 'core.hooksPath'])
} catch (error) {
  if (error.status !== 1) throw error
}
const previousHook = path.resolve(root, git(['rev-parse', '--git-path', 'hooks/pre-push']))
if ((configured && configured !== '.githooks') || (!configured && existsSync(previousHook))) {
  throw new Error(
    'An existing hook configuration was found; integrate it manually instead of overwriting it.',
  )
}
if (process.platform !== 'win32') chmodSync(path.join(root, '.githooks/pre-push'), 0o755)
git(['config', '--local', 'core.hooksPath', '.githooks'])
console.log('Repository-local pre-push verification enabled. No global hooks or tools changed.')
