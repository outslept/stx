> [!WARNING]
> This project is a WIP.

# stx

CSS, SCSS, Sass, and Less linter with 154 rules, 153 of them mirroring Stylelint 18.1.0.
It ships as a single binary and as a Node-API addon, so the same engine serves
`stx` on the command line and `lint()` inside Node.

- **154 rules**, same configuration keys and message wording as the Stylelint rules they mirror.
- **Presets**: `recommended` (47 rules) and `standard` (70).
- **Autofix** for 40 rules (30 unconditional, 10 option dependent), through `stx --fix`
  or `lint(code, { fix: true })`.
- **Replay score**: 8,495 of 8,565 replayable Stylelint fixtures match. Date and commit below.
- **Reads what Stylelint reads**: `.stylelintrc*`, `stylelint.config.json`, `.stxrc*`,
  or a `stylelint` key in `package.json`.

Jump to: [install](#install) | [quick start](#quick-start) | [CLI](#command-line) |
[config](#configuration) | [JavaScript API](#javascript-api) | [current state](#current-state) |
[compatibility](#compatibility-with-stylelint) | [development](#development)

## Install

```sh
npm install --save-dev @outslept/stx     # provides the `stx` binary
```

```sh
cargo install --path . --bin stx         # from a checkout
```

Prebuilt binaries for Linux (x64, arm64), macOS (x64, arm64), and Windows (x64) are
attached to each release. The Node addon requires Node 22.14 or newer.

## Quick start

1. Install the package.
2. Create a config next to your styles:

   ```json
   {
     "extends": "stylelint-config-recommended",
     "rules": {
       "color-hex-length": "short"
     }
   }
   ```

3. Run it:

   ```sh
   npx stx "src/**/*.css"
   ```

4. Fix what can be fixed:

   ```sh
   npx stx "src/**/*.css" --fix
   ```

The report goes to stderr, so redirecting stdout does not capture it. See
[output](#output) for the reason.

## Command line

| Flag | Meaning |
| --- | --- |
| `PATH_OR_GLOB...` | files, directories, or quoted globs; prefix with `!` to exclude |
| `-c, --config <PATH>` | JSON config file to use instead of discovery |
| `-f, --fix` | apply fixes in place, or write fixed stdin to stdout |
| `--formatter <STRING\|JSON\|VERBOSE>` | report format, default `string`, alias `text` for `verbose` |
| `-q, --quiet` | only report rules with `error` severity |
| `--stdin`, `--stdin-filename <PATH>` | read from stdin; the virtual filename sets syntax and config lookup |
| `--allow-empty-input` | succeed when nothing matched |
| `-i, --ignore-path <PATH>` | read ignore patterns from this file instead of `.stylelintignore` |
| `--ignore-pattern <GLOB>` | extra ignore pattern, repeatable |
| `--disable-default-ignores` | stop skipping `node_modules` automatically |
| `--max-warnings <N>` | exit with code 2 when the warning count exceeds `N` |
| `--output-file <PATH>` | also write the report to a file |

Calling `stx` with no paths and no piped input prints the help and exits 0.

### Exit codes

| Code | Meaning |
| --- | --- |
| `0` | no problems, or `--help` / `--version` |
| `1` | runtime failure: unreadable file, failed write, `--stdin` with no pipe, no files matched |
| `2` | at least one warning has `error` severity, or `--max-warnings` was exceeded |
| `64` | invalid command line: bad arguments or an invalid glob |
| `78` | config problem: unreadable, invalid JSON, validation error, bad ignore pattern |

Stylelint uses 1 and 2 for the same situations and has no 64 or 78. Wrappers that test
zero versus non-zero are unaffected; wrappers that branch on 1 versus 2 will see config
errors as 78 instead of 2.

## Configuration

Discovery walks up from each linted file and takes the first match, in this order:
`.stylelintrc`, `.stylelintrc.json`, `stylelint.config.json`, `.stxrc`, `.stxrc.json`,
`.stylelintrc.yaml`, `.stylelintrc.yml`, `.stylelintrc.js`, `.stylelintrc.cjs`,
`.stylelintrc.mjs`, `stylelint.config.js`, `stylelint.config.cjs`, `stylelint.config.mjs`,
`stylelint.config.ts`, `stylelint.config.cts`, `stylelint.config.mts`, then a
`stylelint` key inside `package.json`.

Only JSON is read. A `.yaml`, `.js`, `.cjs`, `.mjs`, `.ts`, `.cts`, or `.mts` config is
found by discovery and then rejected with `Unsupported config format` and exit 78, which
is better than silently linting with the wrong settings but does mean a TypeScript config
project cannot use `stx` yet.

### Keys

| Key | Behaviour |
| --- | --- |
| `rules` | read; `null` disables a rule, an array gives primary and secondary options, a string sets severity |
| `extends` | read; only names containing `standard` or `recommended` map to a built-in preset, anything else is ignored |
| `defaultSeverity` | read |
| `ignoreFiles` | read, becomes a gitignore-style matcher rooted at that config's directory |
| `overrides` | read; `files` globs matched against the linted path, rules merged on top |
| `plugins`, `processors`, `customSyntax`, `languageOptions`, `cache`, `reportNeedlessDisables`, `reportInvalidScopeDisables`, `reportDescriptionlessDisables` | not read, ignored silently |

```json
{
  "extends": ["stylelint-config-standard"],
  "defaultSeverity": "warning",
  "rules": {
    "block-no-empty": true,
    "color-hex-length": "short",
    "declaration-block-no-duplicate-properties": [true, { "ignore": ["consecutive-duplicates"] }]
  },
  "ignoreFiles": ["**/*.min.css"],
  "overrides": [
    {
      "files": ["**/*.scss"],
      "rules": { "at-rule-no-unknown": null }
    }
  ]
}
```

`extends` takes a string or an array of strings. Each entry is matched by substring
against the two built-in presets, so `stylelint-config-recommended` and
`stylelint-config-standard` both resolve, and an unknown package name is ignored without
a warning.

## Which files get linted

| Rule | Behaviour |
| --- | --- |
| Extensions | `.css`, `.pcss`, `.postcss`, `.scss`, `.sass`, `.sss`, `.sugarss`, `.less`; anything else is skipped, even when an explicit glob matches it |
| `node_modules` | skipped, hard-coded; `--disable-default-ignores` turns this off |
| Hidden files and directories | skipped by the directory walker |
| `.gitignore` | not read |
| `.stylelintignore` | read from the working directory when `--ignore-path` is absent |
| `--ignore-path`, `--ignore-pattern` | replace and extend the ignore file, gitignore syntax, repeatable |
| `ignoreFiles` from config | applied per config directory |
| `!glob` on the command line | treated as an exclude |
| Nothing matched | exit 1, unless `--allow-empty-input` |

## Output

| Surface | Behaviour |
| --- | --- |
| Report stream | stderr, always |
| stdout | only the fixed source, and only for `--stdin --fix` |
| `--output-file` | writes the same report to a file as well |
| `--formatter` | `string`, `json`, `verbose`, alias `text` |
| Warning order | sorted for regular CLI output |

The report goes to stderr because stdout belongs to the fixed source in the stdin case.
`stx src/ > report.txt` therefore writes an empty file, not a report.

```sh
stx --formatter json src/ 2> report.json
```

## Fixing

| Item | Behaviour |
| --- | --- |
| Passes | up to 10, stopping when the source stops changing |
| Overlapping edits | an edit that starts before the previous edit ended is dropped |
| Fix metadata | on `LintPass.fixes` in the library, absent from JSON output |
| `computeEditInfo` equivalent | none in the public API |

## Disable directives

| Item | Behaviour |
| --- | --- |
| Granularity | line ranges, not source ranges |
| Same-line `disable` and `enable` pair | does not cancel the rule, unlike Stylelint |
| Invalid sequences: already disabled, not disabled | ignored silently, Stylelint reports them |
| `// stylelint-disable` in SCSS and Less | supported, including multi-line `// --` descriptions |
| `stylelint-enable` for one rule after a disable-all | supported |

## Parse failures

When the parser rejects a stylesheet, the run emits a `CssSyntaxError` diagnostic and
then runs the rules on an empty tree. Two consequences: rules with a source-text fallback
can still report, and comments are lost in that path, so disable directives stop working
for that file.

## JavaScript API

```js
const { lint } = require('@outslept/stx')

const result = lint('a { color: #FFFFFF; }', {
  config: { rules: { 'color-hex-length': 'short' } },
  fix: true,
  syntax: 'css',        // 'css' | 'scss' | 'sass' | 'less'
  filename: 'a.css',
})

// { code: 'a { color: #FFF; }', errored: true, warnings: [...] }
```

Without `config`, the `standard` preset is used. Warnings carry `line`, `column`,
`endLine`, `endColumn`, `rule`, `severity`, and `text`.

## Syntaxes

| `.css` `.pcss` `.postcss` | `.scss` | `.sass` `.sss` `.sugarss` | `.less` |
| --- | --- | --- | --- |
| CSS | SCSS | Sass (indented) | Less |

Selected by file extension, or by `--stdin-filename` / the `syntax` option.

## Current state

This section describes `dev` at commit `36e2386` (2026-11-25). Numbers carry their date
because they move.

### What works

- Linting in four syntaxes, chosen by file extension or by `--stdin-filename`.
- Autofix for 40 rules, applied in up to ten passes until the source stops changing.
- Configuration with `rules`, `extends`, `overrides`, `defaultSeverity`, `ignoreFiles`.
- Disable directives, including multi-line `// --` descriptions in SCSS and Less.
- Two entry points on one engine: the `stx` binary and `lint()` from the Node addon.

### Not done yet

Real gaps, with their honest size:

- **Complete option validation.** Unknown rule names are rejected. Per-rule schemas are
  not: most secondary options are read but never validated, so a typo in an option name
  passes silently.
- **Complete CSS value grammar.** `declaration-property-value-no-unknown` matches 151 of
  176 fixtures. Unmodelled properties and functions are skipped rather than guessed.
- **Parser-rejected input.** Where the parser refuses a stylesheet, rules run on an empty
  tree, so some malformed selectors never reach the rules that would report them.
- **`extends` beyond two presets.** Any other value is ignored, where Stylelint resolves
  the package or fails loudly.
- **Editors.** No IDE integration, no `--stdin` source map handling for embedded styles.

### Numbers

| Metric | Value |
| --- | --- |
| Rules | 154 (153 mirror Stylelint 18.1.0, 1 is STX-only) |
| Replayable upstream fixtures | 8,565 of 8,790 |
| Matching fixtures | 8,495 (99.18%) |
| Rules matching every replayable fixture | 132 of 153 |
| Rules below 90% | 21 |
| `#[test]` functions in `src/` | 607 |
| Rules with autofix declared | 40 (30 unconditional, 10 conditional) |
| Rules with no local test | 5 |
| Accepted replay commit | `6bca026`, 2026-11-25 |

The replay was not re-run after the refactors that produced `36e2386`. Those commits
moved files and changed preset membership; they did not change rule behaviour, and
replay fixtures carry their own explicit configs.

Three rules account for most of the remaining differences:

| Rule | Match | What remains |
| --- | --- | --- |
| `declaration-property-value-no-unknown` | 151/176 | custom property substitution, `@property`, `text-box-edge`, `color-mix()` |
| `selector-no-invalid` | 45/54 | selectors rejected before rules run, `::slotted()` arguments |
| `selector-disallowed-list` | 37/42 | regex and selector-list cases |

Per-rule status for all 154 rules lives in [RULES.md](RULES.md), which is generated from
the rule metadata and the replay baseline rather than written by hand.

## Compatibility with Stylelint

Where behaviour differs, it is listed here rather than left to be discovered.

| Area | Stylelint 18.1.0 | STX |
| --- | --- | --- |
| Exit codes | 1 or 2 | 0, 1, 2, 64, 78 (see [exit codes](#exit-codes)) |
| Report stream | stdout | stderr |
| Formatters | 7 | 3 (`string`, `json`, `verbose`) |
| `extends` | any package or path, recursive | two built-in presets, matched by substring |
| Config formats | JSON, YAML, JS, TS via cosmiconfig | JSON only, other formats fail with exit 78 |
| `.gitignore` | not consulted | not consulted |
| Hidden paths | linted when matched | skipped |
| Secondary options | validated, all implemented | implemented per rule, not validated |
| Unknown rule names | warning, exit code per severity | stderr message, exit 78 |
| `maxWarnings` | config key | CLI flag only |
| Disable granularity | source ranges | line ranges |
| Invalid disable sequences | reported | ignored |
| Autofix engine | single pass plus re-lint, conflict aware | up to 10 passes, conflict aware |
| `computeEditInfo` | public API | absent |
| Embedded styles | via `customSyntax` | not supported |
| Parser | PostCSS | `oxc-css-parser` 0.0.22 |

## Development

```sh
just check      # fmt, clippy, tests: the same commands CI runs
just napi       # build and test the Node-API addon
just example    # lint the example project
just deny       # advisories, licenses, banned crates
```

## License

MIT.
