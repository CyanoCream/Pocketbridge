// Usage: node shot.mjs <url> <out.png> <width> <height> [dsf=1] [mobile=0] [scheme=light|dark] [js]
// Renders a page with exact viewport emulation through the Chrome DevTools Protocol.
import { spawn } from 'node:child_process';
import { writeFileSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const [url, out, w, h, dsf = '1', mobile = '0', scheme = 'light', js = ''] = process.argv.slice(2);
const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const port = 9300 + Math.floor(Math.random() * 500);
const profile = mkdtempSync(join(tmpdir(), 'pbcdp-'));
const chrome = spawn(CHROME, ['--headless=new', '--disable-gpu', '--hide-scrollbars', `--remote-debugging-port=${port}`,
  `--user-data-dir=${profile}`, 'about:blank'], { stdio: 'ignore' });
const sleep = ms => new Promise(r => setTimeout(r, ms));

try {
  let target;
  for (let i = 0; i < 50 && !target; i++) {
    await sleep(200);
    try { target = (await (await fetch(`http://127.0.0.1:${port}/json`)).json()).find(t => t.type === 'page'); } catch {}
  }
  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise(r => ws.onopen = r);
  let id = 0; const pending = new Map();
  ws.onmessage = e => { const m = JSON.parse(e.data); if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); } };
  const send = (method, params = {}) => new Promise(r => { const i = ++id; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });

  await send('Emulation.setDeviceMetricsOverride', { width: +w, height: +h, deviceScaleFactor: +dsf, mobile: mobile === '1' });
  await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: scheme }] });
  await send('Page.enable');
  await send('Page.navigate', { url });
  await sleep(3500);
  if (js) { await send('Runtime.evaluate', { expression: js, awaitPromise: true }); await sleep(1200); }
  const shot = await send('Page.captureScreenshot', { format: 'png', clip: { x: 0, y: 0, width: +w, height: +h, scale: 1 } });
  writeFileSync(out, Buffer.from(shot.result.data, 'base64'));
  ws.close();
} finally {
  chrome.kill();
}
