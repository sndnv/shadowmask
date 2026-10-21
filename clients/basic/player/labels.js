const NAMES = new Intl.DisplayNames(["en"], { type: "language", languageDisplay: "standard" });
const REGIONAL = { "zh-CN": "Chinese (Simplified)", "zh-TW": "Chinese (Traditional)" };
const MACHINE_MADE = { machine_translated: "Translated", generated: "Generated" };

export function languageName(code) {
  if (!code || code === "und") return "Unknown";
  if (code.includes("+")) return code.split("+").map(languageName).join(" + ");
  if (REGIONAL[code]) return REGIONAL[code];
  try {
    return NAMES.of(code) || code;
  } catch {
    return code;
  }
}

function joined(parts) {
  return parts.filter((part) => part).join(" · ");
}

function numbered(labels) {
  const seen = new Map();
  return labels.map((label) => {
    const count = (seen.get(label) || 0) + 1;
    seen.set(label, count);
    return count === 1 ? label : label + " " + count;
  });
}

export function audioLabels(version) {
  const tracks = (version && version.audio) || [];
  return numbered(tracks.map((a) => joined([languageName(a.language), a.title])));
}

export function subtitleLabels(version) {
  const tracks = ((version && version.subtitles) || []).map((s) =>
    joined([languageName(s.language), s.title, s.forced ? "Forced" : null])
  );
  const files = ((version && version.subtitle_files) || []).map((f) =>
    joined([languageName(f.language), f.label, MACHINE_MADE[f.source]])
  );
  return numbered(tracks.concat(files));
}
