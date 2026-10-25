sub RunnerInit()
    m.playback = invalid
    m.playbackQueue = []
    m.playbackDraining = false
    m.beatKey = ""
    m.slowId = -1
    m.slowTimer = invalid
    m.timeoutKey = ""
    m.timeoutTimer = invalid
    m.settleTimer = invalid
    m.endWaitTimer = invalid
    m.pendingStart = invalid
    m.pendingEnds = 0
    m.giveUpTimer = invalid
    m.attachId = -1
    m.attachLoaded = false
    m.started = false
    m.painted = false
    m.toldToPlay = false
    m.runnerReleased = false
    m.reportHistory = []

    m.video.ObserveField("state", "onPlaybackVideoState")
    m.video.ObserveField("position", "onPlaybackPosition")
    m.video.ObserveField("streamingSegment", "onPlaybackPosition")
    m.video.ObserveField("bufferingStatus", "onPlaybackBuffering")
    m.beat.ObserveField("fire", "onPlaybackBeat")
    m.global.ObserveFieldScoped("endsDone", "onEndsDone")
end sub

sub NoteHistory(event as string, facts = invalid as dynamic)
    HistoryAdd(m.reportHistory, event, facts, HistoryNow())
end sub

sub RunnerOpen(versionId as string, controls as dynamic)
    if m.runnerReleased then return
    NoteHistory("opened")
    if m.playback = invalid
        m.playback = PlaybackOpens(versionId, controls)
        return
    end if

    PostPlayback({ kind: "leave" })
    PostPlayback({ kind: "open", versionId: versionId, remote: ControlsCopy(controls) })
end sub

sub RunnerRelease()
    m.runnerReleased = true
    PostPlayback({ kind: "leave" })
    m.global.UnobserveFieldScoped("endsDone")
end sub

sub PostPlayback(event as object)
    if m.playback = invalid then return

    m.playbackQueue.Push(event)
    if m.playbackDraining then return

    m.playbackDraining = true
    while m.playbackQueue.Count() > 0
        HandlePlayback(m.playbackQueue.Shift())
    end while
    m.playbackDraining = false
end sub

sub HandlePlayback(event as object)
    posted = event
    if posted.kind = "readNow" then posted = { kind: "reads", reading: PlayerReading() }

    before = m.playback
    taken = PlaybackStep(before, posted)
    after = taken.state
    m.playback = after

    ArmPlayback(before, after, taken.commands)
    for each command in taken.commands
        RunPlaybackCommand(command)
    end for

    if PlaybackReadsNow(before, after, taken.commands) then m.playbackQueue.Push({ kind: "readNow" })
    if before.status.life = "open" and after.status.life = "closed" then ClosePlayback()
    PlaybackChanged(before, after)
end sub

sub ArmPlayback(before as object, after as object, commands as object)
    plan = PlaybackTimers(before, after, commands)

    if plan.heartbeatKey <> m.beatKey
        m.beatKey = plan.heartbeatKey
        m.beat.control = "stop"
        if not IsBlank(plan.heartbeatKey)
            m.beat.duration = plan.heartbeatS
            m.beat.control = "start"
        end if
    end if

    if plan.slowId <> m.slowId
        m.slowId = plan.slowId
        StopTimer(m.slowTimer)
        m.slowTimer = invalid
        if plan.slowId >= 0 then m.slowTimer = ArmTimer(StallSeconds(), "onPlaybackSlow", { id: plan.slowId })
    end if

    if plan.timeoutKey <> m.timeoutKey
        m.timeoutKey = plan.timeoutKey
        StopTimer(m.timeoutTimer)
        m.timeoutTimer = invalid
        if not IsBlank(plan.timeoutKey) then m.timeoutTimer = ArmTimer(plan.timeoutS, "onPlaybackTimeout", { id: plan.timeoutId, kind: plan.timeoutKind })
    end if

    if plan.settle = "keep" then return

    StopTimer(m.settleTimer)
    m.settleTimer = invalid
    if plan.settle = "arm" then m.settleTimer = ArmTimer(SeekSettleWaitMs() / 1000.0, "onPlaybackSettle", { targetMs: plan.settleMs })
end sub

sub ClosePlayback()
    m.beatKey = ""
    m.beat.control = "stop"
    m.slowId = -1
    m.timeoutKey = ""
    StopTimer(m.slowTimer)
    StopTimer(m.timeoutTimer)
    StopTimer(m.settleTimer)
    StopTimer(m.endWaitTimer)
    m.slowTimer = invalid
    m.timeoutTimer = invalid
    m.settleTimer = invalid
    m.endWaitTimer = invalid
    m.pendingStart = invalid
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
        PlaybackPlay()
    else if kind = "pausePlayer"
        m.toldToPlay = false
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

    m.pendingStart = command
    m.pendingEnds = ahead
    StopTimer(m.endWaitTimer)
    m.endWaitTimer = ArmTimer(EndWaitSeconds(), "onPlaybackEndWait", { id: command.id })
end sub

sub StartPlaybackNow(command as object)
    session = SessionFor(m.global)
    capabilities = CapabilitiesBody(DeviceDecoding(), Int(m.global.profileVersion))
    NoteHistory("sent_start", { position_ms: command.positionMs, controls: ControlsStartBody(command.controls), capabilities: capabilities })
    request = StartSessionRequest(session.serverUrl, session.token, m.playback.versionId, command.positionMs, capabilities, command.controls)
    PlaybackRequest(request, command.id, "onPlaybackStarted")
end sub

sub onEndsDone()
    if m.pendingStart = invalid or Int(m.global.endsDone) < m.pendingEnds then return
    ReleasePlaybackStart(m.pendingStart.id)
end sub

sub onPlaybackEndWait(event as object)
    ReleasePlaybackStart(Int(TimerData(event).id))
end sub

sub ReleasePlaybackStart(id as integer)
    command = m.pendingStart
    if command = invalid or command.id <> id then return

    m.pendingStart = invalid
    StopTimer(m.endWaitTimer)
    m.endWaitTimer = invalid
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
    m.attachId = command.id
    m.global.negotiated = command.stream
    m.global.sessionId = command.stream.sessionId
    ApplyPlaybackCaptions(command.captions)
    m.video.content = content
    m.video.control = "prebuffer"
    m.giveUpTimer = ArmTimer(PrebufferGiveUpSeconds(), "onPlaybackGiveUp", { id: command.id })
end sub

sub DetachPlayback()
    m.attachId = -1
    m.attachLoaded = false
    m.started = false
    m.painted = false
    m.toldToPlay = false
    StopTimer(m.giveUpTimer)
    m.giveUpTimer = invalid
    m.video.control = "stop"
end sub

sub ApplyPlaybackCaptions(wanted as boolean)
    ApplySubtitleSize()
    m.video.suppressCaptions = not wanted
    if not wanted
        RestoreCaptionMode()
        return
    end if
    if TextOrBlank(m.global.captionMode) = "On" then return

    m.captionsForced = true
    m.video.globalCaptionMode = "On"
end sub

sub PlaybackPlay()
    m.toldToPlay = true
    if m.started and TextOrBlank(m.video.state) <> "finished"
        m.video.control = "resume"
        return
    end if

    m.started = true
    m.video.control = "play"
end sub

sub PlaybackSeek(offsetMs as integer)
    seconds = 0.0
    if offsetMs > 0 then seconds = offsetMs / 1000.0
    m.video.seek = seconds
end sub

function PlayerReading() as object
    state = TextOrBlank(m.video.state)
    stopped = state = "finished" or state = "stopped" or state = "error"
    reading = { positionMs: -1, durationMs: 0, bufferedAheadMs: 0, playing: m.toldToPlay and not stopped, ended: state = "finished" }

    if m.video.duration <> invalid and m.video.duration > 0 then reading.durationMs = Int(m.video.duration * 1000.0)
    if not m.painted or m.video.position = invalid then return reading

    reading.positionMs = Int(m.video.position * 1000.0)
    if reading.positionMs < 0 then reading.positionMs = 0
    fetching = Int(ValueAt(m.video.streamingSegment, "segStartTime", 0) * 1000.0)
    if fetching > reading.positionMs then reading.bufferedAheadMs = fetching - reading.positionMs
    return reading
end function

sub onPlaybackPosition()
    if m.attachId < 0 then return
    PostPlayback({ kind: "reads", reading: PlayerReading() })
end sub

sub onPlaybackBuffering()
    if m.attachId < 0 or m.attachLoaded then return
    if ValueAt(m.video.bufferingStatus, "prebufferDone", false) = true then PlaybackAttached(m.attachId)
end sub

sub onPlaybackGiveUp(event as object)
    PlaybackAttached(Int(TimerData(event).id))
end sub

sub PlaybackAttached(id as integer)
    if id <> m.attachId or m.attachLoaded then return

    m.attachLoaded = true
    StopTimer(m.giveUpTimer)
    m.giveUpTimer = invalid
    NoteHistory("loaded")
    PostPlayback({ kind: "loaded", id: id })
end sub

sub onPlaybackVideoState()
    state = TextOrBlank(m.video.state)
    NoteVideoState(state)
    if m.attachId < 0 then return

    if state = "playing" then m.painted = true
    if state <> "error"
        PostPlayback({ kind: "reads", reading: PlayerReading() })
        return
    end if

    message = PhraseWith("error.playbackFailed", { detail: VideoDetail() })
    if m.attachLoaded
        PostPlayback({ kind: "errs", message: message })
        return
    end if

    id = m.attachId
    DetachPlayback()
    PostPlayback({ kind: "failedToLoad", id: id, message: message })
end sub
