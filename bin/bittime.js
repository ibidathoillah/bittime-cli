#!/usr/bin/env node

const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');

const binName = process.platform === 'win32' ? 'bittime.exe' : 'bittime';
const binPath = path.join(__dirname, binName);

if (!fs.existsSync(binPath)) {
  console.error('\x1b[31mError: bittime native binary not found.\x1b[0m');
  console.error(`Expected at: ${binPath}`);
  console.error('\nTry reinstalling with: npm install -g bittime-cli');
  console.error('Or install from Cargo: cargo install bittime-cli');
  process.exit(1);
}

const child = spawn(binPath, process.argv.slice(2), {
  stdio: 'inherit',
});

child.on('exit', (code) => {
  process.exit(code || 0);
});

child.on('error', (err) => {
  console.error(`\x1b[31mError spawning bittime binary:\x1b[0m ${err.message}`);
  process.exit(1);
});
