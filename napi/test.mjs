import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { test } from 'node:test'
import { lint } from '@outslept/stx'

const config = { rules: { 'color-hex-length': 'short' } }
const code = 'a { color: #FFFFFF; }'

test('loads from ESM and CommonJS', () => {
  assert.equal(createRequire(import.meta.url)('@outslept/stx').lint, lint)
  assert.equal(lint('a {}').warnings[0].rule, 'block-no-empty')
})

test('reports diagnostics and fixes CSS in memory', () => {
  assert.deepEqual(lint(code, { config }), {
    code,
    errored: true,
    warnings: [{
      line: 1,
      column: 12,
      endLine: 1,
      endColumn: 19,
      rule: 'color-hex-length',
      severity: 'error',
      text: 'Expected "#FFFFFF" to be "#FFF" (color-hex-length)',
    }],
  })
  assert.deepEqual(lint(code, { config, fix: true }), {
    code: 'a { color: #FFF; }',
    errored: false,
    warnings: [],
  })
})

test('keeps the BOM and non-ASCII text when fixing', () => {
  const source = '\uFEFF.😀 { color: /* 日本語 */ #FFFFFF; }'
  assert.equal(lint(source, { config, fix: true }).code, source.replace('#FFFFFF', '#FFF'))
})

test('uses filename for syntax and rule overrides', () => {
  const source = '// stylelint-disable-next-line color-hex-length\n' + code
  assert.deepEqual(lint(source, { filename: 'src/style.scss', config }).warnings, [])
  assert.equal(lint('a\n  color: #FFFFFF', { syntax: 'sass', config }).warnings[0].rule, 'color-hex-length')
  assert.deepEqual(lint(code, {
    filename: 'src/style.css',
    config: { ...config, overrides: [{ files: 'src/*.css', rules: { 'color-hex-length': null } }] },
  }).warnings, [])
})

test('reports syntax errors and rejects bad configuration', () => {
  assert.equal(lint(', a {}', { config }).warnings[0].rule, 'CssSyntaxError')
  assert.throws(() => lint(code, { config: { rules: { 'not-a-rule': true } } }), /Unknown rule/)
  assert.throws(() => lint(code, { config: [] }), /config must be an object/)
  assert.throws(() => lint(code, { syntax: 'html' }))
})

const cliPath = join(dirname(createRequire(import.meta.url).resolve('@outslept/stx')), 'cli.cjs')

function cliProject(t, files) {
  const cwd = realpathSync(mkdtempSync(join(tmpdir(), 'stx-cli-')))
  t.after(() => rmSync(cwd, { recursive: true, force: true }))
  for (const [name, contents] of Object.entries(files)) {
    const path = join(cwd, name)
    mkdirSync(dirname(path), { recursive: true })
    writeFileSync(path, contents)
  }
  return {
    cwd,
    read: (name) => readFileSync(join(cwd, name), 'utf8'),
    run(args = [], input) {
      const child = spawnSync(process.execPath, [cliPath, ...args], { cwd, input, encoding: 'utf8', timeout: 10_000 })
      assert.ifError(child.error)
      return child
    },
  }
}

test('CLI expands quoted globs, subtracts exclusions, and finds the nearest JSON config', (t) => {
  const project = cliProject(t, {
    '.stylelintrc.json': JSON.stringify(config),
    '.stylelintignore': 'vendor/\n',
    '.hidden/style.css': code,
    'node_modules/dep/style.css': code,
    'docker/style.css': code,
    'vendor/style.css': code,
    'top.css': code,
    'src/.stylelintrc.json': JSON.stringify({ rules: { 'color-hex-length': 'long' } }),
    'src/style.scss': code,
    'src/style.sass': 'a\n  color: #FFFFFF\n',
  })
  const result = project.run(['top.css', '**/*.{css,scss,sass}', '!**/docker/**', 'docker/style.css', '-f', 'json'])
  assert.equal(result.status, 2, result.stderr)
  assert.equal(result.stdout, '')
  const reports = JSON.parse(result.stderr)
  assert.deepEqual(reports.map(({ source }) => source), ['src/style.sass', 'src/style.scss', 'top.css'].map((name) => join(project.cwd, name)))
  assert.deepEqual(reports.map(({ errored }) => errored), [false, false, true])
  const top = project.run(['*.css', '--formatter=json'])
  assert.deepEqual(JSON.parse(top.stderr).map(({ source }) => source), [join(project.cwd, 'top.css')])
})

test('CLI fixes files in place and can save the stderr report', (t) => {
  const project = cliProject(t, {
    'explicit.json': JSON.stringify({ ...config, ignoreFiles: ['ignored.css'] }),
    '.stylelintrc.json': JSON.stringify({ rules: {} }),
    'style.css': code,
    'ignored.css': code,
  })
  const result = project.run(['*.css', '--config', 'explicit.json', '--fix', '-f', 'json', '--output-file', 'report.json'])
  assert.equal(result.status, 0, result.stderr)
  assert.equal(result.stdout, '')
  assert.equal(project.read('style.css'), 'a { color: #FFF; }')
  assert.equal(project.read('ignored.css'), code)
  assert.equal(project.read('report.json'), result.stderr)
  const [ignored, fixed] = JSON.parse(result.stderr)
  assert.equal(ignored.ignored, true)
  assert.equal(fixed.autofixed, true)
})

test('CLI reads piped CSS and keeps fixed code separate from reports', (t) => {
  const project = cliProject(t, {
    'src/.stylelintrc.json': JSON.stringify(config),
  })
  const args = ['--stdin-filename', 'src/virtual.scss', '--fix', '-f', 'json']
  const result = project.run(args, `// comment\n${code}`)
  assert.equal(result.status, 0, result.stderr)
  assert.equal(result.stdout, '// comment\na { color: #FFF; }')
  const [report] = JSON.parse(result.stderr)
  assert.equal(report.source, join(project.cwd, 'src/virtual.scss'))
  assert.deepEqual(report.warnings, [])
  const empty = project.run([...args, '--stdin'], '')
  assert.equal(empty.status, 0, empty.stderr)
  assert.equal(empty.stdout, '')
  const invalid = project.run(args, ', a {}')
  assert.equal(invalid.status, 2, invalid.stderr)
  assert.equal(invalid.stdout, '')
  assert.equal(JSON.parse(invalid.stderr)[0].warnings[0].rule, 'CssSyntaxError')
})

test('CLI distinguishes usage, config, and lint failures', (t) => {
  const project = cliProject(t, {
    'style.css': code,
    'invalid.json': '{',
    'src/style.css': code,
    'src/stylelint.config.mjs': 'export default {}',
    '.stylelintrc.json': JSON.stringify({ ...config, defaultSeverity: 'warning' }),
  })
  assert.equal(project.run(['absent/**/*.css']).status, 1)
  const empty = project.run(['absent/**/*.css', '--allow-empty-input', '-f', 'json'])
  assert.equal(empty.status, 0, empty.stderr)
  assert.deepEqual(JSON.parse(empty.stderr), [])
  assert.equal(project.run(['--wrong-option']).status, 64)
  assert.equal(project.run(['style.css', '--config', 'invalid.json']).status, 78)
  const unsupported = project.run(['src/style.css'])
  assert.equal(unsupported.status, 78)
  assert.match(unsupported.stderr, /use JSON/)
  const budget = project.run(['style.css', '--max-warnings', '0'])
  assert.equal(budget.status, 2)
  assert.match(budget.stderr, /Max warnings exceeded/)
  const quiet = project.run(['style.css', '--quiet'])
  assert.equal(quiet.status, 0, quiet.stderr)
  assert.equal(quiet.stderr, '')
})
