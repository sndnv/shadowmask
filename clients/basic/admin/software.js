const LABELS = {
  transcription: "Transcription",
  translation: "Translation",
  upscaling: "Upscaling",
  opensubtitles: "Subtitle search",
  tmdb: "Metadata",
  tls: "HTTPS",
  webhooks: "Webhooks",
  content_fetch: "Content fetch",
  hardware_transcode: "Hardware transcoding",
};

export function featureLabel(name) {
  return Object.hasOwn(LABELS, name) ? LABELS[name] : name;
}

export function featureState(feature) {
  if (feature.enabled) return "On";
  return feature.available ? "Off" : "Not available";
}

export function featureFacts(features) {
  return features.map((f) => [featureLabel(f.name), featureState(f)]);
}

export function uptimeText(ms) {
  const total = Math.floor(Math.max(ms, 0) / 60000);
  const days = Math.floor(total / 1440);
  const hours = Math.floor((total % 1440) / 60);
  const minutes = total % 60;
  const pad2 = (n) => String(n).padStart(2, "0");
  if (days > 0) return days + "d " + pad2(hours) + "h";
  return hours > 0 ? hours + "h " + pad2(minutes) + "m" : minutes + "m";
}

export function uptimeParts(info, time) {
  if (!info || typeof info.uptime_ms !== "number" || !info.started_at) return null;
  return [time(info.started_at), uptimeText(info.uptime_ms) + " ago"];
}

export function versionFacts(software, capabilities) {
  const ffmpeg = capabilities && capabilities.ffmpeg ? capabilities.ffmpeg.version : null;
  return [
    ["Server", software ? software.version : null],
    ["FFmpeg", ffmpeg],
    ["yt-dlp", software ? software.yt_dlp : null],
  ];
}
