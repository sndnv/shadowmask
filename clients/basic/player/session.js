export const SEEK_REACH_MS = 30000;
export const SEEK_SETTLE_MS = 1000;
export const SEEK_SETTLE_WAIT_MS = 2000;
export const REQUEST_TIMEOUT_S = 120;
export const LOAD_TIMEOUT_MARGIN_S = 5;
export const SLOW_S = 20;
export const NETWORK_TIMEOUTS = [10, 30, 60, 90];
export const DELIVERY_MODES = ["auto", "never", "always"];

const AUTO = { kind: "auto" };
const OFF = { kind: "off" };

function asInt(value) {
  if (value == null || value === "") return null;
  const n = parseInt(value, 10);
  return Number.isFinite(n) ? n : null;
}

function trackOf(value) {
  if (!value) return null;
  return value.type === "embedded"
    ? { type: "embedded", index: value.index }
    : { type: "file", id: value.id };
}

export function trackFromWire(value) {
  if (!value) return null;
  const sep = value.indexOf(":");
  if (sep < 0) return null;
  const key = value.slice(sep + 1);
  return value.slice(0, sep) === "embedded"
    ? { type: "embedded", index: asInt(key) || 0 }
    : { type: "file", id: key };
}

export function trackToWire(track) {
  return track.type === "embedded" ? "embedded:" + track.index : "file:" + track.id;
}

export function controlsFromQuery(params) {
  const track = trackFromWire(params.get("sub"));
  const delivery = params.get("delivery");
  return {
    height: asInt(params.get("height")),
    burn: params.get("burn") === "1",
    downmix: params.get("downmix") === "1",
    delivery: DELIVERY_MODES.indexOf(delivery) > 0 ? delivery : "auto",
    audioTrack: asInt(params.get("audio")),
    subtitle: track ? { kind: "on", track } : params.get("soff") === "1" ? OFF : AUTO,
    offsetMs: asInt(params.get("offset")) || 0,
  };
}

export function controlsToQuery(versionId, remote) {
  const p = new URLSearchParams();
  p.set("version", versionId);
  if (remote.height != null) p.set("height", String(remote.height));
  if (remote.burn) p.set("burn", "1");
  if (remote.downmix) p.set("downmix", "1");
  if (remote.delivery !== "auto") p.set("delivery", remote.delivery);
  if (remote.audioTrack != null) p.set("audio", String(remote.audioTrack));
  if (remote.subtitle.kind === "on") p.set("sub", trackToWire(remote.subtitle.track));
  if (remote.subtitle.kind === "off") p.set("soff", "1");
  if (remote.offsetMs) p.set("offset", String(remote.offsetMs));
  return p.toString();
}

function subtitleSet(remote) {
  return Object.assign(
    { track: remote.subtitle.track },
    remote.offsetMs ? { offset_ms: remote.offsetMs } : {},
  );
}

export function startBody(remote) {
  const body = {};
  if (remote.height != null) body.target_height = remote.height;
  if (remote.burn) body.force_burn = true;
  if (remote.downmix) body.downmix_stereo = true;
  if (remote.audioTrack != null) body.audio_track = remote.audioTrack;
  if (remote.subtitle.kind === "on") body.subtitle = subtitleSet(remote);
  if (remote.subtitle.kind === "off") body.subtitle_off = true;
  if (remote.delivery !== "auto") body.delivery = remote.delivery;
  return body;
}

export function updateBody(remote) {
  const body = {
    target_height: remote.height,
    force_burn: remote.burn,
    downmix_stereo: remote.downmix,
    delivery: remote.delivery,
  };
  if (remote.audioTrack != null) body.audio_track = remote.audioTrack;
  body.subtitle = remote.subtitle.kind === "on"
    ? Object.assign({ action: "set" }, subtitleSet(remote))
    : { action: "disable" };
  return body;
}

export function sameUpdate(a, b) {
  return JSON.stringify(updateBody(a)) === JSON.stringify(updateBody(b));
}

export function withSelection(remote, selected) {
  if (!selected) return remote;
  const track = trackOf(selected.subtitle_track);
  return Object.assign({}, remote, {
    audioTrack: selected.audio_track != null ? selected.audio_track : remote.audioTrack,
    subtitle: track ? { kind: "on", track } : remote.subtitle,
  });
}

export function opens({ versionId, remote, local, playing }) {
  return {
    versionId,
    durationMs: 0,
    timeline: {
      positionMs: 0,
      seekingToMs: null,
      reachMs: 0,
      bufferedAheadMs: 0,
      playing,
      ended: false,
    },
    local,
    remote,
    stream: null,
    status: {
      life: "open",
      gathered: false,
      sent: null,
      lastChangeId: 0,
      slow: false,
      error: null,
      heldError: null,
      notice: null,
    },
  };
}

export function watching(s) {
  return s.status.life === "open" && s.status.sent == null && s.stream != null;
}

export function reportsPlaying(s) {
  return s.timeline.playing && s.status.sent == null;
}

function same(s) {
  return [s, []];
}

function withTimeline(s, changes) {
  return Object.assign({}, s, { timeline: Object.assign({}, s.timeline, changes) });
}

function withStatus(s, changes) {
  return Object.assign({}, s, { status: Object.assign({}, s.status, changes) });
}

function withProgress(before, [after, commands]) {
  const stream = after.stream;
  if (!stream || !watching(after) || reportsPlaying(after) === reportsPlaying(before)) {
    return [after, commands];
  }
  return [
    after,
    commands.concat({
      kind: "sendProgress",
      sessionId: stream.sessionId,
      positionMs: after.timeline.positionMs,
      playing: reportsPlaying(after),
    }),
  ];
}

function endOf(s, stream) {
  return {
    kind: "endSession",
    sessionId: stream.sessionId,
    positionMs: s.timeline.positionMs,
    playing: reportsPlaying(s),
  };
}

function streamFails(s, stream, message) {
  return [
    withStatus(Object.assign({}, s, { stream: null }), {
      sent: null,
      slow: false,
      error: message,
      heldError: null,
    }),
    [
      {
        kind: "endSession",
        sessionId: stream.sessionId,
        positionMs: s.timeline.positionMs,
        playing: false,
      },
      { kind: "unloadPlayer" },
    ],
  ];
}

function changing(s, sent) {
  return Object.assign({}, s.status, { lastChangeId: sent.id, notice: null, sent });
}

function reachable(s, stream, targetMs) {
  if (!stream.sequential) return true;
  if (targetMs < stream.originMs) return false;
  return targetMs <= s.timeline.reachMs + SEEK_REACH_MS;
}

function withinTitle(s, positionMs) {
  const floor = Math.max(positionMs, 0);
  return s.durationMs > 0 ? Math.min(floor, s.durationMs) : floor;
}

function viewerSeeks(s, positionMs) {
  const stream = s.stream;
  if (!watching(s)) return same(s);
  const target = withinTitle(s, positionMs);
  if (reachable(s, stream, target)) {
    return [
      withTimeline(s, { positionMs: target, seekingToMs: target, ended: false }),
      [{ kind: "seekPlayer", offsetMs: target - stream.originMs }],
    ];
  }
  return serverSeek(s, stream, target);
}

function playerSeeks(s, offsetMs) {
  const stream = s.stream;
  if (!watching(s)) return same(s);
  const target = withinTitle(s, stream.originMs + offsetMs);
  return reachable(s, stream, target) ? same(s) : serverSeek(s, stream, target);
}

function serverSeek(s, stream, target) {
  const sent = {
    kind: "seek",
    id: s.status.lastChangeId + 1,
    positionMs: target,
    previousMs: s.timeline.positionMs,
    previousPlaying: s.timeline.playing,
  };
  return [
    Object.assign(
      {},
      withTimeline(s, { positionMs: target, seekingToMs: null, ended: false }),
      { status: changing(s, sent) },
    ),
    [
      { kind: "pausePlayer" },
      { kind: "sendSeek", id: sent.id, sessionId: stream.sessionId, positionMs: target },
    ],
  ];
}

function play(s, playing) {
  if (!watching(s)) return same(s);
  return withProgress(s, [
    withTimeline(s, { playing }),
    [{ kind: playing ? "playPlayer" : "pausePlayer" }],
  ]);
}

function viewerReplays(s) {
  if (!watching(s)) return same(s);
  const [sought, seeking] = viewerSeeks(s, 0);
  const after = withTimeline(sought, { playing: true });
  if (after.status.sent != null) return [after, seeking];
  return withProgress(s, [after, seeking.concat({ kind: "playPlayer" })]);
}

function viewerToggles(s) {
  if (s.timeline.ended) return viewerReplays(s);
  return play(s, !s.timeline.playing);
}

function viewerChangesRemote(s, controls) {
  if (s.status.life === "closed" || s.status.sent != null) return same(s);
  const stream = s.stream;
  if (!stream || sameUpdate(controls, s.remote)) {
    return [Object.assign({}, s, { remote: controls }), []];
  }
  const sent = {
    kind: "controls",
    id: s.status.lastChangeId + 1,
    controls,
    previous: s.remote,
    positionMs: s.timeline.positionMs,
  };
  return [
    Object.assign({}, s, { remote: controls, status: changing(s, sent) }),
    [
      { kind: "pausePlayer" },
      {
        kind: "sendUpdate",
        id: sent.id,
        sessionId: stream.sessionId,
        controls,
        positionMs: s.timeline.positionMs,
      },
    ],
  ];
}

function viewerChangesLocal(s, local) {
  if (s.status.life === "closed") return same(s);
  return [
    Object.assign({}, s, { local }),
    [{ kind: "applyLocal", local }, { kind: "saveLocal", local }],
  ];
}

function viewerRetries(s) {
  if (s.status.life !== "open" || s.status.sent != null || s.status.error == null) {
    return same(s);
  }
  const stream = s.stream;
  const sent = { kind: "start", id: s.status.lastChangeId + 1, positionMs: s.timeline.positionMs };
  return [
    Object.assign({}, withTimeline(s, { seekingToMs: null }), {
      stream: null,
      status: Object.assign(changing(s, sent), { error: null }),
    }),
    (stream ? [endOf(s, stream)] : []).concat({
      kind: "sendStart",
      id: sent.id,
      controls: s.remote,
      positionMs: sent.positionMs,
    }),
  ];
}

function awaits(s, id) {
  return s.status.life === "open" && s.status.sent != null && s.status.sent.id === id;
}

function streamOf(session) {
  return {
    sessionId: session.session_id,
    manifestUrl: session.manifest_url,
    originMs: session.origin_ms || 0,
    sequential: session.sequential === true,
    mode: session.mode,
    container: session.container || null,
    subtitleDelivery: (session.selected && session.selected.subtitle_delivery) || null,
    heartbeatIntervalS: session.heartbeat_interval_s || 10,
  };
}

function replaced(stream, negotiation) {
  return Object.assign({}, stream, {
    manifestUrl: negotiation.manifest_url,
    originMs: negotiation.origin_ms || 0,
    sequential: negotiation.sequential === true,
    mode: negotiation.mode,
    container: negotiation.container || null,
  });
}

function load(s, stream, id, atMs) {
  return [
    Object.assign(
      {},
      withTimeline(s, { positionMs: atMs, seekingToMs: atMs, reachMs: atMs, ended: false }),
      {
        stream,
        status: Object.assign({}, s.status, {
          sent: { kind: "load", id, positionMs: atMs },
          error: null,
          heldError: null,
        }),
      },
    ),
    [{ kind: "loadStream", id, stream, offsetMs: atMs - stream.originMs }],
  ];
}

function resume(before, s) {
  return withProgress(before, [s, s.timeline.playing ? [{ kind: "playPlayer" }] : []]);
}

function serverStarted(s, id, session, positionMs) {
  if (!awaits(s, id) || s.status.sent.kind !== "start") {
    return [
      s,
      [{ kind: "endSession", sessionId: session.session_id, positionMs: null, playing: false }],
    ];
  }
  return load(
    Object.assign({}, s, { remote: withSelection(s.remote, session.selected) }),
    streamOf(session),
    id,
    positionMs,
  );
}

function serverRefusedStart(s, id, message) {
  if (!awaits(s, id) || s.status.sent.kind !== "start") return same(s);
  return same(withStatus(s, { sent: null, slow: false, error: message }));
}

function serverUpdated(s, id, negotiation) {
  const sent = s.status.sent;
  if (!awaits(s, id) || sent.kind !== "controls" || !s.stream) return same(s);
  const delivery = (negotiation.selected && negotiation.selected.subtitle_delivery) || null;
  return load(
    s,
    Object.assign(replaced(s.stream, negotiation), { subtitleDelivery: delivery }),
    id,
    sent.positionMs,
  );
}

function serverSeeked(s, id, negotiation) {
  const sent = s.status.sent;
  if (!awaits(s, id) || sent.kind !== "seek" || !s.stream) return same(s);
  return load(s, replaced(s.stream, negotiation), id, sent.positionMs);
}

function requesting(s) {
  const sent = s.status.sent;
  return s.status.life === "open" && sent != null && (sent.kind === "controls" || sent.kind === "seek");
}

function putBack(s, sent) {
  return sent.kind === "controls"
    ? Object.assign({}, s, { remote: sent.previous })
    : withTimeline(s, { positionMs: sent.previousMs, playing: sent.previousPlaying });
}

function serverFailed(s, id) {
  if (!awaits(s, id) || !requesting(s)) return same(s);
  const sent = s.status.sent;
  const back = putBack(s, sent);
  if (s.status.heldError != null) return streamFails(back, s.stream, s.status.heldError);
  return resume(
    s,
    withStatus(back, {
      sent: null,
      slow: false,
      notice: sent.kind === "controls" ? "changeFailed" : "seekFailed",
    }),
  );
}

function loadingId(s, id) {
  return awaits(s, id) && s.status.sent.kind === "load";
}

function playerReads(s, reading) {
  if (!watching(s)) return same(s);
  const stream = s.stream;
  const atMs = stream.originMs + reading.positionMs;
  const seekingToMs = s.timeline.seekingToMs;
  const settled = seekingToMs == null || Math.abs(atMs - seekingToMs) <= SEEK_SETTLE_MS;
  const fillsDuration = s.durationMs <= 0 && reading.durationMs > 0;
  return withProgress(s, [
    Object.assign(
      {},
      withTimeline(s, {
        positionMs: settled ? atMs : s.timeline.positionMs,
        seekingToMs: settled ? null : seekingToMs,
        reachMs: settled ? atMs + reading.bufferedAheadMs : s.timeline.reachMs,
        bufferedAheadMs: reading.bufferedAheadMs,
        playing: reading.playing,
        ended: settled ? reading.ended : s.timeline.ended,
      }),
      { durationMs: fillsDuration ? stream.originMs + reading.durationMs : s.durationMs },
    ),
    [],
  ]);
}

function playerLoaded(s, id) {
  if (!loadingId(s, id)) return same(s);
  return withProgress(s, [
    withStatus(s, { sent: null, slow: false }),
    s.timeline.playing ? [{ kind: "playPlayer" }] : [],
  ]);
}

function playerFailedToLoad(s, id, message) {
  if (!loadingId(s, id) || !s.stream) return same(s);
  return streamFails(s, s.stream, message);
}

function playerErrs(s, message) {
  if (watching(s)) return streamFails(s, s.stream, message);
  if (requesting(s)) return same(withStatus(s, { heldError: message }));
  return same(s);
}

function lifecycleOpened(s, durationMs, resumeMs) {
  if (s.status.life === "closed" || s.status.gathered) return same(s);
  const id = s.status.lastChangeId + 1;
  return [
    Object.assign({}, withTimeline(s, { positionMs: resumeMs }), {
      durationMs,
      status: Object.assign({}, s.status, {
        gathered: true,
        lastChangeId: id,
        sent: { kind: "start", id, positionMs: resumeMs },
      }),
    }),
    [
      { kind: "applyLocal", local: s.local },
      { kind: "sendStart", id, controls: s.remote, positionMs: resumeMs },
    ],
  ];
}

function lifecycleLeaves(s) {
  if (s.status.life === "closed") return same(s);
  const stream = s.stream;
  return [
    Object.assign({}, s, {
      stream: null,
      status: Object.assign({}, s.status, {
        life: "closed",
        sent: null,
        slow: false,
        heldError: null,
      }),
    }),
    stream ? [endOf(s, stream)] : [],
  ];
}

function heartbeatDue(s) {
  const stream = s.stream;
  if (s.status.life !== "open" || !stream) return same(s);
  return [
    s,
    [{
      kind: "sendProgress",
      sessionId: stream.sessionId,
      positionMs: s.timeline.positionMs,
      playing: reportsPlaying(s),
    }],
  ];
}

function changeSlow(s, id) {
  return awaits(s, id) ? same(withStatus(s, { slow: true })) : same(s);
}

function changeTimedOut(s, sent, message) {
  const stream = s.stream;
  if (s.status.life !== "open" || s.status.sent !== sent) return same(s);
  if (sent.kind === "start") return serverRefusedStart(s, sent.id, message);
  if (sent.kind === "load") return playerFailedToLoad(s, sent.id, message);
  if (!stream) return same(s);
  return streamFails(putBack(s, sent), stream, message);
}

function seekSettled(s, targetMs) {
  return s.timeline.seekingToMs === targetMs ? same(withTimeline(s, { seekingToMs: null })) : same(s);
}

export function step(s, e) {
  switch (e.kind) {
    case "opened":
      return lifecycleOpened(s, e.durationMs, e.resumeMs);
    case "seek":
      return viewerSeeks(s, e.positionMs);
    case "toggle":
      return viewerToggles(s);
    case "changeRemote":
      return viewerChangesRemote(s, e.controls);
    case "changeLocal":
      return viewerChangesLocal(s, e.local);
    case "retry":
      return viewerRetries(s);
    case "leave":
      return lifecycleLeaves(s);
    case "started":
      return serverStarted(s, e.id, e.session, e.positionMs);
    case "refusedStart":
      return serverRefusedStart(s, e.id, e.message);
    case "updated":
      return serverUpdated(s, e.id, e.negotiation);
    case "seeked":
      return serverSeeked(s, e.id, e.negotiation);
    case "failed":
      return serverFailed(s, e.id);
    case "loaded":
      return playerLoaded(s, e.id);
    case "failedToLoad":
      return playerFailedToLoad(s, e.id, e.message);
    case "reads":
      return playerReads(s, e.reading);
    case "errs":
      return playerErrs(s, e.message);
    case "playerSeeks":
      return playerSeeks(s, e.offsetMs);
    case "heartbeatDue":
      return heartbeatDue(s);
    case "changeSlow":
      return changeSlow(s, e.id);
    case "changeTimedOut":
      return changeTimedOut(s, e.sent, e.message);
    case "seekSettled":
      return seekSettled(s, e.targetMs);
    default:
      return same(s);
  }
}
