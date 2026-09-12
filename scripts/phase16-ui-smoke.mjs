// Local browser regression. Does not mock a native backend or claim native IPC coverage.
import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
const root = process.cwd();
const browser = [process.env.PROGRAMFILES, process.env["PROGRAMFILES(X86)"]]
  .filter(Boolean)
  .map((p) => path.join(p, "Microsoft/Edge/Application/msedge.exe"))
  .find((p) => fs.existsSync(p));
if (!browser)
  throw new Error("Microsoft Edge is required for this local UI smoke test.");
fs.mkdirSync(".tmp", { recursive: true });
const preview = spawn(
  process.execPath,
  [
    "node_modules/vite/bin/vite.js",
    "preview",
    "--host",
    "127.0.0.1",
    "--port",
    "1423",
    "--strictPort",
  ],
  { cwd: root, windowsHide: true, stdio: "ignore" },
);
const edge = spawn(
  browser,
  [
    "--headless=new",
    "--disable-gpu",
    "--no-first-run",
    "--no-default-browser-check",
    "--remote-debugging-port=9333",
    "--remote-debugging-address=127.0.0.1",
    `--user-data-dir=${path.join(root, ".tmp/phase16-edge")}`,
    "about:blank",
  ],
  { windowsHide: true, stdio: "ignore" },
);
const pause = (ms) => new Promise((r) => setTimeout(r, ms));
async function retry(url) {
  for (let i = 0; i < 60; i++) {
    try {
      const r = await fetch(url);
      if (r.ok) return r;
    } catch {}
    await pause(250);
  }
  throw new Error(`Not ready: ${url}`);
}
let ws;
try {
  await retry("http://127.0.0.1:1423");
  await retry("http://127.0.0.1:9333/json/version");
  const targets = await (await fetch("http://127.0.0.1:9333/json")).json();
  ws = new WebSocket(
    targets.find((t) => t.type === "page").webSocketDebuggerUrl,
  );
  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = reject;
  });
  const pending = new Map();
  let id = 0;
  const exceptions = [];
  ws.onmessage = (e) => {
    const v = JSON.parse(e.data);
    if (v.id && pending.has(v.id)) {
      const p = pending.get(v.id);
      pending.delete(v.id);
      v.error
        ? p.reject(new Error(JSON.stringify(v.error)))
        : p.resolve(v.result);
    }
    if (v.method === "Runtime.exceptionThrown")
      exceptions.push(v.params.exceptionDetails.text);
  };
  function cdp(method, params = {}) {
    return new Promise((resolve, reject) => {
      const key = ++id;
      pending.set(key, { resolve, reject });
      ws.send(JSON.stringify({ id: key, method, params }));
      setTimeout(() => {
        if (pending.delete(key)) reject(new Error(`${method} timed out`));
      }, 10000).unref();
    });
  }
  async function evaluate(expression) {
    const r = await cdp("Runtime.evaluate", {
      expression,
      returnByValue: true,
      awaitPromise: true,
    });
    if (r.exceptionDetails) throw new Error(r.exceptionDetails.text);
    return r.result.value;
  }
  await cdp("Runtime.enable");
  await cdp("Page.enable");
  await cdp("Emulation.setDeviceMetricsOverride", {
    width: 1000,
    height: 760,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await cdp("Page.navigate", { url: "http://127.0.0.1:1423" });
  await pause(1200);
  const results = [];
  for (const name of [
    "Overview",
    "Command history",
    "Skills",
    "Voice & wake word",
    "Privacy & data",
    "Projects",
    "Workflows",
    "Application tools",
    "File & folder tools",
    "System tools",
    "About",
  ]) {
    const found = await evaluate(
      `(()=>{const b=[...document.querySelectorAll('nav button')].find(b=>b.textContent.trim()===${JSON.stringify(name)});if(!b)return false;b.click();return true;})()`,
    );
    if (!found) throw new Error(`Missing navigation: ${name}`);
    await pause(250);
    const heading = await evaluate(`document.querySelector('h1')?.textContent`);
    if (!heading) throw new Error(`Page lacks heading: ${name}`);
    results.push({ page: name, heading, status: "passed" });
  }
  await evaluate(
    `([...document.querySelectorAll('nav button')].find(b=>b.textContent.trim()==='Projects')).click()`,
  );
  await pause(200);
  await evaluate(
    `([...document.querySelectorAll('button')].find(b=>b.textContent==='New project')).click()`,
  );
  const project = await evaluate(
    `({fields:[...document.querySelectorAll('form input,form textarea,form select')].length,saveDisabled:[...document.querySelectorAll('form button')].find(b=>b.textContent==='Save approved profile')?.disabled})`,
  );
  if (project.fields < 9 || !project.saveDisabled)
    throw new Error("Project approval form failed");
  const shot = await cdp("Page.captureScreenshot", { format: "png" });
  fs.writeFileSync(
    ".tmp/phase16-projects.png",
    Buffer.from(shot.data, "base64"),
  );
  await evaluate(
    `([...document.querySelectorAll('nav button')].find(b=>b.textContent.trim()==='Workflows')).click()`,
  );
  await pause(200);
  await evaluate(
    `([...document.querySelectorAll('button')].find(b=>b.textContent==='New workflow')).click()`,
  );
  await evaluate(
    `([...document.querySelectorAll('button')].find(b=>b.textContent==='Add step')).click()`,
  );
  await evaluate(
    `([...document.querySelectorAll('button')].find(b=>b.textContent==='Add step')).click()`,
  );
  const steps = await evaluate(
    `document.querySelectorAll('.workflow-steps li').length`,
  );
  if (steps !== 2) throw new Error("Workflow step editor failed");
  await evaluate(
    `document.querySelector('button[aria-label="Move step 2 up"]').click()`,
  );
  await evaluate(
    `([...document.querySelectorAll('.workflow-steps button')].find(b=>b.textContent==='Remove')).click()`,
  );
  if (
    (await evaluate(
      `document.querySelectorAll('.workflow-steps li').length`,
    )) !== 1
  )
    throw new Error("Workflow step removal failed");
  if (exceptions.length)
    throw new Error(`Unhandled UI exceptions: ${exceptions.join(", ")}`);
  fs.writeFileSync(
    ".tmp/phase16-ui-results.json",
    JSON.stringify(
      {
        pages: results,
        projectApproval: "passed",
        workflowEditing: "passed",
        uncaughtExceptions: exceptions,
        nativeIPC: "not tested in browser preview",
      },
      null,
      2,
    ),
  );
  console.log(
    JSON.stringify(
      {
        pages: results.length,
        projectApproval: "passed",
        workflowEditing: "passed",
        nativeIPC: "not tested",
      },
      null,
      2,
    ),
  );
  await cdp("Browser.close").catch(() => {});
} finally {
  if (ws) ws.close();
  edge.kill();
  preview.kill();
}
