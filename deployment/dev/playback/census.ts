import { OUT } from "./results.ts";

const ROOTS = { movie: "/media/movies", tv: "/media/tv" } as const;
const PER_SHAPE = Number(Deno.env.get("PLAYBACK_PER_SHAPE") ?? "2");
const VIDEO_EXT = new Set([
  "mkv",
  "mp4",
  "m4v",
  "avi",
  "mov",
  "ts",
  "m2ts",
  "mts",
  "webm",
  "wmv",
  "mpg",
  "mpeg",
  "ogv",
]);
const SUB_EXT = new Set(["srt", "ass", "ssa", "vtt", "sub", "idx", "sup"]);
const TEXT_CODECS = new Set(["subrip", "ass", "ssa", "webvtt", "mov_text", "text"]);
const PROBES_AT_ONCE = 4;

type Stream = {
  codec_type: string;
  codec_name?: string;
  pix_fmt?: string;
  bits_per_raw_sample?: string;
  color_transfer?: string;
  channel_layout?: string;
  channels?: number;
};
type Probe = { format?: { format_name?: string }; streams?: Stream[] };
type Entry = { path: string; kind: keyof typeof ROOTS; shape: string };

async function run(args: string[]): Promise<string> {
  const output = await new Deno.Command("ffprobe", { args, stdout: "piped", stderr: "null" }).output();
  return new TextDecoder().decode(output.stdout);
}

async function pixelDepths(): Promise<Map<string, number>> {
  const table = JSON.parse(await run(["-v", "error", "-show_pixel_formats", "-of", "json"]));
  const depths = new Map<string, number>();
  for (const format of table.pixel_formats ?? []) {
    const bits = (format.components ?? []).map((c: { bit_depth: number }) => c.bit_depth);
    depths.set(format.name, bits.length > 0 ? Math.max(...bits) : 8);
  }
  return depths;
}

async function walk(dir: string, found: string[]): Promise<void> {
  for await (const entry of Deno.readDir(dir)) {
    if (entry.name.startsWith(".")) {
      continue;
    }
    const path = `${dir}/${entry.name}`;
    if (entry.isDirectory) {
      await walk(path, found);
    } else if (entry.isFile && VIDEO_EXT.has(entry.name.split(".").pop()?.toLowerCase() ?? "")) {
      found.push(path);
    }
  }
}

async function siblings(path: string): Promise<boolean> {
  const dir = path.slice(0, path.lastIndexOf("/"));
  const base = path.slice(dir.length + 1, path.lastIndexOf("."));
  for await (const entry of Deno.readDir(dir)) {
    const ext = entry.name.split(".").pop()?.toLowerCase() ?? "";
    if (entry.isFile && entry.name.startsWith(base) && SUB_EXT.has(ext)) {
      return true;
    }
  }
  return false;
}

async function shapeOf(path: string, depths: Map<string, number>): Promise<string> {
  const probe: Probe = JSON.parse(
    await run(["-v", "error", "-show_format", "-show_streams", "-of", "json", path]) || "{}",
  );
  const streams = probe.streams ?? [];
  const container = (probe.format?.format_name ?? "unknown").split(",")[0];
  const video = streams.find((s) => s.codec_type === "video" && s.codec_name !== "mjpeg" && s.codec_name !== "png");
  const audio = streams.filter((s) => s.codec_type === "audio");
  const subs = streams.filter((s) => s.codec_type === "subtitle");
  const depth = Number(video?.bits_per_raw_sample) || depths.get(video?.pix_fmt ?? "") || 8;
  const transfer = video?.color_transfer ?? "";
  const hdr = transfer === "smpte2084" ? "hdr10" : transfer === "arib-std-b67" ? "hlg" : "sdr";
  const first = audio[0];
  const layout = first ? (first.channel_layout ?? `${first.channels ?? "?"}ch`) : "";
  const sound = first ? `${first.codec_name} ${layout}${audio.length > 1 ? ` +${audio.length - 1}` : ""}` : "no audio";
  const kinds = new Set<string>();
  for (const s of subs) {
    kinds.add(TEXT_CODECS.has(s.codec_name ?? "") ? "embedded text" : "embedded image");
  }
  if (await siblings(path)) {
    kinds.add("external");
  }
  const subtitles = kinds.size > 0 ? [...kinds].sort().join(", ") : "none";
  return `${container} | ${video?.codec_name ?? "no video"} ${depth}-bit ${hdr} | ${sound} | subtitles: ${subtitles}`;
}

function rootOf(entry: Entry): string | null {
  const base = ROOTS[entry.kind];
  const rest = entry.path.slice(base.length + 1).split("/");
  if (rest.length < 2) {
    return null;
  }
  return entry.kind === "movie" ? entry.path.slice(0, entry.path.lastIndexOf("/")) : `${base}/${rest[0]}`;
}

function spread<T>(items: T[], n: number): T[] {
  if (items.length <= n) {
    return items;
  }
  return Array.from({ length: n }, (_, i) => items[Math.floor((i * items.length) / n)]);
}

const depths = await pixelDepths();
const entries: Entry[] = [];
for (const kind of ["movie", "tv"] as const) {
  const files: string[] = [];
  await walk(ROOTS[kind], files).catch(() => undefined);
  files.sort();
  for (let i = 0; i < files.length; i += PROBES_AT_ONCE) {
    const batch = files.slice(i, i + PROBES_AT_ONCE);
    const shapes = await Promise.all(batch.map((path) => shapeOf(path, depths).catch(() => "unreadable")));
    batch.forEach((path, j) => entries.push({ path, kind, shape: shapes[j] }));
  }
  console.log(`probed [${files.length}] ${kind} file(s)`);
}

const byShape = new Map<string, Entry[]>();
for (const entry of entries.filter((e) => e.shape !== "unreadable")) {
  byShape.set(entry.shape, [...(byShape.get(entry.shape) ?? []), entry]);
}

const samples: Record<string, string> = {};
const roots = { movie: new Set<string>(), tv: new Set<string>() };
const census = [...byShape.entries()]
  .sort((a, b) => b[1].length - a[1].length)
  .map(([shape, members]) => {
    const picked = spread(members.filter((m) => rootOf(m) !== null), PER_SHAPE);
    for (const entry of picked) {
      samples[entry.path] = `${shape} | ${entry.path.slice(ROOTS[entry.kind].length + 1)}`;
      roots[entry.kind].add(rootOf(entry)!);
    }
    return { shape, count: members.length, samples: picked.map((p) => p.path) };
  });

const libraries = (["movie", "tv"] as const)
  .filter((kind) => roots[kind].size > 0)
  .map((kind) => ({
    name: kind === "movie" ? "Movies" : "TV",
    kind,
    roots: [...roots[kind]].sort(),
  }));
const toml = libraries.map((l) =>
  `[[libraries]]\nname = "${l.name}"\nkind = "${l.kind}"\nroots = [${
    l.roots.map((r) => JSON.stringify(r)).join(", ")
  }]\nwatcher = "manual"\nmetadata_sources = []\n`
).join("\n");
const users =
  `[[users]]\nusername = "\${SHADOWMASK_ADMIN_USERNAME:admin}"\npassword = "\${SHADOWMASK_ADMIN_PASSWORD}"\nrole = "admin"\nlibraries = [${
    libraries.map((l) => JSON.stringify(l.name)).join(", ")
  }]\n`;

await Deno.mkdir(`${OUT}/bootstrap`, { recursive: true });
await Deno.writeTextFile(`${OUT}/bootstrap/libraries.toml`, toml);
await Deno.writeTextFile(`${OUT}/bootstrap/users.toml`, users);
await Deno.writeTextFile(`${OUT}/samples.json`, JSON.stringify(samples, null, 2));
await Deno.writeTextFile(
  `${OUT}/census.json`,
  JSON.stringify(
    {
      files: entries.length,
      unreadable: entries.filter((e) => e.shape === "unreadable").length,
      shapes: census,
    },
    null,
    2,
  ),
);
const lines = census.map((c) => `| ${c.count} | ${c.shape} | ${c.samples.length} |`);
await Deno.writeTextFile(
  `${OUT}/census.md`,
  `# Census\n\n${entries.length} file(s), ${census.length} shape(s)\n\n| Files | Shape | Sampled |\n|---|---|---|\n${
    lines.join("\n")
  }\n`,
);
console.log(`[${entries.length}] file(s) in [${census.length}] shape(s); [${Object.keys(samples).length}] sampled`);
if (Object.keys(samples).length === 0) {
  Deno.exit(1);
}
