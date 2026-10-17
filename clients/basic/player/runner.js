import {
  LOAD_TIMEOUT_MARGIN_S,
  opens,
  REQUEST_TIMEOUT_S,
  SEEK_SETTLE_WAIT_MS,
  SLOW_S,
  startBody,
  step,
  updateBody,
} from "./session.js";

function quietly(send) {
  return new Promise((resolve) => resolve(send())).catch(() => {});
}

function wire(playing) {
  return playing ? "playing" : "paused";
}

function failure(prefix, err) {
  const detail = err && err.message;
  return detail ? prefix + " (" + detail + ")." : prefix + ".";
}

export class PlaybackRunner {
  #api;
  #player;
  #platform;
  #saveLocal;
  #profileVersion = 1;
  #state;
  #queue = [];
  #draining = false;
  #listeners = [];
  #ending = Promise.resolve();
  #applied = null;
  #beat = null;
  #heartbeat = null;
  #slow = null;
  #timeout = null;
  #settling = null;
  #unlisten = null;

  constructor({ api, player, platform, saveLocal, versionId, remote, local, playing = true }) {
    this.#api = api;
    this.#player = player;
    this.#platform = platform;
    this.#saveLocal = saveLocal;
    this.#state = opens({ versionId, remote, local, playing });
    this.#unlisten = player.listen(
      (reading) => this.#post({ kind: "reads", reading }),
      (message) => this.#post({ kind: "errs", message }),
      (offsetMs) => this.#post({ kind: "playerSeeks", offsetMs }),
    );
  }

  get state() {
    return this.#state;
  }

  subscribe(listener) {
    this.#listeners.push(listener);
    return () => {
      this.#listeners = this.#listeners.filter((l) => l !== listener);
    };
  }

  opened({ durationMs, resumeMs, profileVersion }) {
    this.#profileVersion = profileVersion || 1;
    this.#post({ kind: "opened", durationMs, resumeMs });
  }

  seek(positionMs) {
    this.#post({ kind: "seek", positionMs });
  }

  toggle() {
    this.#post({ kind: "toggle" });
  }

  changeRemote(controls) {
    this.#post({ kind: "changeRemote", controls });
  }

  changeLocal(local) {
    this.#post({ kind: "changeLocal", local });
  }

  retry() {
    this.#post({ kind: "retry" });
  }

  leave() {
    this.#post({ kind: "leave" });
  }

  #post(event) {
    this.#queue.push(event);
    if (this.#draining) return;
    this.#draining = true;
    try {
      while (this.#queue.length) this.#handle(this.#queue.shift());
    } finally {
      this.#draining = false;
    }
  }

  #handle(e) {
    const event = e.kind === "readNow" ? { kind: "reads", reading: this.#player.reading() } : e;
    const before = this.#state;
    const [after, commands] = step(before, event);
    this.#state = after;
    this.#arm(before, after, commands);
    for (const command of commands) this.#run(command);
    const open = after.status.life === "open";
    const plays = commands.some((c) => c.kind === "playPlayer");
    if (open && !plays && before.status.sent != null && after.status.sent == null) {
      this.#post({ kind: "readNow" });
    }
    if (!open && before.status.life === "open") this.#close();
    if (after !== before) {
      for (const listener of this.#listeners) listener(after, before);
    }
  }

  #arm(before, after, commands) {
    const open = after.status.life === "open";
    const stream = open ? after.stream : null;
    const beat = stream ? stream.sessionId + "@" + stream.heartbeatIntervalS : null;
    if (beat !== this.#beat) {
      this.#beat = beat;
      clearInterval(this.#heartbeat);
      this.#heartbeat = stream
        ? setInterval(
          () => this.#post({ kind: "heartbeatDue" }),
          Math.max(1, stream.heartbeatIntervalS) * 1000,
        )
        : null;
    }
    const sent = after.status.sent;
    const was = before.status.sent;
    if ((sent && sent.id) !== (was && was.id)) {
      clearTimeout(this.#slow);
      this.#slow = sent
        ? setTimeout(() => this.#post({ kind: "changeSlow", id: sent.id }), SLOW_S * 1000)
        : null;
    }
    if (sent !== was) {
      clearTimeout(this.#timeout);
      this.#timeout = sent ? this.#timeoutFor(sent, after.local) : null;
    }
    const settles = commands.some((c) => c.kind === "seekPlayer") || was != null;
    const target = after.timeline.seekingToMs;
    if (!open || sent) {
      clearTimeout(this.#settling);
      this.#settling = null;
    } else if (settles) {
      clearTimeout(this.#settling);
      this.#settling = target == null ? null : setTimeout(
        () => this.#post({ kind: "seekSettled", targetMs: target }),
        SEEK_SETTLE_WAIT_MS,
      );
    }
  }

  #timeoutFor(sent, local) {
    const seconds = sent.kind === "load"
      ? local.networkTimeoutS + LOAD_TIMEOUT_MARGIN_S
      : REQUEST_TIMEOUT_S;
    const message = sent.kind === "start"
      ? "Could not start playback (no answer)."
      : "Playback failed (no answer).";
    return setTimeout(
      () => this.#post({ kind: "changeTimedOut", sent, message }),
      seconds * 1000,
    );
  }

  #run(command) {
    switch (command.kind) {
      case "sendStart":
        this.#start(command);
        break;
      case "sendUpdate":
        this.#answer(command.id, "updated", () =>
          this.#api.update(
            command.sessionId,
            Object.assign(updateBody(command.controls), { position_ms: command.positionMs }),
          ));
        break;
      case "sendSeek":
        this.#answer(command.id, "seeked", () =>
          this.#api.seek(command.sessionId, command.positionMs));
        break;
      case "sendProgress":
        quietly(() =>
          this.#api.progress(command.sessionId, command.positionMs, wire(command.playing))
        );
        break;
      case "endSession":
        this.#ending = this.#afterEnds().then(() => this.#end(command));
        break;
      case "loadStream":
        this.#load(command);
        break;
      case "seekPlayer":
        this.#player.seekTo(command.offsetMs);
        break;
      case "playPlayer":
        this.#player.play();
        break;
      case "pausePlayer":
        this.#player.pause();
        break;
      case "unloadPlayer":
        this.#player.detach();
        break;
      case "applyLocal":
        this.#apply(command.local);
        break;
      case "saveLocal":
        this.#saveLocal(command.local);
        break;
    }
  }

  #start(command) {
    const versionId = this.#state.versionId;
    this.#afterEnds()
      .then(() =>
        this.#api.start(
          Object.assign(
            {
              version_id: versionId,
              start_position_ms: command.positionMs,
              capabilities: { platform: this.#platform, profile_version: this.#profileVersion },
            },
            startBody(command.controls),
          ),
        )
      )
      .then(
        (session) =>
          this.#post({
            kind: "started",
            id: command.id,
            session,
            positionMs: command.positionMs,
          }),
        (err) =>
          this.#post({
            kind: "refusedStart",
            id: command.id,
            message: failure("Could not start playback", err),
          }),
      );
  }

  #answer(id, kind, request) {
    new Promise((resolve) => resolve(request())).then(
      (negotiation) => this.#post({ kind, id, negotiation }),
      () => this.#post({ kind: "failed", id }),
    );
  }

  #load(command) {
    const stream = command.stream;
    new Promise((resolve) =>
      resolve(this.#player.attach(stream.manifestUrl, stream.mode, command.offsetMs))
    ).then(
      () => this.#post({ kind: "loaded", id: command.id }),
      (err) =>
        this.#post({
          kind: "failedToLoad",
          id: command.id,
          message: failure("Playback failed", err),
        }),
    );
  }

  #afterEnds() {
    let timer = null;
    const bound = new Promise((resolve) => {
      timer = setTimeout(resolve, this.#state.local.networkTimeoutS * 1000);
    });
    return Promise.race([this.#ending, bound]).then(() => clearTimeout(timer));
  }

  #end(command) {
    const progress = command.positionMs != null
      ? quietly(() =>
        this.#api.progress(command.sessionId, command.positionMs, wire(command.playing))
      )
      : null;
    const end = quietly(() => this.#api.end(command.sessionId));
    return Promise.all([progress, end]).then(() => {});
  }

  #apply(local) {
    if (!this.#applied || local.networkTimeoutS !== this.#applied.networkTimeoutS) {
      this.#player.setNetworkTimeout(local.networkTimeoutS);
    }
    this.#applied = local;
  }

  #close() {
    clearInterval(this.#heartbeat);
    clearTimeout(this.#slow);
    clearTimeout(this.#timeout);
    clearTimeout(this.#settling);
    this.#heartbeat = null;
    this.#slow = null;
    this.#timeout = null;
    this.#settling = null;
    this.#beat = null;
    if (this.#unlisten) this.#unlisten();
    this.#unlisten = null;
  }
}
