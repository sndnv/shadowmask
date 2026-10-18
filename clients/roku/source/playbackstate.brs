function SeekReachMs() as integer
    return 30000
end function

function SeekSettleMs() as integer
    return 1000
end function

function SeekSettleWaitMs() as integer
    return 2000
end function

function ChangeRequestSeconds() as integer
    return 120
end function

function ChangeLoadSeconds() as integer
    return 15
end function

function EndWaitSeconds() as integer
    return 10
end function

function PlaybackOpens(versionId as string, remote as dynamic, lastChangeId = 0 as integer) as object
    return {
        versionId: versionId,
        durationMs: 0,
        timeline: { positionMs: 0, seekingToMs: -1, reachMs: 0, playing: true, ended: false, placed: false },
        remote: ControlsCopy(remote),
        stream: invalid,
        status: { life: "open", gathered: false, sent: invalid, lastChangeId: lastChangeId, slow: false, error: "", notice: "" }
    }
end function

function IsWatching(s as object) as boolean
    return s.status.life = "open" and s.status.sent = invalid and s.stream <> invalid
end function

function ReportsPlaying(s as object) as boolean
    return s.timeline.playing and s.status.sent = invalid
end function

function SeeksLocally(s as object, positionMs as integer) as boolean
    if not IsWatching(s) or not s.timeline.placed then return false
    return PlaybackReachable(s, PlaybackWithinTitle(s, positionMs))
end function

function SameUpdate(left as dynamic, right as dynamic) as boolean
    return FormatJson(ControlsUpdateBody(left)) = FormatJson(ControlsUpdateBody(right))
end function

function PlaybackCaptions(stream as object, remote as object) as boolean
    if stream.subtitleDelivery = "burned" or remote.subtitleOff = true then return false
    return SubtitleRef(remote.subtitle) <> invalid
end function

function PlaybackCopy(source as object, changes as object) as object
    copy = {}
    copy.Append(source)
    copy.Append(changes)
    return copy
end function

function PlaybackWithTimeline(s as object, changes as object) as object
    return PlaybackCopy(s, { timeline: PlaybackCopy(s.timeline, changes) })
end function

function PlaybackWithStatus(s as object, changes as object) as object
    return PlaybackCopy(s, { status: PlaybackCopy(s.status, changes) })
end function

function PlaybackStepOf(s as object, commands as object) as object
    return { state: s, commands: commands }
end function

function PlaybackSame(s as object) as object
    return PlaybackStepOf(s, [])
end function

function PlaybackWithProgress(before as object, taken as object) as object
    after = taken.state
    if not IsWatching(after) or ReportsPlaying(after) = ReportsPlaying(before) then return taken

    commands = []
    commands.Append(taken.commands)
    commands.Push({ kind: "sendProgress", sessionId: after.stream.sessionId, positionMs: after.timeline.positionMs, playing: ReportsPlaying(after) })
    return PlaybackStepOf(after, commands)
end function

function PlaybackEndOf(s as object) as object
    return { kind: "endSession", sessionId: s.stream.sessionId, positionMs: s.timeline.positionMs, playing: ReportsPlaying(s) }
end function

function PlaybackStreamFails(s as object, message as string) as object
    failed = PlaybackWithStatus(PlaybackCopy(s, { stream: invalid }), { sent: invalid, slow: false, error: message })
    return PlaybackStepOf(failed, [
        { kind: "endSession", sessionId: s.stream.sessionId, positionMs: s.timeline.positionMs, playing: false },
        { kind: "unloadPlayer" }
    ])
end function

function PlaybackChanging(s as object, change as object) as object
    return PlaybackCopy(s.status, { lastChangeId: change.id, notice: "", sent: change })
end function

function PlaybackReachable(s as object, targetMs as integer) as boolean
    if not s.stream.sequential then return true
    if targetMs < s.stream.originMs then return false
    return targetMs <= s.timeline.reachMs + SeekReachMs()
end function

function PlaybackWithinTitle(s as object, positionMs as integer) as integer
    at = positionMs
    if at < 0 then at = 0
    if s.durationMs > 0 and at > s.durationMs then at = s.durationMs
    return at
end function

function PlaybackAwaits(s as object, id as integer) as boolean
    return s.status.life = "open" and s.status.sent <> invalid and s.status.sent.id = id
end function

function PlaybackAwaitsKind(s as object, id as integer, kind as string) as boolean
    return PlaybackAwaits(s, id) and s.status.sent.kind = kind
end function

function PlaybackRequesting(s as object) as boolean
    if s.status.life <> "open" or s.status.sent = invalid then return false
    return s.status.sent.kind = "controls" or s.status.sent.kind = "seek"
end function

function PlaybackPutBack(s as object) as object
    change = s.status.sent
    if change.kind = "controls" then return PlaybackCopy(s, { remote: change.previous })
    return PlaybackWithTimeline(s, { positionMs: change.previousMs, playing: change.previousPlaying })
end function

function SubtitleDeliveryOf(json as dynamic) as string
    return LCase(TextOrBlank(ValueAt(json, "selected.subtitle_delivery", "")))
end function

function PlaybackStreamReplaced(stream as object, json as dynamic) as object
    return PlaybackCopy(stream, {
        manifestUrl: TextOrBlank(ValueAt(json, "manifest_url", "")),
        originMs: Int(ValueAt(json, "origin_ms", 0)),
        sequential: ValueAt(json, "sequential", false) = true,
        mode: TextOrBlank(ValueAt(json, "mode", "")),
        container: TextOrBlank(ValueAt(json, "container", ""))
    })
end function

function PlaybackStreamOf(json as dynamic) as object
    beat = Int(ValueAt(json, "heartbeat_interval_s", 0))
    if beat <= 0 then beat = HeartbeatFallbackSeconds()

    return PlaybackStreamReplaced({
        sessionId: TextOrBlank(ValueAt(json, "session_id", "")),
        heartbeatS: beat,
        subtitleDelivery: SubtitleDeliveryOf(json)
    }, json)
end function

function PlaybackLoad(s as object, stream as object, id as integer, atMs as integer) as object
    loading = PlaybackCopy(PlaybackWithTimeline(s, { positionMs: atMs, seekingToMs: atMs, reachMs: atMs, ended: false, placed: false }), {
        stream: stream,
        status: PlaybackCopy(s.status, { sent: { kind: "load", id: id, positionMs: atMs }, error: "" })
    })
    return PlaybackStepOf(loading, [{ kind: "loadStream", id: id, stream: stream, offsetMs: atMs - stream.originMs, captions: PlaybackCaptions(stream, s.remote) }])
end function

function ServerSeek(s as object, targetMs as integer) as object
    change = { kind: "seek", id: s.status.lastChangeId + 1, positionMs: targetMs, previousMs: s.timeline.positionMs, previousPlaying: s.timeline.playing }
    sought = PlaybackCopy(PlaybackWithTimeline(s, { positionMs: targetMs, seekingToMs: -1, ended: false }), { status: PlaybackChanging(s, change) })
    return PlaybackStepOf(sought, [
        { kind: "stopPlayer" },
        { kind: "sendSeek", id: change.id, sessionId: s.stream.sessionId, positionMs: targetMs }
    ])
end function

function PlaybackLoadAgain(s as object, id as integer, atMs as integer) as object
    loading = PlaybackLoad(s, s.stream, id, atMs)
    return PlaybackStepOf(PlaybackWithTimeline(loading.state, { reachMs: s.timeline.reachMs }), loading.commands)
end function

function PlaybackReloads(s as object, targetMs as integer) as object
    change = { kind: "load", id: s.status.lastChangeId + 1, positionMs: targetMs }
    return PlaybackLoadAgain(PlaybackCopy(s, { status: PlaybackChanging(s, change) }), change.id, targetMs)
end function

function PlaybackSeeksTo(s as object, targetMs as integer) as object
    if not PlaybackReachable(s, targetMs) then return ServerSeek(s, targetMs)
    if not s.timeline.placed then return PlaybackReloads(s, targetMs)

    sought = PlaybackWithTimeline(s, { positionMs: targetMs, seekingToMs: targetMs, ended: false })
    return PlaybackStepOf(sought, [{ kind: "seekPlayer", offsetMs: targetMs - s.stream.originMs }])
end function

function ViewerSeeks(s as object, positionMs as integer) as object
    if not IsWatching(s) then return PlaybackSame(s)

    target = PlaybackWithinTitle(s, positionMs)
    if s.timeline.ended and target >= s.timeline.positionMs then return PlaybackSame(s)
    return PlaybackSeeksTo(s, target)
end function

function ViewerSeeksBy(s as object, deltaMs as integer) as object
    if s.durationMs <= 0 then return PlaybackSame(s)
    return ViewerSeeks(s, s.timeline.positionMs + deltaMs)
end function

function ViewerPlays(s as object, playing as boolean) as object
    if not IsWatching(s) then return PlaybackSame(s)

    kind = "pausePlayer"
    if playing then kind = "playPlayer"
    return PlaybackWithProgress(s, PlaybackStepOf(PlaybackWithTimeline(s, { playing: playing }), [{ kind: kind }]))
end function

function ViewerReplays(s as object) as object
    if not IsWatching(s) then return PlaybackSame(s)

    sought = PlaybackSeeksTo(s, 0)
    return PlaybackStepOf(PlaybackWithTimeline(sought.state, { playing: true }), sought.commands)
end function

function ViewerToggles(s as object) as object
    if s.timeline.ended then return ViewerReplays(s)
    return ViewerPlays(s, not s.timeline.playing)
end function

function ViewerChangesRemote(s as object, controls as dynamic) as object
    if s.status.life = "closed" or s.status.sent <> invalid then return PlaybackSame(s)

    wanted = ControlsCopy(controls)
    if s.stream = invalid or SameUpdate(wanted, s.remote) then return PlaybackSame(PlaybackCopy(s, { remote: wanted }))

    change = { kind: "controls", id: s.status.lastChangeId + 1, controls: wanted, previous: s.remote, positionMs: s.timeline.positionMs }
    changed = PlaybackCopy(s, { remote: wanted, status: PlaybackChanging(s, change) })
    return PlaybackStepOf(changed, [
        { kind: "stopPlayer" },
        { kind: "sendUpdate", id: change.id, sessionId: s.stream.sessionId, controls: wanted, positionMs: s.timeline.positionMs }
    ])
end function

function ViewerRetries(s as object) as object
    if s.status.life <> "open" or s.status.sent <> invalid or IsBlank(s.status.error) then return PlaybackSame(s)

    change = { kind: "start", id: s.status.lastChangeId + 1, positionMs: s.timeline.positionMs }
    retried = PlaybackCopy(PlaybackWithTimeline(s, { seekingToMs: -1 }), { status: PlaybackCopy(PlaybackChanging(s, change), { error: "" }) })
    return PlaybackStepOf(retried, [{ kind: "sendStart", id: change.id, controls: s.remote, positionMs: change.positionMs }])
end function

function ServerStarted(s as object, id as integer, json as dynamic) as object
    if not PlaybackAwaitsKind(s, id, "start")
        return PlaybackStepOf(s, [{ kind: "endSession", sessionId: TextOrBlank(ValueAt(json, "session_id", "")), positionMs: -1, playing: false }])
    end if

    picked = PlaybackCopy(s, { remote: ControlsWithSelection(s.remote, ValueAt(json, "selected", invalid)) })
    return PlaybackLoad(picked, PlaybackStreamOf(json), id, s.status.sent.positionMs)
end function

function ServerRefusedStart(s as object, id as integer, message as string) as object
    if not PlaybackAwaitsKind(s, id, "start") then return PlaybackSame(s)
    return PlaybackSame(PlaybackWithStatus(s, { sent: invalid, slow: false, error: message }))
end function

function ServerUpdated(s as object, id as integer, json as dynamic) as object
    if not PlaybackAwaitsKind(s, id, "controls") or s.stream = invalid then return PlaybackSame(s)

    stream = PlaybackCopy(PlaybackStreamReplaced(s.stream, json), { subtitleDelivery: SubtitleDeliveryOf(json) })
    return PlaybackLoad(s, stream, id, s.status.sent.positionMs)
end function

function ServerSeeked(s as object, id as integer, json as dynamic) as object
    if not PlaybackAwaitsKind(s, id, "seek") or s.stream = invalid then return PlaybackSame(s)
    return PlaybackLoad(s, PlaybackStreamReplaced(s.stream, json), id, s.status.sent.positionMs)
end function

function ServerFailed(s as object, id as integer) as object
    if not PlaybackAwaits(s, id) or not PlaybackRequesting(s) then return PlaybackSame(s)

    notice = "seekFailed"
    if s.status.sent.kind = "controls" then notice = "changeFailed"
    back = PlaybackWithStatus(PlaybackPutBack(s), { notice: notice })
    return PlaybackLoadAgain(back, id, back.timeline.positionMs)
end function

function PlayerReads(s as object, reading as object) as object
    if not IsWatching(s) then return PlaybackSame(s)

    origin = s.stream.originMs
    durationMs = s.durationMs
    if durationMs <= 0 and reading.durationMs > 0 then durationMs = origin + reading.durationMs

    changes = { playing: reading.playing }
    if reading.ended
        changes.seekingToMs = -1
        changes.ended = true
        changes.placed = false
        if reading.positionMs >= 0
            endMs = origin + reading.positionMs
            if durationMs > 0 and endMs > durationMs then endMs = durationMs
            changes.positionMs = endMs
            changes.reachMs = endMs + reading.bufferedAheadMs
        end if
    else if reading.positionMs >= 0
        changes.placed = true
        atMs = origin + reading.positionMs
        target = s.timeline.seekingToMs
        if target < 0 or Abs(atMs - target) <= SeekSettleMs()
            changes.positionMs = atMs
            changes.seekingToMs = -1
            changes.reachMs = atMs + reading.bufferedAheadMs
            changes.ended = false
        end if
    end if

    return PlaybackWithProgress(s, PlaybackSame(PlaybackCopy(PlaybackWithTimeline(s, changes), { durationMs: durationMs })))
end function

function PlayerLoaded(s as object, id as integer) as object
    if not PlaybackAwaitsKind(s, id, "load") then return PlaybackSame(s)

    commands = []
    if s.timeline.playing then commands.Push({ kind: "playPlayer" })
    return PlaybackWithProgress(s, PlaybackStepOf(PlaybackWithStatus(s, { sent: invalid, slow: false }), commands))
end function

function PlayerFailedToLoad(s as object, id as integer, message as string) as object
    if not PlaybackAwaitsKind(s, id, "load") or s.stream = invalid then return PlaybackSame(s)
    return PlaybackStreamFails(s, message)
end function

function PlayerErrs(s as object, message as string) as object
    if not IsWatching(s) then return PlaybackSame(s)
    return PlaybackStreamFails(s, message)
end function

function LifecycleOpens(s as object, versionId as string, remote as dynamic) as object
    if s.status.life <> "closed" then return PlaybackSame(s)
    return PlaybackSame(PlaybackOpens(versionId, remote, s.status.lastChangeId))
end function

function LifecycleOpened(s as object, durationMs as integer, resumeMs as integer, remote as dynamic) as object
    if s.status.life = "closed" or s.status.gathered then return PlaybackSame(s)

    id = s.status.lastChangeId + 1
    controls = ControlsCopy(remote)
    opened = PlaybackCopy(PlaybackWithTimeline(s, { positionMs: resumeMs }), {
        durationMs: durationMs,
        remote: controls,
        status: PlaybackCopy(s.status, { gathered: true, lastChangeId: id, sent: { kind: "start", id: id, positionMs: resumeMs } })
    })
    return PlaybackStepOf(opened, [{ kind: "sendStart", id: id, controls: controls, positionMs: resumeMs }])
end function

function LifecycleLeaves(s as object) as object
    if s.status.life = "closed" then return PlaybackSame(s)

    closed = PlaybackCopy(s, { stream: invalid, status: PlaybackCopy(s.status, { life: "closed", sent: invalid, slow: false }) })
    if s.stream = invalid then return PlaybackSame(closed)
    return PlaybackStepOf(closed, [PlaybackEndOf(s), { kind: "unloadPlayer" }])
end function

function HeartbeatDue(s as object) as object
    if s.status.life <> "open" or s.stream = invalid then return PlaybackSame(s)
    return PlaybackStepOf(s, [{ kind: "sendProgress", sessionId: s.stream.sessionId, positionMs: s.timeline.positionMs, playing: ReportsPlaying(s) }])
end function

function ChangeSlow(s as object, id as integer) as object
    if not PlaybackAwaits(s, id) then return PlaybackSame(s)
    return PlaybackSame(PlaybackWithStatus(s, { slow: true }))
end function

function ChangeTimedOut(s as object, id as integer, kind as string, message as string) as object
    if not PlaybackAwaitsKind(s, id, kind) then return PlaybackSame(s)
    if kind = "start" then return ServerRefusedStart(s, id, message)
    if kind = "load" then return PlayerFailedToLoad(s, id, message)
    if s.stream = invalid then return PlaybackSame(s)

    return PlaybackStreamFails(PlaybackPutBack(s), message)
end function

function SeekSettled(s as object, targetMs as integer) as object
    if targetMs < 0 or s.timeline.seekingToMs <> targetMs then return PlaybackSame(s)
    return PlaybackSame(PlaybackWithTimeline(s, { seekingToMs: -1 }))
end function

function PlaybackStep(s as object, event as object) as object
    kind = event.kind
    if kind = "open" then return LifecycleOpens(s, event.versionId, event.remote)
    if kind = "opened" then return LifecycleOpened(s, event.durationMs, event.resumeMs, event.remote)
    if kind = "seek" then return ViewerSeeks(s, event.positionMs)
    if kind = "seekBy" then return ViewerSeeksBy(s, event.deltaMs)
    if kind = "toggle" then return ViewerToggles(s)
    if kind = "changeRemote" then return ViewerChangesRemote(s, event.controls)
    if kind = "retry" then return ViewerRetries(s)
    if kind = "leave" then return LifecycleLeaves(s)
    if kind = "started" then return ServerStarted(s, event.id, event.json)
    if kind = "refusedStart" then return ServerRefusedStart(s, event.id, event.message)
    if kind = "updated" then return ServerUpdated(s, event.id, event.json)
    if kind = "seeked" then return ServerSeeked(s, event.id, event.json)
    if kind = "failed" then return ServerFailed(s, event.id)
    if kind = "loaded" then return PlayerLoaded(s, event.id)
    if kind = "failedToLoad" then return PlayerFailedToLoad(s, event.id, event.message)
    if kind = "reads" then return PlayerReads(s, event.reading)
    if kind = "errs" then return PlayerErrs(s, event.message)
    if kind = "heartbeatDue" then return HeartbeatDue(s)
    if kind = "changeSlow" then return ChangeSlow(s, event.id)
    if kind = "changeTimedOut" then return ChangeTimedOut(s, event.id, event.changeKind, event.message)
    if kind = "seekSettled" then return SeekSettled(s, event.targetMs)
    return PlaybackSame(s)
end function

function PlaybackTimers(before as object, after as object, commands as object) as object
    plan = { heartbeatKey: "", heartbeatS: 0, slowId: -1, timeoutKey: "", timeoutId: -1, timeoutKind: "", timeoutS: 0, settle: "keep", settleMs: -1 }
    if after.status.life <> "open"
        plan.settle = "stop"
        return plan
    end if

    if after.stream <> invalid
        plan.heartbeatKey = after.stream.sessionId + "@" + after.stream.heartbeatS.ToStr()
        plan.heartbeatS = after.stream.heartbeatS
    end if

    change = after.status.sent
    if change <> invalid
        plan.slowId = change.id
        plan.timeoutKey = change.kind + "@" + change.id.ToStr()
        plan.timeoutId = change.id
        plan.timeoutKind = change.kind
        plan.timeoutS = ChangeRequestSeconds()
        if change.kind = "load" then plan.timeoutS = ChangeLoadSeconds()
        plan.settle = "stop"
        return plan
    end if

    settles = before.status.sent <> invalid
    for each command in commands
        if command.kind = "seekPlayer" then settles = true
    end for
    if not settles then return plan

    plan.settle = "stop"
    if after.timeline.seekingToMs < 0 then return plan

    plan.settle = "arm"
    plan.settleMs = after.timeline.seekingToMs
    return plan
end function

function PlaybackReadsNow(before as object, after as object, commands as object) as boolean
    if after.status.life <> "open" or before.status.sent = invalid or after.status.sent <> invalid then return false

    for each command in commands
        if command.kind = "playPlayer" then return false
    end for
    return true
end function
