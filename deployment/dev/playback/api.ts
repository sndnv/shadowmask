export const SERVER = Deno.env.get("PLAYBACK_SERVER") ?? "http://server:8080";
const API = `${SERVER}/api/v1`;
const PAGE = 200;

export type AudioTrack = { index: number; codec: string; channels: number; language: string | null };
export type EmbeddedSubtitle = { index: number; language: string | null; format: string };
export type SubtitleFile = { id: string; language: string | null; format: string };
export type VideoTrack = { index: number; codec: string; bit_depth: number };

export type Version = {
  id: string;
  path: string;
  duration_ms: number;
  container: string;
  video: VideoTrack[];
  audio: AudioTrack[];
  subtitles: EmbeddedSubtitle[];
  subtitle_files: SubtitleFile[];
};

export type Selected = {
  audio_track?: number | null;
  audio_delivery?: string | null;
  subtitle_delivery?: string | null;
  subtitle_url?: string | null;
};

export type Negotiation = {
  session_id: string;
  mode: string;
  container: string | null;
  manifest_url: string;
  origin_ms: number;
  sequential: boolean;
  selected?: Selected;
};

export type Job = { id: string; kind: string; status: string; cancellable: boolean };

type Page<T> = { items: T[]; total: number };

export const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

async function healthy(): Promise<boolean> {
  const response = await fetch(`${SERVER}/health`).catch(() => null);
  if (!response) {
    return false;
  }
  await response.body?.cancel();
  return response.ok;
}

export class Api {
  readonly started: string[] = [];

  private constructor(private readonly token: string) {}

  static async login(timeoutMs = 180_000): Promise<Api> {
    const until = Date.now() + timeoutMs;
    while (!(await healthy())) {
      if (Date.now() > until) {
        throw new Error(`server [${SERVER}] not healthy within [${timeoutMs}] ms`);
      }
      await sleep(1000);
    }
    const response = await fetch(`${API}/auth/login`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ username: "admin", password: "passw0rd" }),
    });
    if (!response.ok) {
      throw new Error(`login answered [${response.status}]`);
    }
    const body = await response.json();
    return new Api(body.access_token);
  }

  private async call<T>(method: string, path: string, body?: unknown): Promise<T> {
    const headers: Record<string, string> = { authorization: `Bearer ${this.token}` };
    if (body !== undefined) {
      headers["content-type"] = "application/json";
    }
    const response = await fetch(`${API}${path}`, {
      method,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    const text = await response.text();
    if (!response.ok) {
      throw new Error(`${method} ${path.split("?")[0]} answered [${response.status}]: ${text.slice(0, 300)}`);
    }
    return (text ? JSON.parse(text) : undefined) as T;
  }

  private async all<T>(path: string): Promise<T[]> {
    const items: T[] = [];
    const joiner = path.includes("?") ? "&" : "?";
    while (true) {
      const page = await this.call<Page<T>>("GET", `${path}${joiner}limit=${PAGE}&offset=${items.length}`);
      items.push(...page.items);
      if (page.items.length === 0 || items.length >= page.total) {
        return items;
      }
    }
  }

  libraries(): Promise<{ id: string; name: string }[]> {
    return this.call("GET", "/libraries");
  }

  scan(library: string): Promise<void> {
    return this.call("POST", `/libraries/${library}/scan`);
  }

  activeJobs(): Promise<Job[]> {
    return this.all<Job>("/admin/jobs?state=active");
  }

  cancel(job: string): Promise<void> {
    return this.call("POST", `/admin/jobs/${job}/cancel`);
  }

  libraryVersions(library: string): Promise<{ id: string; path: string }[]> {
    return this.all(`/libraries/${library}/versions`);
  }

  version(id: string): Promise<Version> {
    return this.call("GET", `/versions/${id}`);
  }

  async profileVersion(): Promise<number> {
    const info = await this.call<{ profile_version?: number }>("GET", "/server/info").catch(() => ({}));
    return (info as { profile_version?: number }).profile_version ?? 1;
  }

  async start(body: Record<string, unknown>): Promise<Negotiation> {
    const stream = await this.call<Negotiation>("POST", "/sessions", body);
    this.started.push(stream.session_id);
    return stream;
  }

  seek(session: string, positionMs: number): Promise<Negotiation> {
    return this.call("POST", `/sessions/${session}/seek`, { position_ms: positionMs });
  }

  progress(session: string, positionMs: number): Promise<void> {
    return this.call<void>("POST", `/sessions/${session}/progress`, {
      position_ms: Math.max(0, Math.round(positionMs)),
      state: "playing",
    }).catch(() => undefined);
  }

  end(session: string): Promise<void> {
    return this.call<void>("DELETE", `/sessions/${session}`).catch(() => undefined);
  }
}
