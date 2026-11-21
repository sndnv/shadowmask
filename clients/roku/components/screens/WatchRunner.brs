sub RunnerInit()
    m.playback = invalid
    m.runner = {
        queue: [],
        draining: false,
        beatKey: "",
        slowId: -1,
        slowTimer: invalid,
        timeoutKey: "",
        timeoutTimer: invalid,
        settleTimer: invalid,
        endWaitTimer: invalid,
        pendingStart: invalid,
        pendingEnds: 0,
        giveUpTimer: invalid,
        loadedId: -1,
        segments: SegmentWatchStart(),
        released: false
    }
    m.reportHistory = []

    m.video.ObserveField("state", "onPlaybackVideoState")
    m.video.ObserveField("position", "onPlaybackPosition")
    m.video.ObserveField("streamingSegment", "onPlaybackPosition")
    m.video.ObserveField("bufferingStatus", "onPlaybackBuffering")
    m.video.ObserveField("downloadedSegment", "onPlaybackSegment")
    m.beat.ObserveField("fire", "onPlaybackBeat")
    m.global.ObserveFieldScoped("endsDone", "onEndsDone")
end sub

sub NoteHistory(event as string, facts = invalid as dynamic)
    HistoryAdd(m.reportHistory, event, facts, HistoryNow())
end sub

function AttachedId() as integer
    if m.playback = invalid then return -1
    return m.playback.player.attachId
end function

sub RunnerOpen(versionId as string, controls as dynamic)
    if m.runner.released then return
    NoteHistory("opened")
    if m.playback = invalid
        m.playback = PlaybackOpens(versionId, controls)
        return
    end if

    PostPlayback({ kind: "leave" })
    PostPlayback({ kind: "open", versionId: versionId, remote: ControlsCopy(controls) })
end sub

sub RunnerRelease()
    m.runner.released = true
    PostPlayback({ kind: "leave" })
    m.global.UnobserveFieldScoped("endsDone")
end sub

sub PostPlayback(event as object)
    if m.playback = invalid then return

    m.runner.queue.Push(event)
    if m.runner.draining then return

    m.runner.draining = true
    while m.runner.queue.Count() > 0
        HandlePlayback(m.runner.queue.Shift())
    end while
    m.runner.draining = false
end sub

sub HandlePlayback(event as object)
    posted = event
    if posted.kind = "readNow" then posted = { kind: "reads", id: AttachedId(), reading: PlayerReading() }

    before = m.playback
    taken = PlaybackStep(before, posted)
    after = taken.state
    m.playback = after

    ArmPlayback(before, after, taken.commands)
    for each command in taken.commands
        RunPlaybackCommand(command)
    end for

    if PlaybackReadsNow(before, after, taken.commands) then m.runner.queue.Push({ kind: "readNow" })
    if before.status.life = "open" and after.status.life = "closed" then ClosePlayback()
    PlaybackChanged(before, after)
end sub

sub ArmPlayback(before as object, after as object, commands as object)
    plan = PlaybackTimers(before, after, commands)

    if plan.heartbeatKey <> m.runner.beatKey
        m.runner.beatKey = plan.heartbeatKey
        m.beat.control = "stop"
        if not IsBlank(plan.heartbeatKey)
            m.beat.duration = plan.heartbeatS
            m.beat.control = "start"
        end if
    end if

    if plan.slowId <> m.runner.slowId
        m.runner.slowId = plan.slowId
        StopTimer(m.runner.slowTimer)
        m.runner.slowTimer = invalid
        if plan.slowId >= 0 then m.runner.slowTimer = ArmTimer(StallSeconds(), "onPlaybackSlow", { id: plan.slowId })
    end if

    if plan.timeoutKey <> m.runner.timeoutKey
        m.runner.timeoutKey = plan.timeoutKey
        StopTimer(m.runner.timeoutTimer)
        m.runner.timeoutTimer = invalid
        if not IsBlank(plan.timeoutKey) then m.runner.timeoutTimer = ArmTimer(plan.timeoutS, "onPlaybackTimeout", { id: plan.timeoutId, kind: plan.timeoutKind })
    end if

    if plan.settle = "keep" then return

    StopTimer(m.runner.settleTimer)
    m.runner.settleTimer = invalid
    if plan.settle = "arm" then m.runner.settleTimer = ArmTimer(SeekSettleWaitMs() / 1000.0, "onPlaybackSettle", { targetMs: plan.settleMs })
end sub

sub ClosePlayback()
    m.runner.beatKey = ""
    m.beat.control = "stop"
    m.runner.slowId = -1
    m.runner.timeoutKey = ""
    StopTimer(m.runner.slowTimer)
    StopTimer(m.runner.timeoutTimer)
    StopTimer(m.runner.settleTimer)
    StopTimer(m.runner.endWaitTimer)
    m.runner.slowTimer = invalid
    m.runner.timeoutTimer = invalid
    m.runner.settleTimer = invalid
    m.runner.endWaitTimer = invalid
    m.runner.pendingStart = invalid
end sub

sub RunPlaybackCommand(command as object)
    kind = command.kind
    if kind = "sendStart"
        SendPlaybackStart(command)
    else if kind = "sendUpdate"
        NoteHistory("sent_update", { session: command.sessionId, controls: ControlsUpdateBody(command.controls), position_ms: command.positionMs })
        session = SessionFor(m.global)
        PlaybackRequest(UpdateSessionRequest(session.serverUrl, session.token, command.sessionId, command.controls, command.positionMs), command.id, "onPlaybackUpdated")
    else if kind = "sendSeek"
        NoteHistory("sent_seek", { session: command.sessionId, position_ms: command.positionMs })
        session = SessionFor(m.global)
        PlaybackRequest(SeekSessionRequest(session.serverUrl, session.token, command.sessionId, command.positionMs), command.id, "onPlaybackSeeked")
    else if kind = "sendProgress"
        session = SessionFor(m.global)
        SendRequest(HeartbeatRequest(session.serverUrl, session.token, command.sessionId, command.positionMs, ProgressState(command.playing)), "onPlaybackQuiet")
    else if kind = "endSession"
        EndPlaybackSession(command)
    else if kind = "loadStream"
        AttachPlayback(command)
    else if kind = "seekPlayer"
        PlaybackSeek(command.offsetMs)
    else if kind = "playPlayer"
        if command.resume = true then m.video.control = "resume" else m.video.control = "play"
    else if kind = "pausePlayer"
        m.video.control = "pause"
    else if kind = "stopPlayer" or kind = "unloadPlayer"
        DetachPlayback()
    end if
end sub

sub PlaybackRequest(request as object, changeId as integer, callback as string)
    request.changeId = changeId
    SendRequest(request, callback)
end sub

sub SendPlaybackStart(command as object)
    ahead = Int(m.global.endsQueued)
    if Int(m.global.endsDone) >= ahead
        StartPlaybackNow(command)
        return
    end if

    m.runner.pendingStart = command
    m.runner.pendingEnds = ahead
    StopTimer(m.runner.endWaitTimer)
    m.runner.endWaitTimer = ArmTimer(EndWaitSeconds(), "onPlaybackEndWait", { id: command.id })
end sub

sub StartPlaybackNow(command as object)
    session = SessionFor(m.global)
    capabilities = CapabilitiesBody(DeviceDecoding(), Int(m.global.profileVersion))
    NoteHistory("sent_start", { position_ms: command.positionMs, controls: ControlsStartBody(command.controls), capabilities: capabilities })
    request = StartSessionRequest(session.serverUrl, session.token, m.playback.versionId, command.positionMs, capabilities, command.controls)
    PlaybackRequest(request, command.id, "onPlaybackStarted")
end sub

sub onEndsDone()
    if m.runner.pendingStart = invalid or Int(m.global.endsDone) < m.runner.pendingEnds then return
    ReleasePlaybackStart(m.runner.pendingStart.id)
end sub

sub onPlaybackEndWait(event as object)
    ReleasePlaybackStart(Int(TimerData(event).id))
end sub

sub ReleasePlaybackStart(id as integer)
    command = m.runner.pendingStart
    if command = invalid or command.id <> id then return

    m.runner.pendingStart = invalid
    StopTimer(m.runner.endWaitTimer)
    m.runner.endWaitTimer = invalid
    if PlaybackAwaitsKind(m.playback, id, "start") then StartPlaybackNow(command)
end sub

sub EndPlaybackSession(command as object)
    if IsBlank(command.sessionId) then return
    NoteHistory("ended", { session: command.sessionId })

    if TextOrBlank(m.global.sessionId) = command.sessionId then m.global.sessionId = ""
    m.top.sessionEnd = { sessionId: command.sessionId, positionMs: command.positionMs, playing: command.playing }
end sub

function PlaybackChangeId(event as object) as integer
    return Int(ValueAt(event.GetRoSGNode().request, "changeId", -1))
end function

function PlaybackAnswer(event as object) as object
    data = event.GetData()
    parsed = ParseResponse(data.status, data.body)
    if m.playback <> invalid and m.playback.status.life = "open" then HandledUnauthorised(m.top, parsed.status)
    return parsed
end function

sub onPlaybackStarted(event as object)
    id = PlaybackChangeId(event)
    parsed = PlaybackAnswer(event)
    if parsed.ok
        NoteHistory("answered", AnsweredFacts("start", parsed.json))
        PostPlayback({ kind: "started", id: id, json: parsed.json })
        return
    end if

    NoteHistory("failed", FailedFacts("start", parsed))
    PostPlayback({ kind: "refusedStart", id: id, message: PhraseWith("error.couldNotPlay", { detail: PlaybackDetail(parsed) }) })
end sub

sub onPlaybackUpdated(event as object)
    PlaybackAnswered(event, "updated")
end sub

sub onPlaybackSeeked(event as object)
    PlaybackAnswered(event, "seeked")
end sub

sub PlaybackAnswered(event as object, kind as string)
    id = PlaybackChangeId(event)
    parsed = PlaybackAnswer(event)
    request = "seek"
    if kind = "updated" then request = "update"
    if parsed.ok
        NoteHistory("answered", AnsweredFacts(request, parsed.json))
        PostPlayback({ kind: kind, id: id, json: parsed.json })
        return
    end if

    NoteHistory("failed", FailedFacts(request, parsed))
    PostPlayback({ kind: "failed", id: id })
end sub

sub onPlaybackQuiet(event as object)
    PlaybackAnswer(event)
end sub

sub onPlaybackBeat()
    PostPlayback({ kind: "heartbeatDue" })
end sub

sub onPlaybackSlow(event as object)
    PostPlayback({ kind: "changeSlow", id: Int(TimerData(event).id) })
end sub

sub onPlaybackTimeout(event as object)
    data = TimerData(event)
    key = "error.playbackFailed"
    if data.kind = "start" then key = "error.couldNotPlay"

    PostPlayback({ kind: "changeTimedOut", id: Int(data.id), changeKind: data.kind, message: PhraseWith(key, { detail: Phrase("error.noAnswer") }) })
end sub

sub onPlaybackSettle(event as object)
    PostPlayback({ kind: "seekSettled", targetMs: Int(TimerData(event).targetMs) })
end sub

sub AttachPlayback(command as object)
    DetachPlayback()

    session = SessionFor(m.global)
    bounds = AbrBounds(0)
    content = CreateObject("roSGNode", "ContentNode")
    content.url = StreamUrl(session.serverUrl, command.stream.manifestUrl)
    content.streamFormat = StreamFormatFor(command.stream.mode, ValueAt(m.version, "container", ""))
    content.title = TextOrBlank(ValueAt(m.top.target, "title", ""))
    content.HttpCertificatesFile = CertificatesFile()
    content.SwitchingStrategy = bounds.strategy
    if bounds.maxKbps > 0 then content.MaxBandwidth = bounds.maxKbps
    content.PlayStart = AttachSeconds(command.offsetMs, 0)

    NoteHistory("load", { session: command.stream.sessionId, offset_ms: command.offsetMs })
    m.global.negotiated = command.stream
    m.global.sessionId = command.stream.sessionId
    SubtitlesAttach(command)
    m.video.content = content
    m.video.control = "prebuffer"
    m.runner.giveUpTimer = ArmTimer(PrebufferGiveUpSeconds(), "onPlaybackGiveUp", { id: command.id })
end sub

sub DetachPlayback()
    m.runner.loadedId = -1
    m.runner.segments = SegmentWatchStart()
    StopTimer(m.runner.giveUpTimer)
    m.runner.giveUpTimer = invalid
    SubtitlesClear()
    m.video.control = "stop"
end sub

sub PlaybackSeek(offsetMs as integer)
    seconds = 0.0
    if offsetMs > 0 then seconds = offsetMs / 1000.0
    m.video.seek = seconds
    SubtitlesSought(Int(seconds * 1000.0))
end sub

function PlayerReading() as object
    reading = { positionMs: -1, durationMs: 0, bufferedAheadMs: 0, bufferedPercent: 0 }
    if m.video.duration <> invalid and m.video.duration > 0 then reading.durationMs = Int(m.video.duration * 1000.0)
    buffering = m.video.bufferingStatus
    if type(buffering) = "roAssociativeArray" then reading.bufferedPercent = Int(ValueAt(buffering, "percentage", 0))
    if m.video.position = invalid then return reading

    reading.positionMs = Int(m.video.position * 1000.0)
    if reading.positionMs < 0 then reading.positionMs = 0
    fetching = Int(ValueAt(m.video.streamingSegment, "segStartTime", 0) * 1000.0)
    if fetching > reading.positionMs then reading.bufferedAheadMs = fetching - reading.positionMs
    return reading
end function

sub onPlaybackPosition()
    if AttachedId() < 0 then return
    PostPlayback({ kind: "reads", id: AttachedId(), reading: PlayerReading() })
end sub

sub onPlaybackBuffering()
    if AttachedId() < 0 then return
    PostPlayback({ kind: "reads", id: AttachedId(), reading: PlayerReading() })
    if ValueAt(m.video.bufferingStatus, "prebufferDone", false) = true then PlaybackAttached(AttachedId())
end sub

sub onPlaybackGiveUp(event as object)
    PlaybackAttached(Int(TimerData(event).id))
end sub

sub PlaybackAttached(id as integer)
    if id <> AttachedId() or id = m.runner.loadedId then return

    m.runner.loadedId = id
    StopTimer(m.runner.giveUpTimer)
    m.runner.giveUpTimer = invalid
    NoteHistory("loaded")
    PostPlayback({ kind: "loaded", id: id })
end sub

sub onPlaybackVideoState()
    state = TextOrBlank(m.video.state)
    NoteVideoState(state)
    if AttachedId() < 0 then return

    if state = "error"
        PlaybackFails(PhraseWith("error.playbackFailed", { detail: VideoDetail() }))
        return
    end if
    PostPlayback({ kind: "videoState", id: AttachedId(), state: state, reading: PlayerReading() })
end sub

sub onPlaybackSegment(event as object)
    if AttachedId() < 0 then return

    report = event.GetData()
    before = m.runner.segments
    m.runner.segments = SegmentWatched(before, report)
    if SegmentStreakStarts(before, m.runner.segments)
        AppendPlaybackLog(m.playbackLog, PlaybackSegmentFailedLine(NowClock(), report))
        RefreshDiagnostics()
    end if
    if not SegmentsLost(m.runner.segments) then return

    AppendPlaybackLog(m.playbackLog, PlaybackLostLine(NowClock(), m.runner.segments.last))
    m.runner.segments = SegmentWatchStart()
    RefreshDiagnostics()
    PlaybackFails(PhraseWith("error.playbackFailed", { detail: Phrase("error.couldNotLoad") }))
end sub

sub PlaybackFails(message as string)
    PostPlayback({ kind: "videoFailed", id: AttachedId(), message: message })
end sub
