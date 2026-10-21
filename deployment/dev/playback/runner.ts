import { Api, type Negotiation, SERVER, sleep, type Version } from "./api.ts";
import type { ImageKind, ImageTrack, Subject, SubtitleTrack } from "./matrix.ts";

export type ImageReading = { start: number | null; codec: string | null; visible: boolean };

export type Reading = {
  positionMs: number;
  bufferedAheadMs: number;
  subtitle: string;
  audio: boolean;
  audioIndex: number | null;
  toneHz: number | null;
  image: ImageReading | null;
};

export type SubtitleLoad = { delivery: string; url: string | null; index: number | null; image: boolean };

export type Tracks = { drawn: boolean; subtitle: SubtitleLoad | null; audioIndex: number | null };

export interface Player {
  readonly log: string | null;
  attach(url: string, mode: string, offsetMs: number, tracks: Tracks): Promise<void>;
  play(): Promise<void>;
  seekTo(offsetMs: number): Promise<void>;
  reading(): Promise<Reading>;
  close(): Promise<string[]>;
}

export type Controls = { subtitle: SubtitleTrack | null; audioTrack: number | null; delivery: string | null };

export interface Engine {
  name: string;
  forcedCases: boolean;
  startBody(versionId: string, startMs: number, controls: Controls): Promise<Record<string, unknown>>;
  loadOffset(stream: Negotiation, atMs: number): number;
  drawsSubtitles(stream: Negotiation): boolean;
  open(name: string): Promise<Player>;
}

export type Status = "pass" | "fail" | "skip";

export type Result = {
  engine: string;
  subject: string;
  action: string;
  status: Status;
  detail: string;
  mode: string;
  log: string | null;
  sessions: string[];
};

const TOLERANCE_MS = 1500;
const TONE_TOLERANCE_HZ = 15;
const IMAGE_LINE_MS = 60_000;
const IMAGE_FORMATS = new Set(["pgs", "vob_sub", "dvb"]);
const IMAGE_CODECS: Record<ImageKind, string> = { pgs: "hdmv_pgs_subtitle", dvb: "dvb_subtitle" };
const OFF: Controls = { subtitle: null, audioTrack: null, delivery: null };
const FORCED: Controls = { ...OFF, delivery: "always" };
const ONLY = Deno.env.get("PLAYBACK_ONLY") ? new RegExp(Deno.env.get("PLAYBACK_ONLY")!) : null;

export function clock(ms: number): string {
  const total = Math.round(ms / 1000);
  const sign = total < 0 ? "-" : "";
  const s = Math.abs(total);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  return `${sign}${h}:${String(m).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
}

class Watch {
  stream!: Negotiation;
  private beat?: ReturnType<typeof setInterval>;
  private controls: Controls = OFF;

  constructor(
    private readonly api: Api,
    private readonly engine: Engine,
    readonly player: Player,
    private readonly version: Version,
  ) {}

  async begin(versionId: string, startMs: number, controls: Controls): Promise<void> {
    this.controls = controls;
    this.stream = await this.api.start(await this.engine.startBody(versionId, startMs, controls));
    await this.load(startMs);
    this.beat = setInterval(() => {
      this.player.reading()
        .then((r) => this.api.progress(this.stream.session_id, this.timeline(r)))
        .catch(() => undefined);
    }, 10_000);
  }

  private async load(atMs: number): Promise<void> {
    const offset = this.engine.loadOffset(this.stream, atMs);
    await this.player.attach(`${SERVER}${this.stream.manifest_url}`, this.stream.mode, offset, this.tracks());
    await this.player.play();
  }

  private tracks(): Tracks {
    const selected = this.stream.selected ?? {};
    const delivery = selected.subtitle_delivery ?? null;
    const wanted = this.controls.subtitle;
    const index = wanted?.type === "embedded" ? wanted.index : null;
    const format = this.version.subtitles.find((s) => s.index === index)?.format.toLowerCase() ?? "";
    const loads = delivery === "embedded" || delivery === "external";
    return {
      drawn: this.engine.drawsSubtitles(this.stream),
      subtitle: loads
        ? {
          delivery,
          url: selected.subtitle_url ? `${SERVER}${selected.subtitle_url}` : null,
          index,
          image: IMAGE_FORMATS.has(format),
        }
        : null,
      audioIndex: selected.audio_delivery === "embedded" ? selected.audio_track ?? null : null,
    };
  }

  timeline(reading: Reading): number {
    return this.stream.origin_ms + reading.positionMs;
  }

  async farSeek(targetMs: number): Promise<void> {
    if (!this.stream.sequential) {
      return await this.player.seekTo(targetMs - this.stream.origin_ms);
    }
    this.stream = await this.api.seek(this.stream.session_id, targetMs);
    await this.load(targetMs);
  }

  nearSeek(targetMs: number): Promise<void> {
    return this.player.seekTo(targetMs - this.stream.origin_ms);
  }

  mode(): string {
    if (!this.stream) {
      return "none";
    }
    return this.stream.container ? `${this.stream.mode}/${this.stream.container}` : this.stream.mode;
  }

  async end(): Promise<string[]> {
    clearInterval(this.beat);
    if (this.stream) {
      await this.api.end(this.stream.session_id);
    }
    return await this.player.close();
  }
}

async function playing(watch: Watch, timeoutMs = 60_000): Promise<Reading> {
  const until = Date.now() + timeoutMs;
  let last = await watch.player.reading();
  while (Date.now() < until) {
    await sleep(500);
    const now = await watch.player.reading();
    if (now.positionMs > last.positionMs + 200) {
      return now;
    }
    last = now;
  }
  throw new Error(`not playing within ${timeoutMs / 1000} s, player at ${clock(last.positionMs)}`);
}

async function landed(watch: Watch, targetMs: number, since: number): Promise<[boolean, string]> {
  await playing(watch);
  await sleep(2000);
  const at = watch.timeline(await watch.player.reading());
  const elapsed = Date.now() - since;
  const ok = at >= targetMs - TOLERANCE_MS && at <= targetMs + elapsed + TOLERANCE_MS;
  return [
    ok,
    `timeline ${clock(at)} for a ${clock(targetMs)} target, ${(elapsed / 1000).toFixed(1)} s after the request`,
  ];
}

async function reach(watch: Watch, targetMs: number, timeoutMs = 90_000): Promise<void> {
  const until = Date.now() + timeoutMs;
  while (Date.now() < until) {
    const r = await watch.player.reading();
    if (watch.timeline(r) + r.bufferedAheadMs >= targetMs + 1000) {
      return;
    }
    await sleep(500);
  }
  throw new Error(`the buffer did not reach ${clock(targetMs)} within ${timeoutMs / 1000} s`);
}

async function lines(watch: Watch, ms: number): Promise<string[]> {
  const seen = new Set<string>();
  const until = Date.now() + ms;
  while (Date.now() < until) {
    const text = (await watch.player.reading()).subtitle.trim();
    if (text) {
      seen.add(text);
    }
    await sleep(100);
  }
  return [...seen];
}

function repeated(texts: string[]): string | null {
  for (const text of texts) {
    const parts = text.split("\n").map((l) => l.trim()).filter((l) => l);
    if (new Set(parts).size !== parts.length) {
      return text;
    }
  }
  return null;
}

function timesShown(texts: string[], line: string): number {
  return Math.max(0, ...texts.map((t) => t.split("\n").filter((l) => l.trim() === line).length));
}

async function passes(watch: Watch, targetMs: number, timeoutMs = 30_000): Promise<Reading> {
  const until = Date.now() + timeoutMs;
  while (Date.now() < until) {
    const r = await watch.player.reading();
    if (watch.timeline(r) >= targetMs) {
      return r;
    }
    await sleep(250);
  }
  throw new Error(`playback did not pass ${clock(targetMs)} within ${timeoutMs / 1000} s`);
}

type Case = { action: string; applies: boolean; run: (watch: Watch) => Promise<[Status, string]> };

function imageCase(subject: Subject, image: ImageTrack): Case {
  return {
    action: `image-subtitle-${image.kind}`,
    applies: true,
    run: async (w) => {
      if (image.index === null) {
        return ["fail", `the ${image.kind} track is not offered`];
      }
      const controls = { ...OFF, subtitle: { type: "embedded" as const, index: image.index } };
      await w.begin(subject.version.id, IMAGE_LINE_MS - 2000, controls);
      await playing(w);
      const r = await passes(w, IMAGE_LINE_MS + 1000);
      const delivery = w.stream.selected?.subtitle_delivery ?? "none";
      if (r.image === null) {
        return ["pass", `delivery ${delivery}, playing at ${clock(w.timeline(r))}`];
      }
      const start = r.image.start === null ? null : w.stream.origin_ms + r.image.start * 1000;
      const ok = start !== null && Math.abs(start - IMAGE_LINE_MS) <= TOLERANCE_MS &&
        r.image.codec === IMAGE_CODECS[image.kind] && r.image.visible;
      return [
        ok ? "pass" : "fail",
        `delivery ${delivery}, line from ${start === null ? "none" : clock(start)}, codec ${
          r.image.codec ?? "none"
        }, drawn by mpv ${r.image.visible}`,
      ];
    },
  };
}

function cases(subject: Subject, engine: Engine, api: Api): Case[] {
  const v = subject.version.id;
  const subs: Controls = { ...OFF, subtitle: subject.subtitle };
  const hasSubs = subject.subtitle !== null;
  const noRepeat = (texts: string[]): [Status, string] | null => {
    const twice = repeated(texts);
    return twice ? ["fail", `a line shown more than once: ${JSON.stringify(twice)}`] : null;
  };
  return [
    {
      action: "play",
      applies: true,
      run: async (w) => {
        await w.begin(v, 0, OFF);
        const first = await playing(w);
        await sleep(10_000);
        const later = await w.player.reading();
        const advanced = w.timeline(later) - w.timeline(first);
        if (advanced < 8000) {
          return ["fail", `advanced ${(advanced / 1000).toFixed(1)} s in 10 s`];
        }
        return later.audio ? ["pass", `advanced ${(advanced / 1000).toFixed(1)} s`] : ["fail", "no audio decoded"];
      },
    },
    {
      action: "resume",
      applies: true,
      run: async (w) => {
        const since = Date.now();
        await w.begin(v, subject.resumeMs, OFF);
        const [ok, detail] = await landed(w, subject.resumeMs, since);
        return [ok ? "pass" : "fail", detail];
      },
    },
    {
      action: "far-seek",
      applies: true,
      run: async (w) => {
        await w.begin(v, 0, OFF);
        await playing(w);
        const since = Date.now();
        await w.farSeek(subject.farSeekMs);
        const [ok, detail] = await landed(w, subject.farSeekMs, since);
        return [ok ? "pass" : "fail", detail];
      },
    },
    {
      action: "near-seek",
      applies: true,
      run: async (w) => {
        await w.begin(v, 0, OFF);
        const start = w.timeline(await playing(w));
        const target = start + 30_000;
        await reach(w, target);
        const since = Date.now();
        await w.nearSeek(target);
        const [ok, detail] = await landed(w, target, since);
        return [ok ? "pass" : "fail", detail];
      },
    },
    {
      action: "subtitles-resume",
      applies: hasSubs,
      run: async (w) => {
        const since = Date.now();
        await w.begin(v, subject.resumeMs, subs);
        const [ok, detail] = await landed(w, subject.resumeMs, since);
        if (!ok) {
          return ["fail", detail];
        }
        return noRepeat(await lines(w, 6000)) ?? ["pass", detail];
      },
    },
    {
      action: "subtitles-far-seek",
      applies: hasSubs,
      run: async (w) => {
        await w.begin(v, 0, subs);
        await playing(w);
        const since = Date.now();
        await w.farSeek(subject.farSeekMs);
        const [ok, detail] = await landed(w, subject.farSeekMs, since);
        if (!ok) {
          return ["fail", detail];
        }
        return noRepeat(await lines(w, 6000)) ?? ["pass", detail];
      },
    },
    {
      action: "subtitles-seek-back",
      applies: hasSubs,
      run: async (w) => {
        const point = subject.linePointMs;
        const line = `Line at ${Math.round(point / 1000) + 2}`;
        await w.begin(v, 0, subs);
        await playing(w);
        await reach(w, point + 15_000);
        const windows: string[][] = [];
        for (let pass = 0; pass < 3; pass++) {
          await w.nearSeek(point);
          windows.push(await lines(w, subject.knownLines ? 5000 : 8000));
        }
        if (subject.knownLines) {
          const counts = windows.map((texts) => timesShown(texts, line));
          const ok = counts.every((n) => n === 1);
          return [ok ? "pass" : "fail", `[${line}] shown ${counts.join(", ")} time(s) over three passes`];
        }
        const twice = windows.map(repeated).find((t) => t !== null);
        if (twice) {
          return ["fail", `a line shown more than once: ${JSON.stringify(twice)}`];
        }
        return windows.every((t) => t.length === 0)
          ? ["skip", `no line on screen near ${clock(point)}`]
          : ["pass", "each line once over three passes"];
      },
    },
    {
      action: "subtitles-mode",
      applies: hasSubs,
      run: async () => {
        const plain = await api.start(await engine.startBody(v, 0, OFF));
        await api.end(plain.session_id);
        const withSubs = await api.start(await engine.startBody(v, 0, subs));
        await api.end(withSubs.session_id);
        const delivery = withSubs.selected?.subtitle_delivery ?? "none";
        return [
          plain.mode === withSubs.mode ? "pass" : "fail",
          `without the subtitle ${plain.mode}, with it ${withSubs.mode} (delivery ${delivery})`,
        ];
      },
    },
    {
      action: "forced-resume",
      applies: engine.forcedCases,
      run: async (w) => {
        const since = Date.now();
        await w.begin(v, subject.resumeMs, FORCED);
        const [ok, detail] = await landed(w, subject.resumeMs, since);
        return [ok ? "pass" : "fail", detail];
      },
    },
    {
      action: "forced-far-seek",
      applies: engine.forcedCases,
      run: async (w) => {
        await w.begin(v, 0, FORCED);
        await playing(w);
        const since = Date.now();
        await w.farSeek(subject.farSeekMs);
        const [ok, detail] = await landed(w, subject.farSeekMs, since);
        return [ok ? "pass" : "fail", detail];
      },
    },
    {
      action: "forced-subtitles-far-seek",
      applies: engine.forcedCases && hasSubs,
      run: async (w) => {
        await w.begin(v, 0, { ...subs, delivery: "always" });
        await playing(w);
        const since = Date.now();
        await w.farSeek(subject.farSeekMs);
        const [ok, detail] = await landed(w, subject.farSeekMs, since);
        if (!ok) {
          return ["fail", detail];
        }
        return noRepeat(await lines(w, 6000)) ?? ["pass", detail];
      },
    },
    ...(subject.knownLines ? subject.images.map((image) => imageCase(subject, image)) : []),
    {
      action: "second-audio",
      applies: subject.secondAudio !== null,
      run: async (w) => {
        const chosen = subject.secondAudio!;
        await w.begin(v, 0, { ...OFF, audioTrack: chosen });
        await playing(w);
        await sleep(1500);
        const r = await w.player.reading();
        if (r.audioIndex !== null) {
          if (w.stream.mode !== "direct") {
            return ["skip", `in a stream mpv cannot tell which source track it plays (chose ${chosen})`];
          }
          return [r.audioIndex === chosen ? "pass" : "fail", `chose stream ${chosen}, playing stream ${r.audioIndex}`];
        }
        const expected = subject.tones?.[chosen] ?? null;
        if (expected === null || r.toneHz === null) {
          return ["skip", "the player cannot tell which track plays"];
        }
        const ok = Math.abs(r.toneHz - expected) <= TONE_TOLERANCE_HZ;
        return [ok ? "pass" : "fail", `chose stream ${chosen} (${expected} Hz), hearing ${Math.round(r.toneHz)} Hz`];
      },
    },
  ];
}

function selected(subject: Subject, c: Case): boolean {
  return c.applies && (ONLY === null || ONLY.test(`${subject.label} | ${c.action}`));
}

export async function runSubject(api: Api, engine: Engine, subject: Subject, number: number): Promise<Result[]> {
  const results: Result[] = [];
  for (const c of cases(subject, engine, api).filter((c) => selected(subject, c))) {
    const player = await engine.open(`${String(number).padStart(2, "0")}-${c.action}`);
    const watch = new Watch(api, engine, player, subject.version);
    const mark = api.started.length;
    let status: Status;
    let detail: string;
    try {
      [status, detail] = await c.run(watch);
    } catch (err) {
      status = "fail";
      detail = err instanceof Error ? err.message : String(err);
    }
    const problems = await watch.end().catch((err) => [`closing failed: ${err}`]);
    if (problems.length > 0) {
      status = "fail";
      detail = `${detail}; ${problems.join("; ")}`;
    }
    const result = {
      engine: engine.name,
      subject: subject.label,
      action: c.action,
      status,
      detail,
      mode: watch.mode(),
      log: watch.player.log,
      sessions: api.started.slice(mark),
    };
    const log = status === "fail" && result.log ? ` (log ${result.log})` : "";
    console.log(
      `${
        status.toUpperCase().padEnd(4)
      } ${engine.name} | ${subject.label} | ${c.action} | ${result.mode} | ${detail}${log}`,
    );
    results.push(result);
  }
  return results;
}
