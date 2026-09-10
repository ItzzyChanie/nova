import fs from "node:fs";
import ts from "typescript";
import assert from "node:assert/strict";
const output = ts.transpileModule(
  fs.readFileSync("src/services/commandHistory.ts", "utf8"),
  {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
    },
  },
).outputText;

const moduleCode = output.replace(
  '"@tauri-apps/api/core"',
  JSON.stringify(import.meta.resolve("@tauri-apps/api/core")),
);

const { localHistoryDay, historyForDay } = await import(
  "data:text/javascript;base64," + Buffer.from(moduleCode).toString("base64")
);

process.env.TZ = "Asia/Manila";
const day = localHistoryDay(new Date("2026-09-10T23:59:59+08:00"));
const history = [day.start - 1, day.start, day.end - 1, day.end].map(
  (timestamp) => ({ timestamp }),
);

assert.deepEqual(
  historyForDay(history, day).map((e) => e.timestamp),
  [day.start, day.end - 1],
);

assert.equal(
  historyForDay(history.slice(0, 3), localHistoryDay(new Date(day.end))).length,
  0,
);

assert.equal(
  localHistoryDay(new Date("2026-12-31T23:59:59+08:00")).end,
  new Date("2027-01-01T00:00:00+08:00").getTime(),
);

process.env.TZ = "America/New_York";
const spring = localHistoryDay(new Date("2026-03-08T12:00:00-04:00"));
const fall = localHistoryDay(new Date("2026-11-01T12:00:00-05:00"));
assert.equal(spring.end - spring.start, 23 * 3600000);
assert.equal(fall.end - fall.start, 25 * 3600000);
console.log(
  "History checks passed: milliseconds, midnight reset, year rollover, and DST.",
);
