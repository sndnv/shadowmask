sub SubtitlesInit()
    m.subtitleView = m.top.FindNode("subtitles")
    m.subtitleTick = m.top.FindNode("subtitleTick")
    m.subtitleTask = invalid
    m.subtitleCues = []
    m.subtitleShown = []
    m.subtitleWatch = CreateObject("roTimespan")
    m.subtitleClock = SubtitleClockHeld(0, SubtitleNowMs(), false)

    m.video.suppressCaptions = true
    m.subtitleTick.duration = SubtitleTickSeconds()
    m.subtitleTick.ObserveField("fire", "onSubtitleTick")
    m.video.ObserveField("position", "onSubtitlePosition")
    m.video.ObserveField("state", "onSubtitleState")
    ApplySubtitleSize()
end sub

function SubtitleNowMs() as integer
    return m.subtitleWatch.TotalMilliseconds()
end function

sub SubtitlesAttach(command as object)
    SubtitlesClear()
    m.video.suppressCaptions = true
    m.subtitleClock = SubtitleClockHeld(AttachSeconds(command.offsetMs, 0) * 1000, SubtitleNowMs(), false)

    path = SubtitleFileUrl(command.stream, command.captions)
    if IsBlank(path) then return

    session = SessionFor(m.global)
    task = CreateObject("roSGNode", "SubtitleTask")
    task.job = { attachId: command.id, url: StreamUrl(session.serverUrl, path), certificates: CertificatesFile(), timeout: SubtitleFetchSeconds() }
    task.ObserveField("result", "onSubtitlesFetched")
    task.control = "run"
    m.subtitleTask = task
end sub

sub SubtitlesSought(offsetMs as integer)
    m.subtitleClock = SubtitleClockHeld(offsetMs, SubtitleNowMs(), m.subtitleClock.playing)
    ShowSubtitlesNow()
end sub

sub SubtitlesClear()
    if m.subtitleTask <> invalid
        m.subtitleTask.UnobserveField("result")
        m.subtitleTask.control = "stop"
        m.subtitleTask = invalid
    end if

    m.subtitleTick.control = "stop"
    m.subtitleCues = []
    ShowSubtitleCues([])
end sub

sub onSubtitlesFetched(event as object)
    result = event.GetData()
    if Int(ValueAt(result, "attachId", -1)) <> m.attachId then return

    event.GetRoSGNode().UnobserveField("result")
    m.subtitleTask = invalid
    if ValueAt(result, "ok", false) <> true
        AppendPlaybackLog(m.playbackLog, PlaybackSubtitleFailedLine(NowClock(), TextOrBlank(ValueAt(result, "error", ""))))
        RefreshDiagnostics()
        return
    end if

    m.subtitleCues = result.cues
    if m.subtitleCues.Count() = 0 then return

    m.subtitleTick.control = "start"
    ShowSubtitlesNow()
end sub

sub onSubtitlePosition(event as object)
    position = event.GetData()
    if position = invalid then return
    m.subtitleClock = SubtitleClockRead(m.subtitleClock, Int(position * 1000.0), SubtitleNowMs())
end sub

sub onSubtitleState(event as object)
    m.subtitleClock = SubtitleClockState(m.subtitleClock, TextOrBlank(event.GetData()), SubtitleNowMs())
end sub

sub onSubtitleTick()
    ShowSubtitlesNow()
end sub

sub ShowSubtitlesNow()
    if m.subtitleCues.Count() = 0 then return

    shown = []
    if SubtitlesOnScreen(m.painted, TextOrBlank(m.video.state)) then shown = ActiveCues(m.subtitleCues, SubtitleClockAt(m.subtitleClock, SubtitleNowMs()))
    ShowSubtitleCues(shown)
end sub

sub ShowSubtitleCues(shown as object)
    if SameIndexes(shown, m.subtitleShown) then return
    m.subtitleShown = shown

    lines = []
    for each index in shown
        lines.Append(m.subtitleCues[index].lines)
    end for
    m.subtitleView.lines = lines
end sub

sub ApplySubtitleSize()
    m.subtitleView.size = SubtitleTextSize(ReadSubtitleSize())
end sub
