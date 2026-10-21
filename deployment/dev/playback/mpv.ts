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
import { loadSubjects, OUT, writeResults } from "./results.ts";

const PLAYER_VIDEO: Record<string, number> = { h264: 10, hevc: 12, vp9: 12, av1: 12, mpeg4: 8, msmpeg4v3: 8 };
const PLAYER_AUDIO: Record<string, number> = {
  aac: 8,
  ac3: 6,
  eac3: 8,
  dts: 8,
  truehd: 8,
  flac: 8,
  opus: 8,
  vorbis: 8,
  mp3: 2,
  mp2: 2,
};
const PLAYER_OWN_CONTAINERS = ["mkv"];
const PLAYER_DEMUXERS: Record<string, string> = {
  mp4: "mov",
  ts: "mpegts",
  avi: "avi",
  mov: "mov",
  m4v: "mov",
  mpg: "mpeg",
  m2ts: "mpegts",
  wmv: "asf",
  flv: "flv",
};
const BUFFER_SECONDS = 60;
const BUFFER_BYTES = 256 * 1024 * 1024;
const NETWORK_TIMEOUT = 10;
const CODEC_PARAMETERS = /Could not find codec parameters/i;
const LOGS = "logs/mpv";
const SUBTITLE_DELIVERIES = ["embedded", "image", "external"];

const MEDIA_KIT_OPTIONS = [
  "--idle=yes",
  "--pause=yes",
  "--keep-open=yes",
  "--audio-display=no",
  "--network-timeout=5",
  "--scale=bilinear",
  "--dscale=bilinear",
  "--dither=no",
  "--cache=yes",
  "--cache-on-disk=yes",
  "--hr-seek=yes",
  "--hr-seek-framedrop=no",
  "--correct-downscaling=no",
  "--linear-downscaling=no",
  "--sigmoid-upscaling=no",
  "--hdr-compute-peak=no",
  "--subs-fallback=yes",
  "--subs-with-matching-audio=yes",
  "--osc=no",
  "--osd-level=0",
  "--demuxer-max-bytes=33554432",
  "--demuxer-max-back-bytes=33554432",
  "--demuxer-lavf-o=seg_max_retry=5,strict=experimental,allowed_extensions=ALL,protocol_whitelist=[udp,rtp,tcp,tls,data,file,http,https,crypto]",
  "--sub-ass=no",
  "--sub-visibility=no",
  "--secondary-sub-visibility=no",
  "--hwdec=auto",
];

type Reply = { error: string; data?: unknown };

class Ipc {
  private next = 1;
  private readonly pending = new Map<number, (reply: Reply) => void>();
  private writing: Promise<void> = Promise.resolve();

  private constructor(private readonly conn: Deno.UnixConn) {
    this.listen();
  }

  static async connect(path: string, timeoutMs = 10_000): Promise<Ipc> {
    const until = Date.now() + timeoutMs;
    while (true) {
      const conn = await Deno.connect({ transport: "unix", path }).catch(() => null);
      if (conn) {
        return new Ipc(conn);
      }
      if (Date.now() > until) {
        throw new Error(`mpv control socket [${path}] did not open`);
      }
      await sleep(100);
    }
  }

  private async listen(): Promise<void> {
    const decoder = new TextDecoder();
    let buffered = "";
    try {
      for await (const chunk of this.conn.readable) {
        buffered += decoder.decode(chunk, { stream: true });
        let newline = buffered.indexOf("\n");
        while (newline >= 0) {
          const line = buffered.slice(0, newline);
          buffered = buffered.slice(newline + 1);
          newline = buffered.indexOf("\n");
          const message = JSON.parse(line);
          const resolve = this.pending.get(message.request_id);
          if (resolve) {
            this.pending.delete(message.request_id);
            resolve(message);
          }
        }
      }
    } catch {
      return;
    }
  }

  private write(args: unknown[], id: number): Promise<void> {
    const line = new TextEncoder().encode(JSON.stringify({ command: args, request_id: id }) + "\n");
    const written = this.writing.then(async () => {
      const writer = this.conn.writable.getWriter();
      try {
        await writer.write(line);
      } finally {
        writer.releaseLock();
      }
    });
    this.writing = written.catch(() => undefined);
    return written;
  }

  async command(...args: unknown[]): Promise<Reply> {
    const id = this.next++;
    const reply = new Promise<Reply>((resolve) => this.pending.set(id, resolve));
    await this.write(args, id);
    return await reply;
  }

  async send(...args: unknown[]): Promise<void> {
    await this.write(args, this.next++).catch(() => undefined);
  }

  async get(name: string): Promise<unknown> {
    const reply = await this.command("get_property", name);
    return reply.error === "success" ? reply.data : null;
  }

  async set(name: string, value: string): Promise<void> {
    const reply = await this.command("set_property", name, value);
    if (reply.error !== "success") {
      throw new Error(`mpv refused [${name}=${value}]: ${reply.error}`);
    }
  }

  close(): void {
    try {
      this.conn.close();
    } catch {
      return;
    }
  }
}

class MpvPlayer implements Player {
  private constructor(
    private readonly process: Deno.ChildProcess,
    private readonly ipc: Ipc,
    private readonly logFile: string,
    readonly log: string | null,
  ) {}

  static async start(n: number, name: string | null): Promise<MpvPlayer> {
    const socket = `/tmp/mpv-${n}.sock`;
    const log = name ? `${OUT}/${LOGS}/${name}.log` : `/tmp/mpv-${n}.log`;
    await Deno.remove(socket).catch(() => undefined);
    await Deno.remove(log).catch(() => undefined);
    const process = new Deno.Command("mpv", {
      args: [
        "--no-config",
        "--terminal=no",
        "--vo=null",
        "--ao=null",
        `--input-ipc-server=${socket}`,
        `--log-file=${log}`,
        ...MEDIA_KIT_OPTIONS,
      ],
      stdin: "null",
      stdout: "null",
      stderr: "null",
    }).spawn();
    return new MpvPlayer(process, await Ipc.connect(socket), log, name ? `${LOGS}/${name}.log` : null);
  }

  async decoders(): Promise<Set<string>> {
    const list = (await this.ipc.get("decoder-list")) as { codec: string }[] | null;
    return new Set((list ?? []).map((d) => d.codec));
  }

  async demuxers(): Promise<Set<string>> {
    const list = (await this.ipc.get("demuxer-lavf-list")) as string[] | null;
    return new Set((list ?? []).flatMap((name) => name.split(",")).map((name) => name.trim()).filter(Boolean));
  }

  async attach(url: string, _mode: string, offsetMs: number, tracks: Tracks): Promise<void> {
    await this.ipc.set("start", "none");
    await this.ipc.set("sid", tracks.drawn ? "auto" : "no");
    await this.ipc.set("sub-visibility", tracks.subtitle?.image ? "yes" : "no");
    await this.ipc.set("network-timeout", `${NETWORK_TIMEOUT}`);
    await this.ipc.set("cache", "yes");
    await this.ipc.set("cache-secs", `${BUFFER_SECONDS}`);
    await this.ipc.set("demuxer-readahead-secs", `${BUFFER_SECONDS}`);
    await this.ipc.set("demuxer-max-bytes", `${BUFFER_BYTES}`);
    await this.ipc.set("demuxer-max-back-bytes", `${BUFFER_BYTES / 4}`);
    await this.ipc.command("stop");
    await this.ipc.command("playlist-clear");
    await this.ipc.command("playlist-play-index", "none");
    await this.ipc.set("pause", "yes");
    await this.ipc.command("loadfile", url, "append");
    await this.ipc.set("playlist-pos", "0");
    const until = Date.now() + 60_000;
    while (!(Number(await this.ipc.get("duration")) > 0)) {
      if (Date.now() > until) {
        throw new Error("mpv never learned the stream's length");
      }
      await sleep(100);
    }
    await this.select(tracks);
    if (offsetMs > 0) {
      await this.ipc.command("seek", offsetMs / 1000, "absolute");
    }
  }

  private async trackId(type: string, ffIndex: number): Promise<number> {
    const list = (await this.ipc.get("track-list")) as { id: number; type: string; "ff-index"?: number }[] | null;
    const track = (list ?? []).find((t) => t.type === type && t["ff-index"] === ffIndex);
    if (!track) {
      throw new Error(`mpv lists no ${type} track for stream ${ffIndex}`);
    }
    return track.id;
  }

  private async select(tracks: Tracks): Promise<void> {
    const subtitle = tracks.subtitle;
    if (subtitle?.delivery === "embedded" && subtitle.index !== null) {
      await this.ipc.set("sid", `${await this.trackId("sub", subtitle.index)}`);
    }
    if (subtitle?.delivery === "external" && subtitle.url) {
      const reply = await this.ipc.command("sub-add", subtitle.url, "select");
      if (reply.error !== "success") {
        throw new Error(`mpv could not load the subtitle file: ${reply.error}`);
      }
    }
    if (tracks.audioIndex !== null) {
      await this.ipc.set("aid", `${await this.trackId("audio", tracks.audioIndex)}`);
    }
  }

  async play(): Promise<void> {
    await this.ipc.set("pause", "no");
  }

  async seekTo(offsetMs: number): Promise<void> {
    await this.ipc.command("seek", offsetMs / 1000, "absolute");
  }

  async reading(): Promise<Reading> {
    const position = Number(await this.ipc.get("time-pos")) || 0;
    const cached = Number(await this.ipc.get("demuxer-cache-time")) || 0;
    const channels = Number(await this.ipc.get("audio-params/channel-count")) || 0;
    const index = await this.ipc.get("current-tracks/audio/ff-index");
    const start = await this.ipc.get("sub-start");
    const codec = await this.ipc.get("current-tracks/sub/codec");
    return {
      positionMs: position * 1000,
      bufferedAheadMs: Math.max(0, (cached - position) * 1000),
      subtitle: String((await this.ipc.get("sub-text")) ?? ""),
      audio: channels > 0,
      audioIndex: typeof index === "number" ? index : null,
      toneHz: null,
      image: {
        start: typeof start === "number" ? start : null,
        codec: typeof codec === "string" ? codec : null,
        visible: (await this.ipc.get("sub-visibility")) === true,
      },
    };
  }

  async close(): Promise<string[]> {
    await this.ipc.send("quit");
    const exited = await Promise.race([this.process.status.then(() => true), sleep(5000).then(() => false)]);
    if (!exited) {
      this.process.kill("SIGKILL");
      await this.process.status;
    }
    this.ipc.close();
    const text = await Deno.readTextFile(this.logFile).catch(() => "");
    const found = text.split("\n").filter((line) => CODEC_PARAMETERS.test(line));
    return found.length > 0
      ? [`mpv: ${found.length} "could not find codec parameters", first: ${found[0].trim()}`]
      : [];
  }
}

class MpvEngine implements Engine {
  readonly name = "mpv";
  readonly forcedCases = true;
  private opened = 0;
  private decoding: Record<string, unknown> | null = null;

  constructor(private readonly profileVersion: number) {}

  private async capabilities(): Promise<Record<string, unknown>> {
    if (!this.decoding) {
      const probe = await MpvPlayer.start(this.opened++, null);
      const decoders = await probe.decoders();
      const demuxers = await probe.demuxers();
      await probe.close();
      this.decoding = {
        ...(demuxers.size > 0
          ? {
            containers: [
              ...PLAYER_OWN_CONTAINERS,
              ...Object.entries(PLAYER_DEMUXERS).filter(([, demuxer]) => demuxers.has(demuxer)).map(([name]) => name),
            ],
          }
          : {}),
        video: Object.entries(PLAYER_VIDEO)
          .filter(([codec]) => decoders.has(codec))
          .map(([codec, depth]) => ({ codec, max_bit_depth: depth, smooth: true })),
        audio: Object.entries(PLAYER_AUDIO)
          .filter(([codec]) => decoders.has(codec))
          .map(([codec, channels]) => ({ codec, max_channels: channels })),
      };
      console.log(`mpv's capability report: ${JSON.stringify(this.decoding)}`);
    }
    return this.decoding;
  }

  async startBody(versionId: string, startMs: number, controls: Controls): Promise<Record<string, unknown>> {
    return {
      version_id: versionId,
      start_position_ms: startMs,
      capabilities: {
        platform: "desktop",
        profile_version: this.profileVersion,
        decoding: await this.capabilities(),
        subtitle_deliveries: SUBTITLE_DELIVERIES,
        audio_tracks: true,
      },
      ...(controls.audioTrack !== null ? { audio_track: controls.audioTrack } : {}),
      ...(controls.subtitle ? { subtitle: { track: controls.subtitle } } : { subtitle_off: true }),
      ...(controls.delivery ? { delivery: controls.delivery } : {}),
    };
  }

  loadOffset(stream: Negotiation, atMs: number): number {
    return stream.sequential && stream.container === "fmp4" ? 0 : atMs - stream.origin_ms;
  }

  drawsSubtitles(stream: Negotiation): boolean {
    return stream.selected?.subtitle_delivery === "hls_vtt";
  }

  open(name: string): Promise<Player> {
    return MpvPlayer.start(this.opened++, name);
  }
}

if (import.meta.main) {
  await Deno.mkdir(`${OUT}/${LOGS}`, { recursive: true });
  const api = await Api.login();
  const engine = new MpvEngine(await api.profileVersion());
  const { versions, labels, library } = await loadSubjects();
  const subjects = library ? librarySubjects(versions, labels) : fixtureSubjects(versions);
  const results: Result[] = [];
  for (const [i, subject] of subjects.entries()) {
    results.push(...await runSubject(api, engine, subject, i + 1));
  }
  await writeResults("mpv", results);
}
