import type { Version } from "./api.ts";

export type SubtitleTrack = { type: "embedded"; index: number } | { type: "file"; id: string };

export type ImageKind = "pgs" | "dvb";

export type ImageTrack = { kind: ImageKind; index: number | null };

export type Subject = {
  label: string;
  version: Version;
  subtitle: SubtitleTrack | null;
  knownLines: boolean;
  secondAudio: number | null;
  tones: Record<number, number> | null;
  images: ImageTrack[];
  resumeMs: number;
  farSeekMs: number;
  linePointMs: number;
};

export const FIXTURES: Record<string, string> = {
  "Harbor Lights (2001)": "F1 H.264, AAC stereo",
  "Deep Field (2002)": "F2 H.264, AC3 5.1(side)",
  "Cold Cases (2003)": "F3 H.264, E-AC3 5.1(side)",
  "Sunrise (2004)": "F4 H.264, DTS 5.1(side)",
  "Silent Signal (2005)": "F5 HEVC 10-bit, E-AC3 5.1(side), embedded SRT",
  "Neon Harbor (2006)": "F6 H.264, AAC stereo + AC3 5.1(side), external SRT",
  "Paper Skies (2007)": "F7 H.264, AAC stereo x2, PGS + DVB",
  "Old Reel (2008)": "F8 MPEG-4 B-frames, MP3 stereo, AVI",
  "Night Ferry (2009)": "F9 MS MPEG-4 v3, MP3 stereo, AVI",
  "Glass Garden (2010)": "F10 MPEG-4 B-frames, AC3 5.1(side)",
  "Low Tide (2011)": "F11 VP9, Opus stereo, WebM",
  "Iron Bridge (2012)": "F12 H.264, FLAC stereo",
  "Blue Hour (2013)": "F13 H.264, AAC stereo, MP4",
  "Copper Canyon (2014)": "F14 H.264, AAC stereo, AVI",
};

const TONES: Record<string, number[]> = {
  "Harbor Lights (2001)": [440],
  "Deep Field (2002)": [330],
  "Cold Cases (2003)": [392],
  "Sunrise (2004)": [523],
  "Silent Signal (2005)": [294],
  "Neon Harbor (2006)": [587, 660],
  "Paper Skies (2007)": [698, 784],
  "Old Reel (2008)": [349],
  "Night Ferry (2009)": [466],
  "Glass Garden (2010)": [554],
  "Low Tide (2011)": [622],
  "Iron Bridge (2012)": [740],
  "Blue Hour (2013)": [831],
  "Copper Canyon (2014)": [880],
};

const IMAGES: Record<string, ImageKind[]> = { "Paper Skies (2007)": ["pgs", "dvb"] };

const TEXT_FORMATS = new Set(["srt", "subrip", "ass", "ssa", "vtt", "webvtt", "mov_text", "text"]);

export function subtitleOf(version: Version): SubtitleTrack | null {
  const embedded = version.subtitles.find((s) => TEXT_FORMATS.has(s.format.toLowerCase()));
  if (embedded) {
    return { type: "embedded", index: embedded.index };
  }
  const file = version.subtitle_files.find((f) => TEXT_FORMATS.has(f.format.toLowerCase()));
  return file ? { type: "file", id: file.id } : null;
}

export function secondAudioOf(version: Version): number | null {
  return version.audio.length > 1 ? version.audio[1].index : null;
}

function folderOf(path: string): string {
  const parts = path.split("/");
  return parts[parts.length - 2] ?? path;
}

function tonesOf(version: Version, folder: string): Record<number, number> {
  return Object.fromEntries(version.audio.map((track, i) => [track.index, TONES[folder][i]]));
}

function imagesOf(version: Version, folder: string): ImageTrack[] {
  return (IMAGES[folder] ?? []).map((kind) => ({
    kind,
    index: version.subtitles.find((s) => s.format.toLowerCase() === kind)?.index ?? null,
  }));
}

export function fixtureSubjects(versions: Version[]): Subject[] {
  return Object.keys(FIXTURES).flatMap((folder) => {
    const version = versions.find((v) => folderOf(v.path) === folder);
    if (!version) {
      throw new Error(`test file [${folder}] was not scanned`);
    }
    return [{
      label: FIXTURES[folder],
      version,
      subtitle: subtitleOf(version),
      knownLines: true,
      secondAudio: secondAudioOf(version),
      tones: tonesOf(version, folder),
      images: imagesOf(version, folder),
      resumeMs: 1_200_000,
      farSeekMs: 600_000,
      linePointMs: 58_000,
    }];
  });
}

export function librarySubjects(versions: Version[], labels: Record<string, string>): Subject[] {
  return versions.map((version) => {
    const duration = version.duration_ms;
    return {
      label: labels[version.path] ?? version.path,
      version,
      subtitle: subtitleOf(version),
      knownLines: false,
      secondAudio: secondAudioOf(version),
      tones: null,
      images: [],
      resumeMs: Math.min(1_200_000, Math.round(duration * 0.5)),
      farSeekMs: Math.round(duration * 0.4),
      linePointMs: Math.round(duration * 0.25),
    };
  });
}
