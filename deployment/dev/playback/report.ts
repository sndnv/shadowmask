import { loadSubjects, OUT, readResults } from "./results.ts";
import type { Result } from "./runner.ts";

const BASELINE = new URL("./modes.json", import.meta.url).pathname;
const ACCEPT = Deno.env.get("PLAYBACK_ACCEPT") === "1";
const NEGOTIATED =
  /negotiated session \[([^\]]+)\] generation \[\d+\]: .*? report \[([^\]]*)\] .*? mode \[(\w+)\] container \[([^\]]*)\] reasons \[([^\]]*)\]/;

type Modes = Record<string, string[]>;

async function negotiations(): Promise<Map<string, string[]>> {
  const found = new Map<string, string[]>();
  const text = await Deno.readTextFile(`${OUT}/server.log`).catch(() => "");
  for (const line of text.split("\n")) {
    const match = NEGOTIATED.exec(line);
    if (!match) {
      continue;
    }
    const [, session, report, mode, container, reasons] = match;
    const shape = container === "none" ? mode.toLowerCase() : `${mode.toLowerCase()}/${container}`;
    found.set(session, [...(found.get(session) ?? []), `${shape} [${reasons}] report ${report}`]);
  }
  return found;
}

function keyOf(r: Result): string {
  return `${r.engine} | ${r.subject} | ${r.action}`;
}

function show(modes: string[]): string {
  return modes.length > 0 ? modes.join(" then ") : "none";
}

function fail(r: Result, why: string): void {
  r.status = "fail";
  r.detail = `${r.detail}; ${why}`;
}

function sorted(modes: Modes): Modes {
  return Object.fromEntries(
    Object.entries(modes).sort(([a], [b]) => a.localeCompare(b, "en", { numeric: true })),
  );
}

const results = await readResults();
const { library } = await loadSubjects();
const negotiated = await negotiations();
const expected: Modes = library ? {} : JSON.parse(await Deno.readTextFile(BASELINE).catch(() => "{}"));
const observed: Modes = { ...expected };
const shown = new Map<Result, string>();
for (const r of results) {
  const got = r.sessions.flatMap((session) => negotiated.get(session) ?? []);
  observed[keyOf(r)] = got;
  shown.set(r, show(got));
  if (library || ACCEPT) {
    continue;
  }
  const want = expected[keyOf(r)];
  if (want === undefined) {
    fail(r, `no expected modes recorded, negotiated ${show(got)}`);
  } else if (show(want) !== show(got)) {
    fail(r, `modes expected ${show(want)}, negotiated ${show(got)}`);
  }
}
if (!library) {
  await Deno.writeTextFile(`${OUT}/modes.json`, `${JSON.stringify(sorted(observed), null, 2)}\n`);
}

const counts = { pass: 0, fail: 0, skip: 0 };
for (const r of results) {
  counts[r.status]++;
}

const header =
  "| Status | Player | Subject | Action | Mode | Negotiated | Detail | Log |\n|---|---|---|---|---|---|---|---|";
const rows = results.map((r) => {
  const log = r.status === "fail" && r.log ? r.log : "";
  return `| ${r.status} | ${r.engine} | ${r.subject} | ${r.action} | ${r.mode} | ${shown.get(r)} | ${
    r.detail.replaceAll("|", "/")
  } | ${log} |`;
});
const summary = `${counts.pass} passed, ${counts.fail} failed, ${counts.skip} skipped`;
await Deno.writeTextFile(`${OUT}/report.md`, `# Playback check\n\n${summary}\n\n${header}\n${rows.join("\n")}\n`);

for (const r of results.filter((r) => r.status === "fail")) {
  console.log(
    `FAIL ${r.engine} | ${r.subject} | ${r.action} | ${r.mode} | ${r.detail}${r.log ? ` (log ${r.log})` : ""}`,
  );
}
console.log(summary);
Deno.exit(counts.fail > 0 || results.length === 0 ? 1 : 0);
