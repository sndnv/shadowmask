import { Api, type Negotiation, sleep } from "./api.ts";
import { fixtureSubjects, librarySubjects } from "./matrix.ts";
import {
  type Controls,
  type Engine,
  type Player,
  type Reading,
  type Result,
  runSubject,
  type Tracks,
} from "./runner.ts";
import { loadSubjects, writeResults } from "./results.ts";

const PAGE_PORT = 8096;
const DEBUG_PORT = 9222;
const VENDOR = Deno.env.get("PLAYBACK_VENDOR") ?? "/work/vendor";
const BASIC = Deno.env.get("PLAYBACK_BASIC") ?? "/work/basic";
const PAGE = new URL("./page.html", import.meta.url).pathname;
const CALL_TIMEOUT_MS = 90_000;
const SUBTITLE_DELIVERIES = ["external", "hls_vtt"];

const TYPES: Record<string, string> = { html: "text/html", js: "text/javascript" };

function servePage(): Deno.HttpServer {
  return Deno.serve({ port: PAGE_PORT, hostname: "0.0.0.0", onListen() {} }, async (req) => {
    const path = new URL(req.url).pathname;
    const file = path === "/page.html"
      ? PAGE
      : path.startsWith("/vendor/")
      ? `${VENDOR}/${path.slice(8)}`
      : path.startsWith("/basic/")
      ? `${BASIC}/${path.slice(7)}`
      : null;
    if (!file || file.includes("..")) {
      return new Response("not found", { status: 404 });
    }
    const body = await Deno.readFile(file).catch(() => null);
    if (!body) {
      return new Response("not found", { status: 404 });
    }
    return new Response(body, {
      headers: { "content-type": TYPES[file.split(".").pop() ?? ""] ?? "application/octet-stream" },
    });
  });
}

function launchChromium(): Deno.ChildProcess {
  return new Deno.Command("chromium", {
    args: [
      "--headless=new",
      "--no-sandbox",
      "--disable-dev-shm-usage",
      "--disable-gpu",
      "--no-first-run",
      "--autoplay-policy=no-user-gesture-required",
      `--remote-debugging-port=${DEBUG_PORT}`,
      "--user-data-dir=/tmp/chromium",
      "about:blank",
    ],
    stdin: "null",
    stdout: "null",
    stderr: "null",
  }).spawn();
}

async function devtoolsReady(): Promise<void> {
  for (let i = 0; i < 100; i++) {
    const response = await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/version`).catch(() => null);
    if (response?.ok) {
      await response.body?.cancel();
      return;
    }
    await sleep(200);
  }
  throw new Error("chromium did not open its debugging port");
}

function withTimeout<T>(work: Promise<T>, what: string): Promise<T> {
  return Promise.race([
    work,
    sleep(CALL_TIMEOUT_MS).then(() => {
      throw new Error(`${what} did not answer within ${CALL_TIMEOUT_MS / 1000} s`);
    }),
  ]);
}

class Tab {
  private next = 1;
  private readonly pending = new Map<number, (message: Record<string, unknown>) => void>();

  private constructor(private readonly ws: WebSocket, readonly id: string) {
    ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      const resolve = this.pending.get(message.id);
      if (resolve) {
        this.pending.delete(message.id);
        resolve(message);
      }
    };
  }

  static async open(url: string): Promise<Tab> {
    const response = await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/new?${url}`, { method: "PUT" });
    const target = await response.json();
    const ws = new WebSocket(target.webSocketDebuggerUrl);
    await new Promise((resolve, reject) => {
      ws.onopen = resolve;
      ws.onerror = reject;
    });
    const tab = new Tab(ws, target.id);
    for (let i = 0; i < 100; i++) {
      if (await tab.evaluate("typeof window.player === 'object'")) {
        return tab;
      }
      await sleep(100);
    }
    throw new Error("the check page did not load");
  }

  private send(method: string, params: Record<string, unknown>): Promise<Record<string, unknown>> {
    const id = this.next++;
    const reply = new Promise<Record<string, unknown>>((resolve) => this.pending.set(id, resolve));
    this.ws.send(JSON.stringify({ id, method, params }));
    return reply;
  }

  async evaluate(expression: string): Promise<unknown> {
    const reply = await withTimeout(
      this.send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true }),
      expression.split("(")[0],
    );
    const result = reply.result as {
      result?: { value?: unknown };
      exceptionDetails?: { exception?: { description?: string } };
    };
    if (result.exceptionDetails) {
      throw new Error(result.exceptionDetails.exception?.description ?? "page exception");
    }
    return result.result?.value;
  }

  async close(): Promise<void> {
    this.ws.close();
    await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/close/${this.id}`).then((r) => r.body?.cancel()).catch(() =>
      undefined
    );
  }
}

class BrowserPlayer implements Player {
  readonly log = null;

  constructor(private readonly tab: Tab) {}

  async attach(url: string, mode: string, offsetMs: number, tracks: Tracks): Promise<void> {
    const subtitleUrl = tracks.subtitle?.delivery === "external" ? tracks.subtitle.url : null;
    const args = [JSON.stringify(url), JSON.stringify(mode), offsetMs, JSON.stringify(subtitleUrl)].join(", ");
    await this.tab.evaluate(`player.attach(${args})`);
  }

  async play(): Promise<void> {
    await this.tab.evaluate("player.play()");
  }

  async seekTo(offsetMs: number): Promise<void> {
    await this.tab.evaluate(`player.seekTo(${offsetMs})`);
  }

  async reading(): Promise<Reading> {
    return await this.tab.evaluate("player.reading()") as Reading;
  }

  async close(): Promise<string[]> {
    const problems = await this.tab.evaluate("player.problems()").catch(() => []) as string[];
    await this.tab.close();
    return problems;
  }
}

async function measureInPage(): Promise<Record<string, unknown> | null> {
  const tab = await Tab.open(`http://localhost:${PAGE_PORT}/page.html`);
  try {
    for (let i = 0; i < 100; i++) {
      if (await tab.evaluate("typeof window.measureDecoding === 'function'")) {
        return await tab.evaluate("measureDecoding()") as Record<string, unknown> | null;
      }
      await sleep(100);
    }
    throw new Error("basic's capabilities.js did not load in the check page");
  } finally {
    await tab.close();
  }
}

class ChromiumEngine implements Engine {
  readonly name = "chromium";
  readonly forcedCases = false;

  constructor(private readonly profileVersion: number, private readonly decoding: Record<string, unknown> | null) {}

  startBody(versionId: string, startMs: number, controls: Controls): Promise<Record<string, unknown>> {
    return Promise.resolve({
      version_id: versionId,
      start_position_ms: startMs,
      capabilities: {
        platform: "chrome",
        profile_version: this.profileVersion,
        subtitle_deliveries: SUBTITLE_DELIVERIES,
        audio_tracks: false,
        ...(this.decoding ? { decoding: this.decoding } : {}),
      },
      ...(controls.audioTrack !== null ? { audio_track: controls.audioTrack } : {}),
      ...(controls.subtitle ? { subtitle: { track: controls.subtitle } } : { subtitle_off: true }),
      ...(controls.delivery ? { delivery: controls.delivery } : {}),
    });
  }

  loadOffset(stream: Negotiation, atMs: number): number {
    return atMs - stream.origin_ms;
  }

  drawsSubtitles(): boolean {
    return true;
  }

  async open(_name: string): Promise<Player> {
    return new BrowserPlayer(await Tab.open(`http://localhost:${PAGE_PORT}/page.html`));
  }
}

if (import.meta.main) {
  const server = servePage();
  const chromium = launchChromium();
  try {
    await devtoolsReady();
    const api = await Api.login();
    const decoding = await measureInPage();
    console.log(`basic's capability report: ${JSON.stringify(decoding)}`);
    const engine = new ChromiumEngine(await api.profileVersion(), decoding);
    const { versions, labels, library } = await loadSubjects();
    const subjects = library ? librarySubjects(versions, labels) : fixtureSubjects(versions);
    const results: Result[] = [];
    for (const [i, subject] of subjects.entries()) {
      results.push(...await runSubject(api, engine, subject, i + 1));
    }
    await writeResults("chromium", results);
  } finally {
    chromium.kill();
    await server.shutdown();
  }
}
