#!/usr/bin/env node

try {
  process.exitCode = require('./index.js').__runCli(process.argv.slice(1))
} catch (error) {
  console.error(error.message)
  process.exitCode = 1
}
