sub init()
    InitHero()
    InitCrumbs()
    InitLayoutTick()

    m.episodes = m.top.FindNode("episodes")
    m.episodesHeading = m.top.FindNode("episodesHeading")
    m.episodesNote = m.top.FindNode("episodesNote")
    m.note = m.top.FindNode("note")
    m.actions = m.hero.actions
    m.navActions = m.hero.navActions
    m.overviewToggle = m.hero.overviewToggle

    m.actions.ObserveField("activated", "onAction")
    m.navActions.ObserveField("activated", "onAction")
    m.overviewToggle.ObserveField("activated", "onAction")
    m.episodes.ObserveField("selected", "onEpisodeSelected")

    InitSections([{ id: "hero", focus: "actions" }, { id: "episodesSection", focus: "episodes" }])

    m.id = ""
    m.seriesId = ""
    m.season = invalid
    m.rollup = {}
    m.episodeList = []
    m.nextEpisode = invalid
    m.episodeCards = []
    m.neighbours = { previous: invalid, following: invalid }
    m.watchedEpisodes = {}
    m.pendingWatched = false
    m.tasks = []
    m.filled = false
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
    LayoutEpisodes(theme, width)
    PlaceScrollBar(height)
    LayoutSections(width, height)
end sub

function CrumbList() as object
    series = TextOrBlank(ValueAt(m.season, "series_title", ""))
    if IsBlank(series) then series = Phrase("nav.series")

    return [
        { label: Phrase("nav.series"), screen: "SeriesScreen", target: invalid },
        SeriesCrumb(series),
        { label: TitleText() }
    ]
end function

function SeriesCrumb(series as string) as object
    if IsBlank(m.seriesId) then return { label: series }
    return { label: series, screen: "TitleScreen", target: { kind: "series", id: m.seriesId, title: series } }
end function

function HeroContent() as object
    pairs = SeasonFacts(m.season, m.rollup)

    return {
        aspect: PosterAspect(),
        imageUri: HeroImageUri(),
        placeholderUri: PosterPlaceholder(),
        heading: TitleText(),
        facts: FactsText(pairs),
        overview: OverviewText()
    }
end function

function HeroImageUri() as string
    if m.season <> invalid
        artwork = FirstArtwork([ValueAt(m.season, "artwork", invalid), ValueAt(m.season, "series_artwork", invalid)])
        url = CardImageUrl(SessionFor(m.global).serverUrl, { aspect: PosterAspect(), artwork: artwork }, HeroArtWidth(PosterAspect()))
        if not IsBlank(url) then return url
    end if
    return TextOrBlank(ValueAt(m.top.target, "imageUri", ""))
end function

function TitleText() as string
    if m.season <> invalid then return SeasonLabel(m.season)
    return TextOrBlank(ValueAt(m.top.target, "title", ""))
end function

sub LayoutEpisodes(theme as object, width as integer)
    if m.season = invalid
        HideSection("episodesSection")
        return
    end if

    space = SpacingScale()
    headingHeight = SectionHeadingHeight()
    DrawSectionHeading(m.episodesHeading, theme, CountLabel(Phrase("heading.episodes"), m.episodeCards.Count()), width)

    if m.episodeCards.Count() = 0
        m.episodes.visible = false
        m.episodesNote.visible = true
        m.episodesNote.theme = theme
        m.episodesNote.fontSize = TypeScale().textBase
        m.episodesNote.noteWidth = width
        m.episodesNote.message = Phrase("empty.noEpisodes")
        m.episodesNote.translation = [0, headingHeight]

        ShowSection("episodesSection", headingHeight + Int(TypeScale().textBase * 1.6), false)
        return
    end if

    columns = ColumnsThatFit(width, LandscapeCardWidth(), space.s4)
    cardWidth = FittedColumnWidth(width, columns, space.s4)
    metrics = CardMetrics(cardWidth, LandscapeAspect(), GridCaptionRows())
    gridHeight = GridContentHeight(m.episodeCards.Count(), columns, metrics.height, space.s5)

    m.episodesNote.visible = false
    m.episodes.visible = true
    m.episodes.theme = theme
    m.episodes.serverUrl = SessionFor(m.global).serverUrl
    m.episodes.cardWidth = cardWidth
    m.episodes.columns = columns
    m.episodes.captionRows = GridCaptionRows()
    m.episodes.gridHeight = gridHeight
    m.episodes.translation = [0, headingHeight]

    if not m.filled
        m.filled = true
        m.episodes.cards = m.episodeCards
    end if

    ShowSection("episodesSection", headingHeight + gridHeight)
end sub

sub BuildActions()
    buttons = []

    if m.episodeCards.Count() > 0
        buttons.Push({ id: "play", label: Phrase("action.nextEpisode"), icon: "icon-play", style: "primary" })
    end if

    buttons.Push({ id: "watched", label: Phrase("action.watchedLabel"), icon: "icon-check", iconOn: "icon-check", pressed: WatchedNow(), collapsible: true })

    if m.episodeCards.Count() > 0
        buttons.Push({ id: "random", label: Phrase("action.randomInSeason"), icon: "icon-shuffle" })
    end if

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

function WatchedNow() as boolean
    return ValueAt(m.rollup, "watched", false) = true
end function

sub Load()
    session = SessionFor(m.global)
    ShowNote(Phrase("state.loading"), "loading")

    Ask(SeasonRequest(session.serverUrl, session.token, m.seriesId, m.id), "onSeason")
    Ask(EpisodesRequest(session.serverUrl, session.token, m.seriesId, m.id), "onEpisodes")
    Ask(SeasonsRequest(session.serverUrl, session.token, m.seriesId), "onSeasons")
    Ask(StateRollupRequest(session.serverUrl, session.token, session.userId, [{ "type": "season", "id": m.id }]), "onRollups")
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

sub onSeason(event as object)
    parsed = Answered(event)
    if m.released then return

    if not parsed.ok
        ShowNote(Phrase("error.couldNotLoadSeason"), "error")
        return
    end if

    m.season = parsed.json
    if IsBlank(m.seriesId) then m.seriesId = TextOrBlank(ValueAt(m.season, "series_id", ""))

    PublishBackdropFrom(SeasonBackdropCandidates(m.season))
    Refresh()
end sub

sub onEpisodes(event as object)
    parsed = Answered(event)
    if m.released then return

    if parsed.ok and type(parsed.json) = "roArray"
        m.episodeList = OrderedEpisodes(parsed.json)
        m.episodeCards = CardsFrom(m.episodeList, EpisodeCard)
        m.filled = false
        LoadEpisodeStates()
    end if
    Refresh()
end sub

sub LoadEpisodeStates()
    refs = LeafRefs(m.episodeCards)
    if refs.Count() = 0 then return

    session = SessionFor(m.global)
    Ask(StateBatchRequest(session.serverUrl, session.token, session.userId, refs), "onEpisodeStates")
end sub

sub onEpisodeStates(event as object)
    parsed = Answered(event)
    if m.released then return

    if parsed.ok and type(parsed.json) = "roArray"
        m.watchedEpisodes = WatchedIdSet(parsed.json, "title")
        ApplyStates(m.episodeCards, parsed.json, [])
        m.episodes.cardStates = CardStateList(m.episodeCards)
    end if
    Refresh()
end sub

sub onSeasons(event as object)
    parsed = Answered(event)
    if m.released then return

    if parsed.ok then m.neighbours = SeasonNeighbours(m.id, parsed.json)
    Refresh()
end sub

sub onRollups(event as object)
    parsed = Answered(event)
    if m.released then return

    if parsed.ok and type(parsed.json) = "roArray" and parsed.json.Count() > 0
        m.rollup = parsed.json[0]
    end if
    Refresh()
end sub

sub Refresh()
    CoalesceLayout()
end sub

sub PaintScreen()
    if m.season = invalid then return

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
    else if id = "watched"
        ToggleWatched()
    else if id = "random"
        PressRandom()
    else if id = "previous"
        OpenSeason(m.neighbours.previous)
    else if id = "following"
        OpenSeason(m.neighbours.following)
    else if id = "overview"
        ShowOverviewDialog(TitleText(), OverviewText())
    end if
end sub

function OverviewText() as string
    return TextOrBlank(ValueAt(m.season, "overview", ""))
end function

sub PressPlay()
    episode = FirstUnwatched(m.episodeList, m.watchedEpisodes)
    if episode = invalid
        RaiseToast("err", Phrase("empty.noEpisodes"))
        return
    end if

    m.nextEpisode = episode
    session = SessionFor(m.global)
    Ask(EpisodeVersionsRequest(session.serverUrl, session.token, m.seriesId, m.id, TextOrBlank(ValueAt(episode, "id", ""))), "onNextEpisodeVersions")
end sub

sub onNextEpisodeVersions(event as object)
    parsed = Answered(event)
    if m.released then return

    target = invalid
    if parsed.ok then target = EpisodePlayTarget(m.nextEpisode, m.seriesId, m.id, parsed.json, ValueAt(m.season, "series_title", ""), ValueAt(m.season, "number", invalid))

    if target = invalid
        RaiseToast("err", Phrase("empty.nothingToPlay"))
        return
    end if

    m.top.advanceTarget = target
    m.top.advance = "WatchScreen"
end sub

sub OpenSeason(link as dynamic)
    if link = invalid then return

    seriesId = TextOrBlank(ValueAt(link, "seriesId", ""))
    if IsBlank(seriesId) then seriesId = m.seriesId

    m.top.advanceTarget = {
        kind: "season",
        id: TextOrBlank(ValueAt(link, "id", "")),
        seriesId: seriesId,
        title: TextOrBlank(ValueAt(link, "label", ""))
    }
    m.top.advance = "SeasonScreen"
end sub

sub PressRandom()
    session = SessionFor(m.global)
    Ask(RandomInSeasonRequest(session.serverUrl, session.token, m.seriesId, m.id), "onRandom")
end sub

sub onRandom(event as object)
    parsed = Answered(event)
    if m.released then return

    versionId = TextOrBlank(ValueAt(parsed.json, "version_id", ""))
    if not parsed.ok or IsBlank(versionId)
        RaiseToast("err", Phrase("error.couldNotPickRandom"))
        return
    end if

    m.top.advanceTarget = { versionId: versionId, resolve: true, percent: 0 }
    m.top.advance = "WatchScreen"
end sub

sub ToggleWatched()
    wanted = not WatchedNow()
    m.pendingWatched = wanted
    m.top.choiceRequest = {
        field: "watched",
        title: Phrase("action.watchedLabel"),
        message: WatchedConfirmTitle("season", wanted),
        options: WatchedConfirmOptions(wanted),
        selected: 0
    }
end sub

sub ApplyWatched(wanted as boolean)
    m.rollup.watched = wanted
    BuildActions()

    session = SessionFor(m.global)
    Ask(SetWatchedRequest(session.serverUrl, session.token, session.userId, "season", m.id, wanted), "onWatchedDone")
end sub

sub onWatchedDone(event as object)
    parsed = Answered(event)
    if m.released then return

    if not parsed.ok
        m.rollup.watched = not WatchedNow()
        BuildActions()
        RaiseActionFailed()
        return
    end if

    if WatchedNow()
        RaiseToast("ok", PhraseWith("toast.ok.markedWatched", { title: TitleText() }))
    else
        RaiseToast("ok", PhraseWith("toast.ok.markedUnwatched", { title: TitleText() }))
    end if
    ReloadStates()
end sub

sub ReloadStates()
    session = SessionFor(m.global)
    Ask(StateRollupRequest(session.serverUrl, session.token, session.userId, [{ "type": "season", "id": m.id }]), "onRollups")
    LoadEpisodeStates()
end sub

sub onEpisodeSelected(event as object)
    target = event.GetData()
    if target = invalid or target.Count() = 0 then return

    if IsBlank(TextOrBlank(ValueAt(target, "seriesId", ""))) then target.seriesId = m.seriesId
    if IsBlank(TextOrBlank(ValueAt(target, "seasonId", ""))) then target.seasonId = m.id

    m.top.advanceTarget = target
    m.top.advance = "EpisodeScreen"
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

sub onChoice()
    result = m.top.choiceResult
    if HandledCardMenuChoice(result) then return

    if result <> invalid and TextOrBlank(ValueAt(result, "field", "")) = "watched"
        if WatchedConfirmAccepted(result) then ApplyWatched(m.pendingWatched)
    end if
end sub

sub ReloadCardStates()
    ReloadStates()
end sub

function onKeyEvent(key as string, press as boolean) as boolean
    if not KeysAreOurs() then return false
    if not press then return false
    if not m.published then return false

    if key = "options" and m.episodes.isInFocusChain()
        return OpenedCardMenu(m.episodes.focusedCard)
    end if

    if HandledStepKey(key, HeroBars()) then return true
    return HandledSectionKey(key)
end function
