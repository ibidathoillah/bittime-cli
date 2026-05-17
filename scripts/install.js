const fs = require('fs');
const https = require('https');
const os = require('os');
const path = require('path');

const pkg = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'package.json'), 'utf8'));
const repo = 'ibidathoillah/bittime-cli';

function targetName() {
  const platform = os.platform();
  const arch = os.arch();

  const platforms = {
    linux: 'linux',
    darwin: 'macos',
    win32: 'windows',
  };

  const arches = {
    x64: 'x86_64',
    arm64: 'aarch64',
  };

  if (!platforms[platform] || !arches[arch]) {
    return null;
  }

  const ext = platform === 'win32' ? '.exe' : '';
  return `bittime-${platforms[platform]}-${arches[arch]}${ext}`;
}

const artifact = targetName();
const binDir = path.join(__dirname, '..', 'bin');
const binName = os.platform() === 'win32' ? 'bittime.exe' : 'bittime';
const binPath = path.join(binDir, binName);

if (!artifact) {
  console.warn(`Unsupported platform for prebuilt bittime binary: ${os.platform()} ${os.arch()}`);
  process.exit(0);
}

fs.mkdirSync(binDir, { recursive: true });

const url = `https://github.com/${repo}/releases/download/v${pkg.version}/${artifact}`;

function download(source, destination, redirects = 0) {
  console.log(`Downloading bittime-cli binary from ${source}...`);

  const file = fs.createWriteStream(destination);
  https
    .get(source, (response) => {
      if ([301, 302, 303, 307, 308].includes(response.statusCode)) {
        file.close();
        fs.unlink(destination, () => {});
        if (redirects > 5) {
          console.warn('Too many redirects while downloading bittime-cli binary.');
          process.exit(0);
        }
        download(response.headers.location, destination, redirects + 1);
        return;
      }

      if (response.statusCode !== 200) {
        file.close();
        fs.unlink(destination, () => {});
        console.warn(`Binary download returned HTTP ${response.statusCode}.`);
        console.warn('npm install will complete, but install Cargo manually if needed: cargo install bittime-cli');
        process.exit(0);
      }

      response.pipe(file);
      file.on('finish', () => {
        file.close();
        fs.chmodSync(destination, 0o755);
        console.log('\x1b[32mbittime-cli binary installed successfully.\x1b[0m');
      });
    })
    .on('error', (err) => {
      file.close();
      fs.unlink(destination, () => {});
      console.warn(`Failed to download bittime-cli binary: ${err.message}`);
      console.warn('npm install will complete, but install Cargo manually if needed: cargo install bittime-cli');
      process.exit(0);
    });
}

download(url, binPath);
