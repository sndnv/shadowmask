sub init()
    m.stage = m.top.FindNode("stage")
    m.video = m.top.FindNode("video")
    m.note = m.top.FindNode("note")
    m.pausedMark = m.top.FindNode("pausedMark")
    m.pausedDisc = m.top.FindNode("pausedDisc")
    m.pausedGlyph = m.top.FindNode("pausedGlyph")
    m.beat = m.top.FindNode("heartbeat")
    m.giveUp = m.top.FindNode("giveUp")
    m.hide = m.top.FindNode("hide")

    m.bar = m.top.FindNode("bar")
    m.scrim = m.top.FindNode("scrim")
    m.header = m.top.FindNode("header")
    m.headerScrim = m.top.FindNode("headerScrim")
    m.barTitle = m.top.FindNode("barTitle")
    m.barFacts = m.top.FindNode("barFacts")
    m.barClock = m.top.FindNode("barClock")
    m.elapsed = m.top.FindNode("elapsed")
    m.timeline = m.top.FindNode("timeline")
    m.total = m.top.FindNode("total")
    m.actions = m.top.FindNode("actions")

    m.diagnostics = m.top.FindNode("diagnostics")
    m.diagnosticsPanel = m.top.FindNode("diagnosticsPanel")
    m.diagnosticsScrim = m.top.FindNode("diagnosticsScrim")
    m.hold = m.top.FindNode("hold")
    m.watchdog = m.top.FindNode("watchdog")
    m.upNext = m.top.FindNode("upNext")
    m.upNextScrim = m.top.FindNode("upNextScrim")
    m.upNextTitle = m.top.FindNode("upNextTitle")
    m.upNextCount = m.top.FindNode("upNextCount")
    m.upNextActions = m.top.FindNode("upNextActions")
    m.countdown = m.top.FindNode("countdown")

    m.showDiagnostics = ReadDiagnostics()
    m.audioGuideOn = false
    m.nextTarget = invalid
    m.nextEpisode = invalid
    m.nextSeasonId = ""
    m.previousTarget = invalid
    m.previousEpisode = invalid
    m.previousSeasonId = ""
    m.nextSeasonNumber = invalid
    m.previousSeasonNumber = invalid
    m.settingMenu = "settings"
    m.pickedKind = ""
    m.holdKey = ""
    m.holdTicks = 0
    m.holdTargetMs = -1
    m.holdDeferred = false
    m.finished = false
    m.choiceOptions = []
    m.settingRows = []
    m.remaining = 0
    m.errored = false
    m.retryMs = 0
    m.barVisible = false
    m.barCommitted = false
    m.barOpened = invalid
    m.captionsForced = false
    m.stallTicks = 0
    m.stallMark = -1
    m.stallWarned = false
    m.released = false
    m.loaded = false
    m.playing = false
    m.prebuffered = false
    m.painted = false
    m.beating = false
    m.applying = false
    m.session = invalid
    m.version = invalid
    m.controls = DefaultControls()
    m.carried = invalid
    m.resumeMs = 0
    m.pendingMs = 0
    m.tasks = []

    m.video.notificationInterval = 1
    m.video.enableTrickPlay = false
    m.video.enableUI = false
    m.video.ObserveField("state", "onVideoState")
    m.video.ObserveField("bufferingStatus", "onBuffering")
    m.video.ObserveField("position", "onPosition")
    m.beat.ObserveField("fire", "onBeatFired")
    m.giveUp.ObserveField("fire", "onGiveUp")
    m.hide.ObserveField("fire", "onHideFired")
    m.actions.ObserveField("activated", "onAction")
    m.actions.ObserveField("focusIndex", "onBarStepped")
    m.countdown.ObserveField("fire", "onCountdownFired")
    m.hold.ObserveField("fire", "onHoldFired")
    m.watchdog.duration = 1
    m.watchdog.ObserveField("fire", "onWatchdogFired")
    m.upNextActions.ObserveField("activated", "onUpNextAction")
    m.video.ObserveField("streamInfo", "onStreamInfo")
end sub

sub render()
    theme = m.top.theme
    target = m.top.target
    if theme = invalid or theme.Count() = 0 then return
    if target = invalid or target.Count() = 0 then return

    m.stage.color = theme.artBg
    m.stage.width = CanvasWidth()
    m.stage.height = CanvasHeight()
    RefreshStage()

    m.video.width = CanvasWidth()
    m.video.height = CanvasHeight()

    m.note.theme = theme
    m.note.fontSize = TypeScale().textBase
    m.note.noteWidth = ContentWidth()
    m.note.translation = [ContentLeft(), ContentTop()]

    DrawBar(theme)
    DrawPausedMark(theme)
    DrawDiagnostics(theme)
    DrawUpNext(theme)

    if not m.loaded
        m.loaded = true
        ShowNote(LoadingNote(), "loading")
        Load()
    end if

    if not m.barVisible then TakeFocus(m.stage)
end sub

sub DrawBar(theme as object)
    space = SpacingScale()
    left = ContentLeft()
    width = ContentWidth()
    times = 200
    row = 28

    DrawHeader(theme)

    m.timeline.theme = theme
    m.timeline.barWidth = width - (times + space.s3) * 2
    m.timeline.translation = [left + times + space.s3, 0]

    DrawTime(m.elapsed, theme, left, 0, times, row, "left")
    DrawTime(m.total, theme, left + width - times, 0, times, row, "right")

    actionsTop = row + space.s4
    m.actions.theme = theme
    m.actions.barWidth = width
    m.actions.translation = [left, actionsTop]
    BuildBarButtons()

    height = actionsTop + ActionControlHeight()
    m.bar.translation = [0, CanvasHeight() - height - space.s6]

    m.scrim.color = WithAlpha(theme.bg, "D9")
    m.scrim.width = CanvasWidth()
    m.scrim.height = height + space.s6 * 2
    m.scrim.translation = [0, 0 - space.s6]

    RefreshProgress()
end sub

sub DrawPausedMark(theme as object)
    size = PausedMarkSize()
    glyph = PausedGlyphSize()

    m.pausedDisc.uri = GlyphUri("disc")
    m.pausedDisc.width = size
    m.pausedDisc.height = size
    m.pausedDisc.blendColor = theme.bg
    m.pausedDisc.opacity = 0.7
    m.pausedDisc.translation = [0, 0]

    m.pausedGlyph.width = glyph
    m.pausedGlyph.height = glyph
    m.pausedGlyph.blendColor = theme.text
    m.pausedGlyph.translation = [Int((size - glyph) / 2), Int((size - glyph) / 2)]

    m.pausedMark.translation = [Int((CanvasWidth() - size) / 2), Int((CanvasHeight() - size) / 2)]
end sub

sub RefreshPausedMark()
    m.pausedMark.visible = m.prebuffered and not m.playing and not m.errored and not m.upNext.visible
    if not m.pausedMark.visible then return

    if m.finished
        m.pausedGlyph.uri = GlyphUri("icon-replay-large")
        return
    end if

    m.pausedGlyph.uri = GlyphUri("icon-play-large")
end sub

sub DrawHeader(theme as object)
    sizes = TypeScale()
    space = SpacingScale()
    clockWidth = 220

    m.barTitle.text = TextOrBlank(ValueAt(m.top.target, "title", ""))
    m.barTitle.color = theme.text
    m.barTitle.font = SizedBoldFont(sizes.textLg)
    m.barTitle.width = ContentWidth() - clockWidth - space.s4
    m.barTitle.maxLines = 1
    m.barTitle.ellipsisText = "…"
    top = space.s5
    m.barTitle.translation = [ContentLeft(), top]

    m.barClock.color = theme.muted
    m.barClock.font = SizedFont(sizes.textSm)
    m.barClock.width = clockWidth
    m.barClock.maxLines = 1
    m.barClock.horizAlign = "right"
    m.barClock.translation = [ContentLeft() + ContentWidth() - clockWidth, top]

    factsTop = top + TextLinePitch(sizes.textLg)
    m.barFacts.color = theme.muted
    m.barFacts.font = SizedFont(sizes.textSm)
    m.barFacts.width = m.barTitle.width
    m.barFacts.maxLines = 1
    m.barFacts.ellipsisText = "…"
    m.barFacts.translation = [ContentLeft(), factsTop]

    RefreshHeader()

    height = factsTop + TextLinePitch(sizes.textSm) + space.s4
    m.headerScrim.color = WithAlpha(theme.bg, "D9")
    m.headerScrim.width = CanvasWidth()
    m.headerScrim.height = height
end sub

sub RefreshHeader()
    m.barFacts.text = JoinParts([NumberText(ValueAt(m.top.target, "year", invalid)), FormatDurationText(FullDurationMs())])
    m.barClock.text = CurrentClockText()
end sub

function CurrentClockText() as string
    if m.clock = invalid then m.clock = CreateObject("roDateTime")
    m.clock.Mark()
    m.clock.ToLocalTime()

    use24 = false
    device = DeviceInfo()
    if device <> invalid then use24 = TextOrBlank(device.GetClockFormat()) = "24h"

    return ClockText(m.clock.GetHours(), m.clock.GetMinutes(), use24)
end function

sub DrawTime(node as object, theme as object, left as integer, top as integer, width as integer, height as integer, align as string)
    node.color = theme.muted
    node.font = SizedFont(TypeScale().textSm)
    node.width = width
    node.height = height
    node.maxLines = 1
    node.horizAlign = align
    node.vertAlign = "center"
    node.translation = [left, top]
end sub

sub BuildBarButtons()
    if m.errored
        m.actions.buttons = [{ id: "retry", label: Phrase("action.retry"), style: "primary" }]
        return
    end if

    m.actions.buttons = PlayerButtons(m.playing, m.finished, EpisodeContext() <> invalid, m.previousTarget <> invalid, m.nextTarget <> invalid)
end sub

function FullDurationMs() as integer
    total = Int(ValueAt(m.version, "duration_ms", 0))
    if total > 0 then return total

    return Int(m.video.duration * 1000.0)
end function

sub RefreshProgress()
    at = TimelineNow()
    total = FullDurationMs()

    m.timeline.durationMs = total
    m.timeline.positionMs = at

    shown = at
    if m.holdTargetMs >= 0 then shown = m.holdTargetMs

    m.elapsed.text = PositionText(shown, total, ReadRemainingTime())
    m.total.text = FormatDuration(total)
end sub

sub ShowBar(seeking = false as boolean)
    if m.barVisible then return

    m.audioGuideOn = AudioGuideOn()
    m.barVisible = true
    m.bar.visible = true
    m.header.visible = true
    ComposeBar(seeking)
    RestartHide()
end sub

sub ComposeBar(seeking = false as boolean)
    ShowTimeRow(not m.errored)
    RefreshHeader()
    RefreshProgress()
    BuildBarButtons()

    if seeking and not m.errored
        TakeFocus(m.timeline)
        return
    end if

    m.actions.focusIndex = TransportIndex(m.actions.buttons)
    TakeFocus(m.actions)
end sub

sub onFocus()
    if not m.top.hasFocus() then return

    if m.upNext.visible
        m.upNextActions.SetFocus(true)
        return
    end if

    if m.barVisible
        m.actions.SetFocus(true)
        return
    end if

    m.stage.SetFocus(true)
end sub

sub ShowTimeRow(shown as boolean)
    m.timeline.visible = shown
    m.elapsed.visible = shown
    m.total.visible = shown
end sub

sub HideBar()
    if not m.barVisible then return

    m.barVisible = false
    m.barCommitted = false
    m.barOpened = invalid
    m.hide.control = "stop"
    TakeFocus(m.stage)
    m.bar.visible = false
    m.header.visible = false
end sub

sub MarkBarOpenedByBack()
    m.barOpened = CreateObject("roTimespan")
end sub

sub MarkBarCommitted()
    m.barCommitted = true
end sub

function HandledBarBack() as boolean
    if not m.playing then return false
    if m.barCommitted
        HideBar()
        return true
    end if

    if m.barOpened <> invalid and m.barOpened.TotalMilliseconds() <= DoubleBackMs() then return false

    HideBar()
    return true
end function

sub RestartHide()
    m.hide.control = "stop"
    if not m.playing then return

    m.hide.duration = HideSecondsFor(m.audioGuideOn)
    m.hide.control = "start"
end sub

sub onBarStepped()
    RestartHide()
end sub

sub onHideFired()
    if not IsBlank(m.holdKey) or m.note.visible then return

    HideBar()
end sub

sub onPosition()
    if m.barVisible
        RefreshHeader()
        RefreshProgress()
    end if
    RefreshDiagnostics()
end sub

sub onAction(event as object)
    id = TextOrBlank(event.GetData())

    if id = "replay"
        RestartPlayback()
        return
    end if

    if id = "retry"
        RetryPlayback()
        return
    end if

    if id = "subtitle"
        OpenSubtitleMenu()
        return
    end if

    if id = "audio"
        OpenAudioMenu()
        return
    end if

    if id = "quality"
        OpenQuality()
        return
    end if

    if id = "settings"
        OpenSettings()
        return
    end if

    if id = "previous"
        PlayNeighbour(m.previousTarget)
        return
    end if

    if id = "next"
        PlayNeighbour(m.nextTarget)
        return
    end if

    if id = "play" then TogglePlay()
    RestartHide()
end sub

sub AskChoice(field as string, title as string, options as object, selected as integer, kind = "choice" as string)
    m.hide.control = "stop"
    m.choiceOptions = options
    m.top.choiceRequest = {
        field: field,
        title: title,
        kind: kind,
        options: MenuOptions(options),
        selected: selected
    }
end sub

function PlayerFlags() as object
    return {
        autoplay: ReadAutoplayNext(),
        autoplayDelay: ReadAutoplayDelay(),
        remaining: ReadRemainingTime(),
        diagnostics: m.showDiagnostics
    }
end function

sub OpenSettings()
    m.settingMenu = "settings"
    m.settingRows = SettingsRows(m.controls, PlayerFlags())
    AskChoice("settings", Phrase("player.settings"), m.settingRows, 0, "menu")
end sub

sub OpenSubtitleMenu()
    m.settingMenu = "subsMenu"
    m.settingRows = SubtitleRows(m.controls, m.version)
    AskChoice("subsMenu", Phrase("heading.subtitles"), m.settingRows, 0, "menu")
end sub

sub OpenAudioMenu()
    m.settingMenu = "audioMenu"
    m.settingRows = AudioRows(m.controls, m.version)
    AskChoice("audioMenu", Phrase("heading.audio"), m.settingRows, 0, "menu")
end sub

sub ReopenMenu()
    if m.settingMenu = "subsMenu"
        OpenSubtitleMenu()
        return
    end if

    if m.settingMenu = "audioMenu"
        OpenAudioMenu()
        return
    end if

    OpenSettings()
end sub

sub OpenQuality()
    options = QualityRungs(SourceHeightOf(m.version))
    AskChoice("quality", Phrase("player.quality"), options, IndexOfValue(options, "height", Int(m.controls.height)))
end sub

sub OpenAudio()
    options = AudioOptions(m.version)
    if options.Count() = 0 then return

    AskChoice("audio", Phrase("heading.audio"), options, IndexOfValue(options, "index", Int(m.controls.audioTrack)))
end sub

sub OpenSubtitle()
    options = SubtitleOptions(m.version)
    AskChoice("subtitle", Phrase("heading.subtitles"), options, IndexOfSubtitle(options, m.controls))
end sub

sub OpenDelivery()
    options = DeliveryOptions()
    AskChoice("delivery", Phrase("player.converting"), options, IndexOfValue(options, "value", TextOrBlank(m.controls.delivery)))
end sub

sub OpenAutoplay()
    options = AutoplayOptions()
    current = AutoplayValue(ReadAutoplayNext(), ReadAutoplayDelay())
    AskChoice("autoplay", Phrase("player.autoplayNext"), options, IndexOfValue(options, "value", current))
end sub

sub OpenOffset()
    steps = [
        { id: "earlier", label: Phrase("player.offsetEarlier") },
        { id: "later", label: Phrase("player.offsetLater") },
        { id: "reset", label: Phrase("player.offsetReset") }
    ]
    AskChoice("offset", SettingRow(Phrase("player.subtitleOffset"), OffsetLabel(Int(m.controls.offsetMs))), steps, 0)
end sub

sub onChoice()
    result = m.top.choiceResult
    if result = invalid or m.released then return

    if TextOrBlank(ValueAt(result, "field", "")) = "stalled"
        HandledStall(result)
        return
    end if

    FocusBar()
    if result.cancelled
        if ReopenedSettingMenu(TextOrBlank(result.field)) then return

        RestartHide()
        return
    end if

    if type(m.choiceOptions) <> "roArray" or m.choiceOptions.Count() = 0 then return

    chosen = m.choiceOptions[ClampInt(Int(result.index), 0, m.choiceOptions.Count() - 1)]
    RunChoice(TextOrBlank(result.field), chosen)
end sub

function ReopenedSettingMenu(field as string) as boolean
    leaves = ["quality", "audio", "subtitle", "delivery", "offset", "autoplay"]

    isLeaf = false
    for each name in leaves
        if name = field then isLeaf = true
    end for
    if not isLeaf then return false

    menu = TextOrBlank(m.settingMenu)
    if menu = "subsMenu"
        OpenSubtitleMenu()
        return true
    end if

    if menu = "audioMenu"
        OpenAudioMenu()
        return true
    end if

    if menu = "settings"
        OpenSettings()
        return true
    end if

    return false
end function

sub RunChoice(field as string, chosen as object)
    if field = "settings" or field = "subsMenu" or field = "audioMenu"
        RunSetting(TextOrBlank(ValueAt(chosen, "id", "")))
        return
    end if

    if field = "quality"
        m.controls.height = Int(ValueAt(chosen, "height", 0))
        ApplyControls()
        return
    end if

    if field = "audio"
        m.controls.audioTrack = Int(ValueAt(chosen, "index", -1))
        ApplyControls()
        return
    end if

    if field = "subtitle"
        ref = ValueAt(chosen, "ref", invalid)
        m.controls.subtitle = ref
        m.controls.subtitleOff = ref = invalid
        ApplyControls()
        return
    end if

    if field = "delivery"
        m.controls.delivery = TextOrBlank(ValueAt(chosen, "value", "auto"))
        ApplyControls()
        return
    end if

    if field = "autoplay"
        StoreAutoplay(Int(ValueAt(chosen, "value", DefaultCountdownSeconds())))
        MarkBarCommitted()
        return
    end if

    if field = "offset" then StepOffset(TextOrBlank(ValueAt(chosen, "id", "")))
end sub

sub StepOffset(towards as string)
    if towards = "reset"
        m.controls.offsetMs = 0
    else if towards = "earlier"
        m.controls.offsetMs = SteppedOffset(Int(m.controls.offsetMs), 0 - OffsetStepMs())
    else
        m.controls.offsetMs = SteppedOffset(Int(m.controls.offsetMs), OffsetStepMs())
    end if

    ApplyControls()
    OpenOffset()
end sub

sub RunSetting(id as string)
    if id = "audio"
        OpenAudio()
        return
    end if
    if id = "subtitle"
        OpenSubtitle()
        return
    end if
    if id = "delivery"
        OpenDelivery()
        return
    end if
    if id = "offset"
        OpenOffset()
        return
    end if

    if id = "burn"
        m.controls.burn = not (m.controls.burn = true)
        ApplyControls()
        ReopenMenu()
        return
    end if
    if id = "downmix"
        m.controls.downmix = not (m.controls.downmix = true)
        ApplyControls()
        ReopenMenu()
        return
    end if

    MarkBarCommitted()

    if id = "autoplay"
        OpenAutoplay()
        return
    else if id = "diagnostics"
        ToggleDiagnostics()
    else if id = "remaining"
        WriteRemainingTime(not ReadRemainingTime())
        RefreshProgress()
    end if
    ReopenMenu()
end sub

sub ApplyControls()
    if m.session = invalid then return
    if m.applying then return

    m.applying = true
    MarkBarCommitted()
    m.pendingMs = TimelineNow()
    m.video.control = "stop"
    ShowNote(LoadingNote(), "loading")

    session = SessionFor(m.global)
    m.updateTask = SendRequest(UpdateSessionRequest(session.serverUrl, session.token, m.session.id, m.controls), "onUpdated")
end sub

sub onUpdated(event as object)
    parsed = Answered(event)
    m.applying = false
    if m.released then return

    if not parsed.ok
        FailPlayback(PhraseWith("error.couldNotPlay", { detail: PlaybackDetail(parsed) }), m.pendingMs)
        return
    end if

    Adopt(parsed.json)
    Attach(m.pendingMs)
end sub

sub FocusBar()
    if not m.barVisible then return

    TakeFocus(m.actions)
end sub

sub RestartPlayback()
    m.retryMs = 0
    ReloadPlayback()
end sub

sub RetryPlayback()
    ReloadPlayback()
end sub

sub ReloadPlayback()
    EndSession()

    m.session = invalid
    m.version = invalid
    m.controls = DefaultControls()
    m.nextTarget = invalid
    m.nextEpisode = invalid
    m.previousTarget = invalid
    m.previousEpisode = invalid
    m.resumeMs = 0
    m.prebuffered = false
    m.painted = false
    m.playing = false
    m.finished = false
    m.errored = false
    ClearHold()

    HideBar()
    ShowTimeRow(true)
    ShowNote(LoadingNote(), "loading")
    Load()
end sub

sub DrawDiagnostics(theme as object)
    space = SpacingScale()
    width = Int(ContentWidth() * 0.42)

    m.diagnostics.theme = theme
    m.diagnostics.listWidth = width
    m.diagnostics.columns = 2
    m.diagnostics.translation = [space.s5, space.s5]

    m.diagnosticsScrim.color = WithAlpha(theme.bg, "D9")
    m.diagnosticsScrim.width = width + space.s5 * 2
    m.diagnosticsPanel.translation = [ContentLeft() + ContentWidth() - width - space.s5 * 2, ContentTop()]
    RefreshDiagnostics()
end sub

sub RefreshDiagnostics()
    m.diagnosticsPanel.visible = m.showDiagnostics
    if not m.showDiagnostics then return

    m.diagnostics.rows = PlayerDiagnostics(m.session, VideoInfo(m.video.streamInfo, m.video.bufferingStatus, m.video.timeToStartStreaming, TextOrBlank(m.video.state)))
    m.diagnosticsScrim.height = m.diagnostics.listHeight + SpacingScale().s5 * 2
end sub

sub onStreamInfo()
    RefreshDiagnostics()
end sub

sub ToggleDiagnostics()
    m.showDiagnostics = not m.showDiagnostics
    WriteDiagnostics(m.showDiagnostics)
    RefreshDiagnostics()
end sub

sub DrawUpNext(theme as object)
    sizes = TypeScale()
    space = SpacingScale()
    left = ContentLeft()
    width = ContentWidth()

    m.upNextCount.color = theme.muted
    m.upNextCount.font = SizedFont(sizes.textSm)
    m.upNextCount.width = width
    m.upNextCount.maxLines = 1
    m.upNextCount.translation = [left, 0]

    titleTop = TextLinePitch(sizes.textSm)
    m.upNextTitle.color = theme.text
    m.upNextTitle.font = SizedBoldFont(sizes.textLg)
    m.upNextTitle.width = width
    m.upNextTitle.maxLines = 1
    m.upNextTitle.ellipsisText = "…"
    m.upNextTitle.translation = [left, titleTop]

    actionsTop = titleTop + TextLinePitch(sizes.textLg) + space.s3
    m.upNextActions.theme = theme
    m.upNextActions.barWidth = width
    m.upNextActions.buttons = [
        { id: "playNow", label: Phrase("action.playNow"), style: "primary" },
        { id: "cancel", label: Phrase("action.cancel") }
    ]
    m.upNextActions.translation = [left, actionsTop]

    height = actionsTop + ActionControlHeight()
    m.upNext.translation = [0, CanvasHeight() - height - space.s6]

    m.upNextScrim.color = WithAlpha(theme.bg, "D9")
    m.upNextScrim.width = CanvasWidth()
    m.upNextScrim.height = height + space.s6 * 2
    m.upNextScrim.translation = [0, 0 - space.s6]
end sub

function EpisodeContext() as dynamic
    context = ValueAt(m.top.target, "episode", invalid)
    if type(context) <> "roAssociativeArray" then return invalid
    if IsBlank(TextOrBlank(ValueAt(context, "id", ""))) then return invalid
    if IsBlank(TextOrBlank(ValueAt(context, "seriesId", ""))) then return invalid
    if IsBlank(TextOrBlank(ValueAt(context, "seasonId", ""))) then return invalid

    return context
end function

sub ResolveNext()
    context = EpisodeContext()
    if context = invalid then return

    session = SessionFor(m.global)
    Ask(EpisodesRequest(session.serverUrl, session.token, context.seriesId, context.seasonId), "onNextEpisodes")
end sub

sub onNextEpisodes(event as object)
    parsed = Answered(event)
    if m.released or not parsed.ok then return

    context = EpisodeContext()
    if context = invalid then return

    episodes = ListItems(parsed.json)
    AdoptPreviousEpisode(PreviousEpisodeBefore(episodes, context.id), context.seasonId)

    episode = NextEpisodeAfter(episodes, context.id)
    if episode <> invalid
        AdoptNextEpisode(episode, context.seasonId)
        if m.previousEpisode <> invalid then return
    end if

    session = SessionFor(m.global)
    Ask(SeasonsRequest(session.serverUrl, session.token, context.seriesId), "onNextSeasons")
end sub

sub onNextSeasons(event as object)
    parsed = Answered(event)
    if m.released or not parsed.ok then return

    context = EpisodeContext()
    if context = invalid then return

    session = SessionFor(m.global)
    neighbours = SeasonNeighbours(context.seasonId, ListItems(parsed.json))

    if m.previousEpisode = invalid and neighbours.previous <> invalid
        m.previousSeasonNumber = ValueAt(neighbours.previous, "number", invalid)
        m.previousSeasonId = TextOrBlank(ValueAt(neighbours.previous, "id", ""))
        if not IsBlank(m.previousSeasonId)
            Ask(EpisodesRequest(session.serverUrl, session.token, context.seriesId, m.previousSeasonId), "onPreviousSeasonEpisodes")
        end if
    end if

    if m.nextEpisode <> invalid or neighbours.following = invalid then return

    m.nextSeasonNumber = ValueAt(neighbours.following, "number", invalid)
    m.nextSeasonId = TextOrBlank(ValueAt(neighbours.following, "id", ""))
    if IsBlank(m.nextSeasonId) then return

    Ask(EpisodesRequest(session.serverUrl, session.token, context.seriesId, m.nextSeasonId), "onNextSeasonEpisodes")
end sub

sub onNextSeasonEpisodes(event as object)
    parsed = Answered(event)
    if m.released or not parsed.ok then return

    episode = FirstEpisodeOf(ListItems(parsed.json))
    if episode = invalid then return

    AdoptNextEpisode(episode, m.nextSeasonId)
end sub

sub onPreviousSeasonEpisodes(event as object)
    parsed = Answered(event)
    if m.released or not parsed.ok then return

    AdoptPreviousEpisode(LastEpisodeOf(ListItems(parsed.json)), m.previousSeasonId)
end sub

sub AdoptNextEpisode(episode as object, seasonId as string)
    context = EpisodeContext()
    if context = invalid or IsBlank(seasonId) then return

    m.nextEpisode = episode
    m.nextSeasonId = seasonId

    session = SessionFor(m.global)
    Ask(EpisodeVersionsRequest(session.serverUrl, session.token, context.seriesId, seasonId, TextOrBlank(ValueAt(episode, "id", ""))), "onNextVersions")
end sub

sub AdoptPreviousEpisode(episode as dynamic, seasonId as string)
    context = EpisodeContext()
    if context = invalid or episode = invalid or IsBlank(seasonId) then return

    m.previousEpisode = episode
    m.previousSeasonId = seasonId

    session = SessionFor(m.global)
    Ask(EpisodeVersionsRequest(session.serverUrl, session.token, context.seriesId, seasonId, TextOrBlank(ValueAt(episode, "id", ""))), "onPreviousVersions")
end sub

sub onNextVersions(event as object)
    parsed = Answered(event)
    if m.released or not parsed.ok then return

    m.nextTarget = NeighbourTarget(m.nextEpisode, m.nextSeasonId, parsed.json)
    if m.barVisible then BuildBarButtons()
end sub

sub onPreviousVersions(event as object)
    parsed = Answered(event)
    if m.released or not parsed.ok then return

    m.previousTarget = NeighbourTarget(m.previousEpisode, m.previousSeasonId, parsed.json)
    if m.barVisible then BuildBarButtons()
end sub

function NeighbourTarget(episode as dynamic, seasonId as string, json as dynamic) as dynamic
    context = EpisodeContext()
    if context = invalid then return invalid

    season = invalid
    if seasonId = context.seasonId then season = ValueAt(m.top.target, "seasonNumber", invalid)
    if seasonId = m.nextSeasonId and m.nextSeasonNumber <> invalid then season = m.nextSeasonNumber
    if seasonId = m.previousSeasonId and m.previousSeasonNumber <> invalid then season = m.previousSeasonNumber

    return EpisodePlayTarget(episode, context.seriesId, seasonId, json, ValueAt(m.top.target, "seriesTitle", ""), season)
end function

function StartCountdown() as boolean
    if not ReadAutoplayNext() or m.nextTarget = invalid then return false

    HideBar()
    m.remaining = CountdownSeconds()
    m.upNextTitle.text = UpNextLabel(m.nextTarget)
    RefreshCountdown()

    m.upNext.visible = true
    RefreshPausedMark()
    TakeFocus(m.upNextActions)
    m.countdown.duration = 1
    m.countdown.control = "start"
    return true
end function

sub RefreshCountdown()
    m.upNextCount.text = PhraseWith("player.upNextIn", { seconds: m.remaining })
end sub

sub onCountdownFired()
    m.remaining = m.remaining - 1
    if m.remaining > 0
        RefreshCountdown()
        return
    end if

    PlayNext()
end sub

sub DismissUpNext()
    StopCountdown()
    ShowNote("", "empty")
    ShowBar()
end sub

sub StopCountdown()
    m.countdown.control = "stop"
    TakeFocus(m.stage)
    m.upNext.visible = false
    RefreshPausedMark()
end sub

sub onUpNextAction(event as object)
    if TextOrBlank(event.GetData()) = "playNow"
        PlayNext()
        return
    end if

    DismissUpNext()
end sub

sub PlayNext()
    PlayNeighbour(m.nextTarget)
end sub

sub PlayNeighbour(target as dynamic)
    if target = invalid then return

    m.top.choiceRequest = DialogCloseRequest()
    m.carried = CarriedTracks(m.controls, m.version)

    StopCountdown()
    EndSession()

    m.nextTarget = invalid
    m.nextEpisode = invalid
    m.previousTarget = invalid
    m.previousEpisode = invalid
    m.session = invalid
    m.version = invalid
    m.controls = DefaultControls()
    m.resumeMs = 0
    m.retryMs = 0
    m.prebuffered = false
    m.painted = false
    m.playing = false
    m.finished = false
    m.errored = false
    m.loaded = false
    ClearHold()

    HideBar()
    ShowTimeRow(true)
    m.top.target = target
    ShowNote(LoadingNote(), "loading")
end sub

sub EndSession()
    StopBeating()
    m.video.control = "stop"
    m.global.sessionId = ""
    if m.session <> invalid and not IsBlank(m.session.id) then m.top.sessionEnd = m.session.id
end sub

function PlayingVersionId() as string
    return TextOrBlank(ValueAt(m.top.target, "versionId", ""))
end function

sub Load()
    if IsBlank(PlayingVersionId())
        ShowNote(Phrase("empty.nothingToPlay"), "empty")
        return
    end if

    session = SessionFor(m.global)
    Ask(VersionRequest(session.serverUrl, session.token, PlayingVersionId()), "onVersion")
end sub

function Ask(request as object, callback as string) as object
    task = SendRequest(request, callback)
    m.tasks.Push(task)
    return task
end function

function Answered(event as object) as object
    if m.released then return { ok: false, json: invalid, status: 0, error: "" }

    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return { ok: false, json: invalid, status: parsed.status, error: "" }
    return parsed
end function

sub onVersion(event as object)
    parsed = Answered(event)
    if m.released then return

    if not parsed.ok
        FailPlayback(Phrase("error.couldNotLoadVersion"), 0)
        return
    end if

    m.version = parsed.json

    if m.carried <> invalid
        m.controls = ControlsForCarried(m.carried, m.version)
        m.carried = invalid
    end if

    if AskedPickedTitle() then return
    AskResume()
end sub

function AskedPickedTitle() as boolean
    if ValueAt(m.top.target, "resolve", false) <> true then return false

    ref = ValueAt(m.version, "title", invalid)
    kind = TextOrBlank(ValueAt(ref, "type", ""))
    id = TextOrBlank(ValueAt(ref, "id", ""))
    if IsBlank(id) then return false

    m.pickedKind = kind
    session = SessionFor(m.global)

    if kind = "episode"
        Ask(EpisodeByIdRequest(session.serverUrl, session.token, id), "onPickedTitle")
    else
        Ask(MovieDetailRequest(session.serverUrl, session.token, id), "onPickedTitle")
    end if

    return true
end function

sub onPickedTitle(event as object)
    parsed = Answered(event)
    if m.released then return

    if parsed.ok
        m.top.target = PickedPlayTarget(m.top.target, m.pickedKind, parsed.json)
        ShowNote(LoadingNote(), "loading")
    end if

    AskResume()
end sub

sub AskResume()
    session = SessionFor(m.global)
    Ask(ResumeRequest(session.serverUrl, session.token, session.userId, PlayingVersionId()), "onResume")
end sub

sub onResume(event as object)
    parsed = Answered(event)
    if m.released then return

    m.resumeMs = RetryPosition(m.retryMs, Int(ValueAt(parsed.json, "position_ms", 0)))
    m.retryMs = 0
    StartSession()
end sub

sub StartSession()
    session = SessionFor(m.global)
    capabilities = CapabilitiesBody(DeviceDecoding(), Int(m.global.profileVersion))

    Ask(StartSessionRequest(session.serverUrl, session.token, PlayingVersionId(), m.resumeMs, capabilities, m.controls), "onStarted")
end sub

sub onStarted(event as object)
    parsed = Answered(event)
    if m.released then return

    if not parsed.ok
        FailPlayback(PhraseWith("error.couldNotPlay", { detail: PlaybackDetail(parsed) }), m.resumeMs)
        return
    end if

    Adopt(parsed.json)
    Attach(m.resumeMs)
    ResolveNext()
end sub

sub Adopt(json as dynamic)
    m.session = SessionFrom(json)
    m.controls = ControlsWithSelection(m.controls, m.session.selected)
    m.global.negotiated = m.session
    m.global.sessionId = TextOrBlank(m.session.id)
    RefreshDiagnostics()
end sub

sub Attach(positionMs as integer)
    if m.session = invalid then return

    session = SessionFor(m.global)
    bounds = AbrBounds(0)

    content = CreateObject("roSGNode", "ContentNode")
    content.url = StreamUrl(session.serverUrl, m.session.manifestUrl)
    content.streamFormat = StreamFormatFor(m.session.mode, ValueAt(m.version, "container", ""))
    content.title = TextOrBlank(ValueAt(m.top.target, "title", ""))
    content.HttpCertificatesFile = CertificatesFile()
    content.SwitchingStrategy = bounds.strategy
    if bounds.maxKbps > 0 then content.MaxBandwidth = bounds.maxKbps
    content.PlayStart = AttachSeconds(positionMs, m.session.originMs, m.session.sequential)

    m.prebuffered = false
    m.painted = false
    RefreshPausedMark()
    ApplyCaptionMode()
    m.video.content = content

    m.video.control = "prebuffer"
    m.giveUp.duration = PrebufferGiveUpSeconds()
    m.giveUp.control = "start"
end sub

function CaptionsWanted() as boolean
    if m.session = invalid then return false
    if SubtitlesAreBurned(m.session.selected) then return false

    return SubtitleRef(m.controls.subtitle) <> invalid
end function

sub ApplyCaptionMode()
    if CaptionsWanted()
        if TextOrBlank(m.global.captionMode) = "On" then return

        m.captionsForced = true
        m.video.globalCaptionMode = "On"
        return
    end if

    RestoreCaptionMode()
end sub

sub RestoreCaptionMode()
    if not m.captionsForced then return

    m.captionsForced = false
    mode = TextOrBlank(m.global.captionMode)
    if IsBlank(mode) then mode = "Off"

    m.video.globalCaptionMode = mode
end sub

sub onBuffering()
    if m.prebuffered then return

    status = m.video.bufferingStatus
    if type(status) <> "roAssociativeArray" then return
    if ValueAt(status, "prebufferDone", false) = true then Play()
end sub

sub onGiveUp()
    Play()
end sub

sub Play()
    if m.prebuffered then return

    m.prebuffered = true
    m.giveUp.control = "stop"
    RefreshStage()
    m.video.control = "play"
end sub

sub RefreshStage()
    m.stage.visible = m.painted or m.errored
end sub

sub onVideoState()
    state = TextOrBlank(m.video.state)

    if state = "playing"
        m.playing = true
        m.finished = false
        m.painted = true
        RefreshStage()
        PublishBackdropUrl("")
        ShowNote("", "loading")
        BuildBarButtons()
        RefreshPausedMark()
        RestartHide()
        StartBeating()
        StartWatchdog()
        return
    end if

    if state = "paused"
        m.playing = false
        StopWatchdog()
        BuildBarButtons()
        RefreshPausedMark()
        RestartHide()
        return
    end if

    if state = "finished"
        m.playing = false
        m.finished = true
        StopWatchdog()
        RefreshPausedMark()
        SendBeat("paused")
        StopBeating()
        if StartCountdown() then return

        BuildBarButtons()
        ShowBar()
        return
    end if

    if state = "error"
        StopWatchdog()
        FailPlayback(PhraseWith("error.couldNotPlay", { detail: VideoDetail() }), TimelineNow())
        RefreshPausedMark()
    end if
end sub

sub HandledStall(result as object)
    if result.cancelled <> true and Int(result.index) = 1
        m.top.advance = "back"
        return
    end if

    m.stallWarned = false
    StartWatchdog()
end sub

sub StartWatchdog()
    m.stallTicks = 0
    m.stallMark = -1
    m.stallWarned = false
    m.watchdog.control = "start"
end sub

sub StopWatchdog()
    m.stallTicks = 0
    m.stallMark = -1
    m.watchdog.control = "stop"
end sub

sub onWatchdogFired()
    if not m.playing or m.errored or m.stallWarned then return

    status = m.video.bufferingStatus
    buffered = 0
    if type(status) = "roAssociativeArray" then buffered = Int(ValueAt(status, "percentage", 0))

    mark = ProgressMark(Int(m.video.position * 1000.0), buffered)
    m.stallTicks = StallTicks(m.stallTicks, m.stallMark, mark)
    m.stallMark = mark

    if not IsStalled(m.stallTicks) then return

    m.stallWarned = true
    m.watchdog.control = "stop"
    m.top.choiceRequest = {
        field: "stalled",
        title: Phrase("player.stalled"),
        message: Phrase("player.stalledBody"),
        options: [Phrase("player.keepWaiting"), Phrase("action.back")],
        selected: 0
    }
end sub

function VideoDetail() as string
    detail = TextOrBlank(m.video.errorMsg)
    if not IsBlank(detail) then return detail

    return Phrase("error.couldNotLoad")
end function

function PlaybackDetail(parsed as object) as string
    detail = TextOrBlank(ValueAt(parsed, "error", ""))
    if not IsBlank(detail) then return detail

    return Phrase("error.couldNotLoad")
end function

sub StartBeating()
    if m.beating or m.session = invalid then return

    m.beating = true
    m.beat.duration = m.session.heartbeatSeconds
    m.beat.control = "start"
end sub

sub StopBeating()
    m.beating = false
    m.beat.control = "stop"
end sub

sub onBeatFired()
    SendBeat(PlaybackStateName())
end sub

function PlaybackStateName() as string
    if m.playing then return "playing"
    return "paused"
end function

sub SendBeat(state as string)
    if m.released or m.session = invalid or IsBlank(m.session.id) then return

    session = SessionFor(m.global)
    m.beatTask = SendRequest(HeartbeatRequest(session.serverUrl, session.token, m.session.id, TimelineNow(), state), "onBeatSent")
end sub

sub onBeatSent(event as object)
    parsed = Answered(event)
    if m.released or parsed.ok then return

    if parsed.status >= 400 and parsed.status < 500 then StopBeating()
end sub

function TimelineNow() as integer
    if m.session = invalid then return 0

    return TimelineMs(m.video.position, m.session.originMs, m.session.sequential)
end function

sub SeekBy(deltaMs as integer)
    ShowBar(true)
    if m.session = invalid then return

    SeekTo(TimelineNow() + deltaMs)
end sub

sub SeekTo(atMs as integer)
    if m.session = invalid then return

    MarkBarCommitted()
    produced = ProducedMs(m.video.streamingSegment, m.video.position)
    plan = SeekPlan(atMs, m.session.originMs, m.session.sequential, produced)

    if not plan.server
        m.video.seek = plan.seconds
        return
    end if

    SeekToServer(plan.positionMs)
end sub

sub SeekToServer(atMs as integer)
    if m.session = invalid then return

    m.pendingMs = atMs
    m.video.control = "stop"
    ShowNote(LoadingNote(), "loading")

    session = SessionFor(m.global)
    m.seekTask = SendRequest(SeekSessionRequest(session.serverUrl, session.token, m.session.id, atMs), "onSeeked")
end sub

sub StartHold(key as string)
    ShowBar(true)
    if m.session = invalid then return

    m.holdKey = key
    m.holdTicks = 0
    m.holdTargetMs = -1
    m.holdDeferred = false

    JumpBy(HoldStepMs(0), false)

    m.hold.control = "stop"
    m.hold.duration = HoldTickMs() / 1000.0
    m.hold.control = "start"
end sub

sub onHoldFired()
    if IsBlank(m.holdKey) then return

    JumpBy(HoldTickStepMs(m.holdTicks), HoldTickMultiplied(m.holdTicks))
    m.holdTicks = m.holdTicks + 1
end sub

function EndHold() as boolean
    if IsBlank(m.holdKey) then return false

    at = m.holdTargetMs
    deferred = m.holdDeferred
    ClearHold()

    if deferred and at >= 0 then SeekTo(at)
    RestartHide()
    return true
end function

sub ClearHold()
    m.holdKey = ""
    m.holdTicks = 0
    m.holdTargetMs = -1
    m.holdDeferred = false
    m.hold.control = "stop"
    m.timeline.scrubMs = -1
end sub

sub JumpBy(stepMs as integer, ghostOnly as boolean)
    if m.session = invalid then return

    delta = stepMs
    if m.holdKey = "left" then delta = 0 - stepMs

    base = m.holdTargetMs
    if base < 0 then base = TimelineNow()

    at = base + delta
    if at < 0 then at = 0
    total = FullDurationMs()
    if total > 0 and at > total then at = total
    m.holdTargetMs = at

    m.timeline.scrubMs = at
    RefreshProgress()

    if ghostOnly
        m.holdDeferred = true
        return
    end if

    produced = ProducedMs(m.video.streamingSegment, m.video.position)
    plan = SeekPlan(at, m.session.originMs, m.session.sequential, produced)

    if plan.server
        m.holdDeferred = true
        return
    end if

    m.holdDeferred = false
    m.video.seek = plan.seconds
end sub

sub onSeeked(event as object)
    parsed = Answered(event)
    if m.released then return

    if not parsed.ok
        FailPlayback(PhraseWith("error.couldNotPlay", { detail: PlaybackDetail(parsed) }), m.pendingMs)
        return
    end if

    Adopt(parsed.json)
    Attach(m.pendingMs)
end sub

sub TogglePlay()
    if m.session = invalid then return
    if not m.prebuffered
        Play()
        return
    end if

    if m.playing
        m.video.control = "pause"
        SendBeat("paused")
        return
    end if

    m.video.control = "resume"
end sub

sub FailPlayback(message as string, atMs as integer)
    m.errored = true
    m.retryMs = atMs
    m.playing = false
    m.finished = false
    RefreshStage()
    ClearHold()
    m.giveUp.control = "stop"
    StopCountdown()
    StopBeating()
    ShowNote(message, "error")

    if m.barVisible
        ComposeBar()
        return
    end if

    ShowBar()
end sub

function LoadingNote() as string
    title = TextOrBlank(ValueAt(m.top.target, "title", ""))
    if IsBlank(title) then return Phrase("player.loading")

    return PhraseWith("player.loadingTitle", { title: title })
end function

sub ShowNote(message as string, kind as string)
    m.note.kind = kind
    m.note.message = message
    m.note.visible = not IsBlank(message)
end sub

sub onRelease()
    if not m.top.release then return

    m.released = true
    m.giveUp.control = "stop"
    m.countdown.control = "stop"
    m.hold.control = "stop"
    m.watchdog.control = "stop"
    RestoreCaptionMode()
    EndSession()

    for each task in m.tasks
        if task <> invalid then task.control = "STOP"
    end for
    m.tasks = []
end sub

function onKeyEvent(key as string, press as boolean) as boolean
    if m.released or not KeysAreOurs() then return false

    if not press
        if key = "left" or key = "right" then return EndHold()
        return false
    end if

    if not IsBlank(m.holdKey) and key <> m.holdKey then EndHold()

    if key = "options"
        ShowBar()
        OpenSettings()
        return true
    end if

    if m.upNext.visible
        if key = "back"
            StopCountdown()
            ShowNote(Phrase("player.replay"), "empty")
            return true
        end if
        return false
    end if

    if m.errored then return false

    if m.barVisible
        RestartHide()
        if key = "back" then return HandledBarBack()
        if HandledStepKey(key, [m.timeline, m.actions]) then return true
    end if

    if key = "back"
        MarkBarOpenedByBack()
        ShowBar()
        return true
    end if

    if key = "up" or key = "down"
        ShowBar()
        return true
    end if

    if key = "OK"
        if not m.barVisible
            ShowBar()
            return true
        end if

        TogglePlay()
        return true
    end if

    if key = "play"
        TogglePlay()
        return true
    end if

    if key = "rewind"
        SeekBy(0 - SeekStepMs(true))
        return true
    end if

    if IsReplayKey(key)
        SeekBy(0 - SeekStepMs(false))
        return true
    end if

    if key = "fastforward"
        SeekBy(SeekStepMs(true))
        return true
    end if

    if key = "left" or key = "right"
        if m.holdKey = key then return true

        StartHold(key)
        return true
    end if

    return false
end function
