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

export function versionFacts(software, capabilities) {
  const ffmpeg = capabilities && capabilities.ffmpeg ? capabilities.ffmpeg.version : null;
  return [
    ["Server", software ? software.version : null],
    ["FFmpeg", ffmpeg],
    ["yt-dlp", software ? software.yt_dlp : null],
  ];
}
