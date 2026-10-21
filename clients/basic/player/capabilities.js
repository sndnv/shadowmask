export const FULL_HD = 1080;
export const ULTRA_HD = 2160;

export const PROBE_SIZES = {
  [FULL_HD]: { width: 1920, bitrate: 8000000 },
  [ULTRA_HD]: { width: 3840, bitrate: 25000000 },
};

export const PROBE_FRAME_RATE = 30;

export const VIDEO_QUESTIONS = [
  { codec: "h264", bitDepth: 10, height: FULL_HD, contentType: 'video/mp4; codecs="avc1.6e0028"' },
  { codec: "h264", bitDepth: 8, height: FULL_HD, contentType: 'video/mp4; codecs="avc1.640028"' },
  { codec: "h264", bitDepth: 8, height: ULTRA_HD, contentType: 'video/mp4; codecs="avc1.640033"' },
  { codec: "hevc", bitDepth: 10, height: FULL_HD, contentType: 'video/mp4; codecs="hvc1.2.4.L120.B0"' },
  { codec: "hevc", bitDepth: 8, height: FULL_HD, contentType: 'video/mp4; codecs="hvc1.1.6.L120.B0"' },
  { codec: "hevc", bitDepth: 10, height: ULTRA_HD, contentType: 'video/mp4; codecs="hvc1.2.4.L153.B0"' },
  { codec: "vp9", bitDepth: 10, height: FULL_HD, contentType: 'video/webm; codecs="vp09.02.40.10"' },
  { codec: "vp9", bitDepth: 8, height: FULL_HD, contentType: 'video/webm; codecs="vp09.00.40.08"' },
  { codec: "vp9", bitDepth: 10, height: ULTRA_HD, contentType: 'video/webm; codecs="vp09.02.51.10"' },
  { codec: "av1", bitDepth: 10, height: FULL_HD, contentType: 'video/mp4; codecs="av01.0.08M.10"' },
  { codec: "av1", bitDepth: 8, height: FULL_HD, contentType: 'video/mp4; codecs="av01.0.08M.08"' },
  { codec: "av1", bitDepth: 10, height: ULTRA_HD, contentType: 'video/mp4; codecs="av01.0.13M.10"' },
  { codec: "mpeg4", bitDepth: 8, height: FULL_HD, contentType: 'video/mp4; codecs="mp4v.20.9"' },
];

export const UNASKABLE_VIDEO = ["msmpeg4v3"];

export const AUDIO_QUESTIONS = [
  { codec: "aac", contentType: 'audio/mp4; codecs="mp4a.40.2"' },
  { codec: "mp3", contentType: "audio/mpeg" },
  { codec: "ac3", contentType: 'audio/mp4; codecs="ac-3"' },
  { codec: "eac3", contentType: 'audio/mp4; codecs="ec-3"' },
  { codec: "dts", contentType: 'audio/mp4; codecs="dtsc"' },
  { codec: "truehd", contentType: 'audio/mp4; codecs="mlpa"' },
  { codec: "flac", contentType: 'audio/mp4; codecs="flac"' },
  { codec: "opus", contentType: 'audio/mp4; codecs="opus"' },
  { codec: "vorbis", contentType: 'audio/mp4; codecs="vorbis"' },
  { codec: "mp2", contentType: 'audio/mp4; codecs="mp4a.6B"' },
];

export const AUDIO_CHANNELS = [2, 6, 8];

const HDR_CONTENT_TYPES = [
  'video/mp4; codecs="hvc1.2.4.L153.B0"',
  'video/webm; codecs="vp09.02.51.10"',
  'video/mp4; codecs="av01.0.13M.10"',
];

export const HDR_COLOR_GAMUT = "rec2020";

export const HDR_QUESTIONS = HDR_CONTENT_TYPES.flatMap((contentType) => [
  { format: "hdr10", contentType, metadata: "smpteSt2086", transfer: "pq" },
  { format: "hdr10plus", contentType, metadata: "smpteSt2094-40", transfer: "pq" },
  { format: "hlg", contentType, metadata: null, transfer: "hlg" },
]);

const REFUSED = { supported: false, smooth: false, powerEfficient: false };

const plays = (a) => a.file.supported && a.stream.supported;
const smooth = (a) => a.file.smooth && a.stream.smooth;
const hardware = (a) => a.file.powerEfficient && a.stream.powerEfficient;

function ceiling(answers) {
  let top = 0;
  for (const a of answers) {
    if (!plays(a) || !smooth(a)) continue;
    const counts = a.question.height <= FULL_HD || hardware(a);
    if (counts && a.question.height > top) top = a.question.height;
  }
  return top;
}

function videoCaps(answers) {
  const codecs = [];
  for (const a of answers) {
    if (plays(a) && !codecs.includes(a.question.codec)) codecs.push(a.question.codec);
  }
  const ceilings = new Map(codecs.map((codec) => [codec, ceiling(answers.filter((a) => a.question.codec === codec))]));
  const top = Math.max(0, ...ceilings.values());
  return codecs.map((codec) => ({
    codec,
    max_bit_depth: Math.max(
      ...answers.filter((a) => a.question.codec === codec && plays(a)).map((a) => a.question.bitDepth),
    ),
    smooth: top > 0 && ceilings.get(codec) === top,
  }));
}

function audioCaps(answers) {
  const channels = new Map();
  for (const a of answers) {
    if (!plays(a)) continue;
    channels.set(a.question.codec, Math.max(channels.get(a.question.codec) || 0, a.channels));
  }
  return [...channels].map(([codec, max]) => ({ codec, max_channels: max }));
}

function hdrFormats(answers) {
  const formats = [];
  for (const a of answers) {
    if (plays(a) && !formats.includes(a.question.format)) formats.push(a.question.format);
  }
  return formats;
}

export function browserReport({ video, audio, hdr }) {
  const report = {};
  const codecs = videoCaps(video);
  const sound = audioCaps(audio);
  if (codecs.length > 0) report.video = codecs;
  if (sound.length > 0) report.audio = sound;
  if (!report.video && !report.audio) return null;
  report.hdr = hdrFormats(hdr);
  const top = ceiling(video);
  if (top > 0) report.max_height = top;
  return report;
}

function videoConfig(question) {
  const size = PROBE_SIZES[question.height];
  return {
    contentType: question.contentType,
    width: size.width,
    height: question.height,
    bitrate: size.bitrate,
    framerate: PROBE_FRAME_RATE,
  };
}

function hdrConfig(question) {
  const config = Object.assign(videoConfig({ contentType: question.contentType, height: ULTRA_HD }), {
    colorGamut: HDR_COLOR_GAMUT,
    transferFunction: question.transfer,
  });
  if (question.metadata) config.hdrMetadataType = question.metadata;
  return config;
}

function audioConfig(question, channels) {
  return { contentType: question.contentType, channels: String(channels), bitrate: 256000, samplerate: 48000 };
}

async function ask(capabilities, type, media) {
  try {
    const info = await capabilities.decodingInfo(Object.assign({ type }, media));
    return { supported: !!info.supported, smooth: !!info.smooth, powerEfficient: !!info.powerEfficient };
  } catch {
    return REFUSED;
  }
}

async function both(capabilities, question, media, extra) {
  return Object.assign({
    question,
    file: await ask(capabilities, "file", media),
    stream: await ask(capabilities, "media-source", media),
  }, extra);
}

export async function measureDecoding(capabilities = navigator.mediaCapabilities) {
  if (!capabilities || typeof capabilities.decodingInfo !== "function") return null;
  const video = [];
  for (const q of VIDEO_QUESTIONS) video.push(await both(capabilities, q, { video: videoConfig(q) }));
  const audio = [];
  for (const q of AUDIO_QUESTIONS) {
    for (const channels of AUDIO_CHANNELS) {
      audio.push(await both(capabilities, q, { audio: audioConfig(q, channels) }, { channels }));
    }
  }
  const hdr = [];
  for (const q of HDR_QUESTIONS) hdr.push(await both(capabilities, q, { video: hdrConfig(q) }));
  return browserReport({ video, audio, hdr });
}
