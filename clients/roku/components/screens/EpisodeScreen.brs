sub init()
    InitHero()
    InitCrumbs()
    InitLayoutTick()

    m.note = m.top.FindNode("note")
    m.actions = m.hero.actions
    m.navActions = m.hero.navActions
    m.overviewToggle = m.hero.overviewToggle

    m.actions.ObserveField("activated", "onAction")
    m.navActions.ObserveField("activated", "onAction")
    m.overviewToggle.ObserveField("activated", "onAction")

    InitSections([{ id: "hero", focus: "actions" }])

    m.id = ""
    m.seriesId = ""
    m.seasonId = ""
    m.episode = invalid
    m.ordered = []
    m.available = []
    m.resumable = {}
    m.watched = false
    m.watchlisted = false
    m.favorite = false
    m.seasons = []
    m.bySeason = {}
    m.pendingSeasons = 0
    m.neighbours = { previous: invalid, following: invalid }
    m.versionChoices = []
    m.dismissedVersion = ""
    m.tasks = []
    m.released = false
    m.started = false
    m.published = false
end sub

sub render()
    theme = m.top.theme
    target = m.top.target
    if theme = invalid or theme.Count() = 0 then return
    if target = invalid or target.Count() = 0 then return

    if IsBlank(m.id)
        m.id = TextOrBlank(ValueAt(target, "id", ""))
        m.seriesId = TextOrBlank(ValueAt(target, "seriesId", ""))
        m.seasonId = TextOrBlank(ValueAt(target, "seasonId", ""))
    end if

    Layout()

    if not m.started
        m.started = true
        Load()
    end if
end sub

sub Layout()
    theme = m.top.theme
    space = SpacingScale()
    left = ContentLeft()
    top = ContentTop()

    width = CanvasWidth() - left - space.s6

    DrawCrumbs(theme, CrumbList(), left, top, width)
    top = top + CrumbHeight()
    height = CanvasHeight() - top - space.s5

    m.viewport.translation = [left, top]

    m.note.theme = theme
    m.note.fontSize = TypeScale().textBase
    m.note.noteWidth = width
    m.note.translation = [left, top]

    BuildActions()
    ShowSection("hero", DrawHero(theme, HeroContent(), width))
    PlaceScrollBar(height)
    LayoutSections(width, height)
end sub

function CrumbList() as object
    series = TextOrBlank(ValueAt(m.episode, "series_title", ""))
    if IsBlank(series) then series = Phrase("nav.series")

    season = TextOrBlank(ValueAt(m.episode, "season_title", ""))
    if IsBlank(season)
        number = ValueAt(m.episode, "season_number", invalid)
        if number <> invalid then season = PhraseWith("status.season", { number: Int(number) })
    end if

    code = EpisodeCode(ValueAt(m.episode, "season_number", invalid), ValueAt(m.episode, "number", invalid))

    return [
        { label: Phrase("nav.series"), screen: "SeriesScreen", target: invalid },
        SeriesCrumb(series),
        SeasonCrumb(season),
        { label: code }
    ]
end function

function SeriesCrumb(series as string) as object
    if IsBlank(m.seriesId) then return { label: series }
    return { label: series, screen: "TitleScreen", target: { kind: "series", id: m.seriesId, title: series } }
end function

function SeasonCrumb(season as string) as object
    if IsBlank(m.seasonId) then return { label: season }
    return { label: season, screen: "SeasonScreen", target: { kind: "season", id: m.seasonId, seriesId: m.seriesId, title: season } }
end function

function HeroContent() as object
    return {
        aspect: LandscapeAspect(),
        imageUri: HeroImageUri(),
        placeholderUri: LandscapePlaceholder(),
        heading: TitleText(),
        facts: FactsText(EpisodeFacts(m.episode, BestVersion(m.ordered))),
        crew: TextOrBlank(ValueAt(m.episode, "series_title", "")),
        overview: OverviewText()
    }
end function

function HeroImageUri() as string
    if m.episode <> invalid
        card = { aspect: LandscapeAspect(), artwork: ValueAt(m.episode, "artwork", invalid) }
        url = CardImageUrl(SessionFor(m.global).serverUrl, card, HeroArtWidth(LandscapeAspect()))
        if not IsBlank(url) then return url
    end if
    return LandscapeFallbackUri(m.top.target)
end function

function TitleText() as string
    title = TextOrBlank(ValueAt(m.episode, "title", ""))
    if IsBlank(title) then title = TextOrBlank(ValueAt(m.top.target, "title", ""))

    code = EpisodeCode(ValueAt(m.episode, "season_number", invalid), ValueAt(m.episode, "number", invalid))
    if IsBlank(code) then return title
    return JoinParts([code, title])
end function

sub BuildActions()
    buttons = []

    if m.available.Count() > 0
        buttons.Push(PlayButton())
        if ResumeTargetId() <> "" then buttons.Push(DismissButton())
    end if

    buttons.Push({ id: "watched", label: Phrase("action.watchedLabel"), icon: "icon-check", iconOn: "icon-check", pressed: m.watched, collapsible: true })

    m.actions.theme = m.top.theme
    m.actions.barWidth = CanvasWidth() - ContentLeft() - SpacingScale().s6
    if buttons.Count() > 0 then m.actions.focusIndex = ClampInt(m.actions.focusIndex, 0, buttons.Count() - 1)
    m.actions.buttons = buttons

    steps = StepButtons(m.neighbours)

    m.navActions.theme = m.top.theme
    m.navActions.barWidth = CanvasWidth() - ContentLeft() - SpacingScale().s6
    if steps.Count() > 0 then m.navActions.focusIndex = ClampInt(m.navActions.focusIndex, 0, steps.Count() - 1)
    m.navActions.buttons = steps
end sub

function ResumeTargetId() as string
    target = ResolvePlayTarget(m.available, m.resumable)
    if target = invalid then return ""

    id = TextOrBlank(ValueAt(target, "id", ""))
    if ResumePercentFor(m.resumable, id) <= 0 then return ""
    return id
end function

function PlayButton() as object
    target = ResolvePlayTarget(m.available, m.resumable)
    if target = invalid
        return { id: "play", label: Phrase("detail.chooseVersion"), icon: "icon-play", style: "primary" }
    end if

    if ResumePercentFor(m.resumable, ValueAt(target, "id", "")) > 0
        return { id: "play", label: Phrase("action.resume"), icon: "icon-play", style: "primary" }
    end if
    return { id: "play", label: Phrase("action.play"), icon: "icon-play", style: "primary" }
end function

function DismissButton() as object
    return { id: "dismissResume", label: Phrase("action.dismissResume"), icon: "icon-close", style: "joined", joined: true }
end function

sub Load()
    session = SessionFor(m.global)
    ShowNote(Phrase("state.loading"), "loading")

    Ask(EpisodeRequest(session.serverUrl, session.token, m.seriesId, m.seasonId, m.id), "onEpisode")
    Ask(EpisodeVersionsRequest(session.serverUrl, session.token, m.seriesId, m.seasonId, m.id), "onVersions")
    Ask(ContinueRequest(session.serverUrl, session.token, session.userId), "onContinue")
    LoadState()
    Ask(SeasonsRequest(session.serverUrl, session.token, m.seriesId), "onSeasons")
end sub

sub LoadState()
    session = SessionFor(m.global)
    Ask(StateBatchRequest(session.serverUrl, session.token, session.userId, [{ "type": "episode", "id": m.id }]), "onState")
end sub

function Ask(request as object, callback as string) as object
    task = SendRequest(request, callback)
    m.tasks.Push(task)
    return task
end function

function Answered(event as object) as object
    if m.released then return { ok: false, json: invalid, status: 0 }

    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return { ok: false, json: invalid, status: parsed.status }
    return parsed
end function

sub onEpisode(event as object)
    parsed = Answered(event)
    if m.released then return

    if not parsed.ok
        ShowNote(Phrase("error.couldNotLoadEpisode"), "error")
        return
    end if

    m.episode = parsed.json
    if IsBlank(m.seriesId) then m.seriesId = TextOrBlank(ValueAt(m.episode, "series_id", ""))
    if IsBlank(m.seasonId) then m.seasonId = TextOrBlank(ValueAt(m.episode, "season_id", ""))

    PublishBackdropFrom(EpisodeBackdropCandidates(m.episode))
    Refresh()
end sub

sub onVersions(event as object)
    parsed = Answered(event)
    if m.released then return

    if parsed.ok
        m.ordered = OrderedVersions(ValueAt(parsed.json, "items", invalid))
        m.available = AvailableVersions(m.ordered)
    end if
    Refresh()
end sub

sub onContinue(event as object)
    parsed = Answered(event)
    if m.released then return

    if parsed.ok then m.resumable = ResumeProgressMap(ContinueCards(parsed.json))
    Refresh()
end sub

sub onState(event as object)
    parsed = Answered(event)
    if m.released then return

    if parsed.ok and type(parsed.json) = "roArray" and parsed.json.Count() > 0
        entry = parsed.json[0]
        m.watched = ValueAt(entry, "watched", false) = true
        m.watchlisted = ValueAt(entry, "watchlisted", false) = true
        m.favorite = ValueAt(entry, "favorite", false) = true
    end if
    Refresh()
end sub

sub onSeasons(event as object)
    parsed = Answered(event)
    if m.released then return

    if not parsed.ok or type(parsed.json) <> "roArray" then return

    m.seasons = parsed.json
    session = SessionFor(m.global)
    m.pendingSeasons = 1
    Ask(EpisodesRequest(session.serverUrl, session.token, m.seriesId, m.seasonId), "onSeasonEpisodes")
end sub

sub onSeasonEpisodes(event as object)
    parsed = Answered(event)
    if m.released then return

    m.pendingSeasons = m.pendingSeasons - 1

    if parsed.ok and type(parsed.json) = "roArray"
        seasonId = SeasonIdOf(parsed.json)
        if not IsBlank(seasonId) then m.bySeason[seasonId] = parsed.json
    end if

    if m.pendingSeasons > 0 then return
    LoadEdgeSeason()
end sub

function SeasonIdOf(episodes as object) as string
    if episodes.Count() = 0 then return ""
    return TextOrBlank(ValueAt(episodes[0], "season_id", ""))
end function

sub LoadEdgeSeason()
    if m.episode = invalid
        ResolveNeighbours()
        return
    end if

    current = ValueAt(m.bySeason, m.seasonId, invalid)
    if type(current) <> "roArray"
        ResolveNeighbours()
        return
    end if

    wanted = SeasonsToLoad(m.episode, m.seasons, current)
    fresh = []
    for each seasonId in wanted
        if not m.bySeason.DoesExist(seasonId) then fresh.Push(seasonId)
    end for

    if fresh.Count() = 0
        ResolveNeighbours()
        return
    end if

    session = SessionFor(m.global)
    m.pendingSeasons = fresh.Count()
    for each seasonId in fresh
        Ask(EpisodesRequest(session.serverUrl, session.token, m.seriesId, seasonId), "onSeasonEpisodes")
    end for
end sub

sub ResolveNeighbours()
    if m.episode = invalid then return

    m.neighbours = EpisodeNeighbours(m.episode, m.seasons, m.bySeason)
    Refresh()
end sub

sub Refresh()
    CoalesceLayout()
end sub

sub PaintScreen()
    if m.episode = invalid then return

    ShowNote("", "loading")
    m.viewport.visible = true
    Layout()

    if not m.published
        m.published = true
        FocusSection(0)
    end if
end sub

sub ShowNote(message as string, kind as string)
    m.note.kind = kind
    m.note.message = message
    m.note.visible = not IsBlank(message)
    if not IsBlank(message) then m.viewport.visible = false
end sub

sub onAction(event as object)
    id = TextOrBlank(event.GetData())

    if id = "play"
        PressPlay()
    else if id = "dismissResume"
        PressDismissResume()
    else if id = "watched"
        ToggleWatched()
    else if id = "previous"
        OpenEpisode(m.neighbours.previous)
    else if id = "following"
        OpenEpisode(m.neighbours.following)
    else if id = "overview"
        ShowOverviewDialog(TitleText(), OverviewText())
    end if
end sub

function OverviewText() as string
    return TextOrBlank(ValueAt(m.episode, "overview", ""))
end function

sub PressPlay()
    target = ResolvePlayTarget(m.available, m.resumable)
    if target = invalid
        AskForVersion()
        return
    end if
    OpenWatch(ValueAt(target, "id", ""))
end sub

sub AskForVersion()
    m.versionChoices = VersionRows(m.available)
    if m.versionChoices.Count() = 0 then return

    labels = []
    for each row in m.versionChoices
        labels.Push(JoinParts([row.label, row.detail]))
    end for

    m.top.choiceRequest = { field: "version", title: Phrase("detail.chooseVersion"), kind: "choice", options: labels, selected: 0 }
end sub

sub onChoice()
    result = m.top.choiceResult
    if HandledCardMenuChoice(result) then return

    if result = invalid or result.cancelled then return
    if result.field <> "version" or m.versionChoices = invalid then return

    choice = m.versionChoices[ClampInt(result.index, 0, m.versionChoices.Count() - 1)]
    OpenWatch(choice.id)
end sub

sub ReloadCardStates()
    LoadState()
end sub

sub PressDismissResume()
    versionId = ResumeTargetId()
    if IsBlank(versionId) then return

    m.dismissedVersion = versionId
    m.resumable.Delete(versionId)
    BuildActions()

    session = SessionFor(m.global)
    Ask(ClearProgressRequest(session.serverUrl, session.token, session.userId, versionId), "onResumeDismissed")
end sub

sub onResumeDismissed(event as object)
    parsed = Answered(event)
    if m.released then return

    if not parsed.ok
        m.resumable[m.dismissedVersion] = 1
        BuildActions()
        RaiseActionFailed()
        return
    end if

    RaiseToast("ok", Phrase("toast.ok.resumeDismissed"))
end sub

sub OpenWatch(versionId as dynamic)
    id = TextOrBlank(versionId)
    if IsBlank(id) then return

    m.top.advanceTarget = {
        versionId: id,
        title: EpisodePlayerTitle(m.episode),
        label: EpisodeLabel(ValueAt(m.episode, "season_number", invalid), m.episode),
        seriesTitle: TextOrBlank(ValueAt(m.episode, "series_title", "")),
        seasonNumber: ValueAt(m.episode, "season_number", invalid),
        percent: ResumePercentFor(m.resumable, id),
        episode: { id: m.id, seriesId: m.seriesId, seasonId: m.seasonId }
    }
    m.top.advance = "WatchScreen"
end sub

sub OpenEpisode(link as dynamic)
    if link = invalid then return

    seriesId = TextOrBlank(ValueAt(link, "seriesId", ""))
    if IsBlank(seriesId) then seriesId = m.seriesId

    m.top.advanceTarget = {
        kind: "episode",
        id: TextOrBlank(ValueAt(link, "id", "")),
        seriesId: seriesId,
        seasonId: TextOrBlank(ValueAt(link, "seasonId", "")),
        title: TextOrBlank(ValueAt(link, "label", ""))
    }
    m.top.advance = "EpisodeScreen"
end sub

sub ToggleWatched()
    m.watched = not m.watched
    BuildActions()

    session = SessionFor(m.global)
    Ask(SetWatchedRequest(session.serverUrl, session.token, session.userId, "episode", m.id, m.watched), "onWatchedDone")
end sub

sub onWatchedDone(event as object)
    parsed = Answered(event)
    if m.released then return

    if not parsed.ok
        m.watched = not m.watched
        BuildActions()
        RaiseActionFailed()
        return
    end if

    if m.watched
        RaiseToast("ok", PhraseWith("toast.ok.markedWatched", { title: TitleText() }))
    else
        RaiseToast("ok", PhraseWith("toast.ok.markedUnwatched", { title: TitleText() }))
    end if
end sub

sub onRelease()
    if not m.top.release then return

    m.released = true
    for each task in m.tasks
        if task <> invalid then task.control = "STOP"
    end for
    m.tasks = []
end sub

sub onFocus()
    if not m.top.hasFocus() then return
    if m.published then FocusSection(m.sectionIndex)
end sub

function EpisodeMenuCard() as dynamic
    if m.episode = invalid then return invalid

    card = EpisodeCard(m.episode)
    card.watched = m.watched
    card.watchlisted = m.watchlisted
    card.favorite = m.favorite
    card.versionId = ResumeTargetId()
    card.progressPercent = ResumePercentFor(m.resumable, card.versionId)
    return card
end function

function onKeyEvent(key as string, press as boolean) as boolean
    if not KeysAreOurs() then return false
    if not press then return false
    if not m.published then return false

    if key = "options" then return OpenedCardMenu(EpisodeMenuCard())

    if HandledStepKey(key, HeroBars()) then return true
    return HandledSectionKey(key)
end function
