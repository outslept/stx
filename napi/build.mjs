import { createBuildCommand } from '@napi-rs/cli'

const target = process.platform === 'win32' && process.arch === 'x64' && !process.env.CARGO_BUILD_TARGET
  ? ['--target', 'x86_64-pc-windows-msvc']
  : []

await createBuildCommand([
  '--platform', '--release', '--no-const-enum', ...target, '--', '--locked', ...process.argv.slice(2),
]).execute()
