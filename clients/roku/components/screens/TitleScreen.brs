sub init()
    InitHero()
    InitCrumbs()
    InitLayoutTick()

    m.cast = m.top.FindNode("cast")
    m.seasons = m.top.FindNode("seasons")
    m.seasonsHeading = m.top.FindNode("seasonsHeading")
    m.collection = m.top.FindNode("collection")
    m.note = m.top.FindNode("note")
    m.actions = m.hero.actions
    m.overviewToggle = m.hero.overviewToggle
    m.crewLinks = m.hero.crewLinks

    m.actions.ObserveField("activated", "onAction")
    m.overviewToggle.ObserveField("activated", "onAction")
    m.crewLinks.ObserveField("activated", "onCrewActivated")
    m.cast.ObserveField("selected", "onCastSelected")
    m.cast.ObserveField("itemFocused", "onCastFocused")
    m.seasons.ObserveField("selected", "onSeasonSelected")
    m.collection.ObserveField("selected", "onCollectionSelected")
    m.actorRail = m.top.FindNode("actorRail")
    m.actorRail.ObserveField("selected", "onCollectionSelected")

    InitSections([
        { id: "hero", focus: "actions" },
        { id: "cast" },
        { id: "seasonsSection", focus: "seasons" },
        { id: "collection" },
        { id: "actorRail" }
    ])

    m.kind = ""
    m.id = ""
    m.detail = invalid
    m.ordered = []
    m.available = []
    m.versionChoices = []
    m.pendingWatched = false
    m.nextEpisode = invalid
    m.nextSeasonNumber = invalid
    m.dismissedVersion = ""
    m.resumable = {}
    m.state = { watched: false, watchlisted: false, favorite: false }
    m.rollup = {}
    m.castCards = []
    m.castLoaded = 0
    m.castWanted = 0
    m.castPending = false
    m.crewPeople = []
    m.seasonCards = []
    m.collectionCards = []
    m.collectionName = ""
    m.tasks = []
    m.seasonsFilled = false
    m.actorCards = []
    m.actorName = ""
    m.actorProbeName = ""
    m.actorIndex = 0
    m.actorProbes = 0
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
        m.kind = TextOrBlank(ValueAt(target, "kind", "movie"))
        m.id = TextOrBlank(ValueAt(target, "id", ""))
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

    LayoutCast(theme, width)
    LayoutSeasons(theme, width)
    LayoutCollection(theme, width)
    LayoutActorRail(theme, width)

    PlaceScrollBar(height)
    LayoutSections(width, height)
end sub

function CrumbList() as object
    section = Phrase("nav.movies")
    screen = "MoviesScreen"
    if m.kind = "series"
        section = Phrase("nav.series")
        screen = "SeriesScreen"
    end if

    return [{ label: section, screen: screen, target: invalid }, { label: TitleText() }]
end function

function HeroContent() as object
    target = m.top.target

    return {
        aspect: PosterAspect(),
        imageUri: HeroImageUri(),
        placeholderUri: CardPlaceholderUri(target),
        heading: TitleText(),
        facts: FactsText(HeroFacts()),
        crewButtons: CrewBar(),
        genreChips: GenreChipList(ValueAt(m.detail, "genres", invalid)),
        ratingChips: RatingChipList(ValueAt(m.detail, "ratings", invalid)),
        overview: OverviewText()
    }
end function

function OverviewText() as string
    return TextOrBlank(ValueAt(m.detail, "overview", ""))
end function

function HeroImageUri() as string
    if m.detail <> invalid
        card = { aspect: PosterAspect(), artwork: ValueAt(m.detail, "artwork", invalid) }
        url = CardImageUrl(SessionFor(m.global).serverUrl, card, HeroArtWidth(PosterAspect()))
        if not IsBlank(url) then return url
    end if
    return TextOrBlank(ValueAt(m.top.target, "imageUri", ""))
end function

function TitleText() as string
    title = TextOrBlank(ValueAt(m.detail, "title", ""))
    if not IsBlank(title) then return title
    return TextOrBlank(ValueAt(m.top.target, "title", ""))
end function

function HeroFacts() as object
    if m.kind = "series" then return SeriesFacts(m.detail, m.rollup)
    return MovieFacts(m.detail, BestVersion(m.ordered))
end function

function CrewBar() as object
    groups = CrewGroups(ValueAt(m.detail, "credits", invalid), CrewNameLimit())
    m.crewPeople = CrewPeople(groups)
    return CrewButtons(groups)
end function

sub LayoutCast(theme as object, width as integer)
    if m.castCards.Count() = 0
        HideSection("cast")
        return
    end if

    m.cast.theme = theme
    m.cast.serverUrl = SessionFor(m.global).serverUrl
    m.cast.railWidth = width
    m.cast.cardWidth = CastCardWidth()
    m.cast.heading = Phrase("heading.cast")
    m.cast.cards = m.castCards

    ShowSection("cast", Int(m.cast.railHeight))
end sub

sub LayoutSeasons(theme as object, width as integer)
    if m.kind <> "series" or m.detail = invalid
        HideSection("seasonsSection")
        return
    end if

    space = SpacingScale()
    headingHeight = SectionHeadingHeight()
    DrawSectionHeading(m.seasonsHeading, theme, CountLabel(Phrase("heading.seasons"), m.seasonCards.Count()), width)

    if m.seasonCards.Count() = 0
        m.seasons.visible = false
        ShowSection("seasonsSection", headingHeight + Int(TypeScale().textBase * 1.6), false)
        return
    end if

    columns = ColumnsThatFit(width, GridCardWidth(), space.s4)
    cardWidth = FittedColumnWidth(width, columns, space.s4)
    metrics = CardMetrics(cardWidth, PosterAspect(), GridCaptionRows())
    gridHeight = GridContentHeight(m.seasonCards.Count(), columns, metrics.height, space.s5)

    m.seasons.visible = true
    m.seasons.theme = theme
    m.seasons.serverUrl = SessionFor(m.global).serverUrl
    m.seasons.cardWidth = cardWidth
    m.seasons.columns = columns
    m.seasons.captionRows = GridCaptionRows()
    m.seasons.gridHeight = gridHeight
    m.seasons.translation = [0, headingHeight]

    if not m.seasonsFilled
        m.seasonsFilled = true
        m.seasons.cards = m.seasonCards
    end if

    ShowSection("seasonsSection", headingHeight + gridHeight)
end sub

sub LayoutCollection(theme as object, width as integer)
    if m.collectionCards.Count() = 0
        HideSection("collection")
        return
    end if

    m.collection.theme = theme
    m.collection.serverUrl = SessionFor(m.global).serverUrl
    m.collection.railWidth = width
    m.collection.cardWidth = RailCardWidth()
    m.collection.heading = PhraseWith("heading.inCollection", { name: m.collectionName })
    m.collection.cards = m.collectionCards

    ShowSection("collection", Int(m.collection.railHeight))
end sub

sub BuildActions()
    theme = m.top.theme
    buttons = []

    if m.kind = "series"
        if m.seasonCards.Count() > 0
            buttons.Push({ id: "play", label: Phrase("action.nextEpisode"), icon: "icon-play", style: "primary" })
        end if
        buttons.Push(WatchedButton())
        if m.seasonCards.Count() > 0
            buttons.Push({ id: "random", label: Phrase("action.randomInSeries"), icon: "icon-shuffle" })
        end if
    else
        if m.available.Count() > 0
            buttons.Push(PlayButton())
            if ResumeTargetId() <> "" then buttons.Push(DismissButton())
        end if
        buttons.Push(WatchedButton())
        buttons.Push(WatchlistButton())
        buttons.Push(FavoriteButton())
    end if

    m.actions.theme = theme
    m.actions.barWidth = CanvasWidth() - ContentLeft() - SpacingScale().s6
    if buttons.Count() > 0 then m.actions.focusIndex = ClampInt(m.actions.focusIndex, 0, buttons.Count() - 1)
    m.actions.buttons = buttons
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

function WatchedNow() as boolean
    if m.kind = "series" then return ValueAt(m.rollup, "watched", false) = true
    return m.state.watched
end function

sub SetWatchedLocal(watched as boolean)
    if m.kind = "series"
        m.rollup.watched = watched
        return
    end if
    m.state.watched = watched
end sub

function WatchedButton() as object
    return { id: "watched", label: Phrase("action.watchedLabel"), icon: "icon-check", iconOn: "icon-check", pressed: WatchedNow(), collapsible: true }
end function

function WatchlistButton() as object
    return { id: "watchlist", label: Phrase("action.watchlistLabel"), icon: "icon-bookmark", iconOn: "icon-bookmark-on", pressed: m.state.watchlisted, collapsible: true }
end function

function FavoriteButton() as object
    return { id: "favorite", label: Phrase("action.favoriteLabel"), icon: "icon-heart", iconOn: "icon-heart-on", pressed: m.state.favorite, collapsible: true }
end function

sub Load()
    session = SessionFor(m.global)
    ShowNote(Phrase("state.loading"), "loading")

    if m.kind = "series"
        Ask(SeriesDetailRequest(session.serverUrl, session.token, m.id), "onDetail")
        Ask(SeasonsRequest(session.serverUrl, session.token, m.id), "onSeasons")
        return
    end if

    Ask(MovieDetailRequest(session.serverUrl, session.token, m.id), "onDetail")
    Ask(MovieVersionsRequest(session.serverUrl, session.token, m.id), "onVersions")
    Ask(ContinueRequest(session.serverUrl, session.token, session.userId), "onContinue")
    Ask(MovieCollectionsRequest(session.serverUrl, session.token, m.id), "onCollections")
    Ask(StateBatchRequest(session.serverUrl, session.token, session.userId, [{ "type": "movie", "id": m.id }]), "onState")
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

sub onDetail(event as object)
    parsed = Answered(event)
    if m.released then return

    if not parsed.ok
        ShowNote(Phrase("error.couldNotLoadTitle"), "error")
        return
    end if

    m.detail = parsed.json
    PublishBackdrop(ValueAt(m.detail, "artwork", invalid))
    LoadCast()
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
        m.state = {
            watched: ValueAt(entry, "watched", false) = true,
            watchlisted: ValueAt(entry, "watchlisted", false) = true,
            favorite: ValueAt(entry, "favorite", false) = true
        }
    end if
    Refresh()
end sub

sub onSeasons(event as object)
    parsed = Answered(event)
    if m.released then return

    if parsed.ok
        m.seasonCards = CardsFrom(OrderedSeasons(parsed.json), SeasonCard)
        m.seasonsFilled = false
    end if
    LoadRollups()
    Refresh()
end sub

sub LoadRollups()
    session = SessionFor(m.global)

    targets = [{ "type": "series", "id": m.id }]
    for each target in RollupTargets(m.seasonCards)
        targets.Push(target)
    end for

    Ask(StateRollupRequest(session.serverUrl, session.token, session.userId, targets), "onRollups")
end sub

sub onRollups(event as object)
    parsed = Answered(event)
    if m.released then return

    if parsed.ok and type(parsed.json) = "roArray"
        lookup = StateLookup(parsed.json, "target")
        key = TitleKey("series", m.id)
        if lookup.DoesExist(key) then m.rollup = lookup[key]
        ApplyStates(m.seasonCards, [], parsed.json)
        m.seasons.cardStates = CardStateList(m.seasonCards)
    end if
    Refresh()
end sub

sub onCollections(event as object)
    parsed = Answered(event)
    if m.released then return

    if not parsed.ok or type(parsed.json) <> "roArray" then return

    for each collection in parsed.json
        cards = []
        for each movie in CardsFrom(ValueAt(collection, "items", invalid), MovieCard)
            if cards.Count() < GridPageLimit() and movie.id <> m.id then cards.Push(movie)
        end for

        if cards.Count() > 0
            m.collectionCards = cards
            m.collectionName = TextOrBlank(ValueAt(collection, "name", ""))
            LoadCollectionStates()
            Refresh()
            return
        end if
    end for
end sub

sub LoadCollectionStates()
    session = SessionFor(m.global)
    refs = LeafRefs(m.collectionCards)
    if refs.Count() = 0 then return

    Ask(StateBatchRequest(session.serverUrl, session.token, session.userId, refs), "onCollectionStates")
end sub

sub onCollectionStates(event as object)
    parsed = Answered(event)
    if m.released then return

    if parsed.ok and type(parsed.json) = "roArray"
        ApplyStates(m.collectionCards, parsed.json, [])
    end if
    Refresh()
end sub

sub LoadCast()
    m.castCards = CardsFrom(BilledActors(ValueAt(m.detail, "credits", invalid)), CastCard)
    m.castLoaded = 0
    m.castWanted = 0
    m.castPending = false
    if m.castCards.Count() = 0 then return

    LoadCastArtwork()
end sub

sub LoadCastArtwork()
    if m.castPending then return
    if m.castLoaded >= m.castCards.Count() then return

    last = ClampInt(m.castLoaded + CastArtworkWindow() - 1, 0, m.castCards.Count() - 1)

    window = []
    for index = m.castLoaded to last
        window.Push(m.castCards[index])
    end for

    m.castPending = true
    m.castWanted = last + 1

    session = SessionFor(m.global)
    Ask(PeopleBatchRequest(session.serverUrl, session.token, PersonIds(window)), "onPeople")
end sub

sub onCastFocused(event as object)
    if not CastArtworkWanted(m.castLoaded, Int(event.GetData()), m.castCards.Count()) then return

    LoadCastArtwork()
end sub

sub onPeople(event as object)
    parsed = Answered(event)
    if m.released then return

    m.castPending = false
    opening = m.castLoaded = 0

    if parsed.ok
        MergeArtwork(m.castCards, ArtworkById(parsed.json))
        m.castLoaded = m.castWanted
    end if

    Refresh()
    if opening then ProbeActor()
end sub

sub ProbeActor()
    if m.released then return
    if m.actorCards.Count() > 0 then return
    if m.actorProbes >= MaxActorProbes() then return
    if m.actorIndex >= m.castCards.Count() then return

    actor = m.castCards[m.actorIndex]
    m.actorIndex = m.actorIndex + 1
    m.actorProbes = m.actorProbes + 1
    m.actorProbeName = TextOrBlank(ValueAt(actor, "title", ""))

    session = SessionFor(m.global)
    Ask(PersonRequest(session.serverUrl, session.token, ValueAt(actor, "id", "")), "onActorTitles")
end sub

sub onActorTitles(event as object)
    parsed = Answered(event)
    if m.released then return

    if parsed.ok
        cards = CardsFrom(ValueAt(parsed.json, "filmography", invalid), FilmographyCard)
        fresh = FreshActorTitles(cards, ShownTitleKeys([[{ kind: m.kind, id: m.id }], m.seasonCards, m.collectionCards]))

        if fresh.Count() >= MinActorTitles()
            m.actorCards = fresh
            m.actorName = m.actorProbeName
            Refresh()
            return
        end if
    end if

    ProbeActor()
end sub

sub LayoutActorRail(theme as object, width as integer)
    if m.actorCards.Count() = 0
        HideSection("actorRail")
        return
    end if

    m.actorRail.theme = theme
    m.actorRail.serverUrl = SessionFor(m.global).serverUrl
    m.actorRail.railWidth = width
    m.actorRail.cardWidth = RailCardWidth()
    m.actorRail.heading = PhraseWith("heading.moreWith", { name: m.actorName })
    m.actorRail.cards = m.actorCards

    ShowSection("actorRail", Int(m.actorRail.railHeight))
end sub

sub Refresh()
    CoalesceLayout()
end sub

sub PaintScreen()
    if m.detail = invalid then return

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
    else if id = "watchlist"
        ToggleWatchlist()
    else if id = "favorite"
        ToggleFavorite()
    else if id = "random"
        PressRandom()
    else if id = "overview"
        ShowOverviewDialog(TitleText(), OverviewText())
    end if
end sub

sub PressPlay()
    if m.kind = "series"
        ResolveNextEpisode()
        return
    end if

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

    if result <> invalid and TextOrBlank(ValueAt(result, "field", "")) = "watched"
        if WatchedConfirmAccepted(result) then ApplyWatched(m.pendingWatched)
        return
    end if

    if result = invalid or result.cancelled then return
    if result.field <> "version" or m.versionChoices = invalid then return

    choice = m.versionChoices[ClampInt(result.index, 0, m.versionChoices.Count() - 1)]
    OpenWatch(choice.id)
end sub

sub ReloadCardStates()
    if m.kind = "series"
        LoadRollups()
        return
    end if
    LoadCollectionStates()
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

    m.top.advanceTarget = { versionId: id, title: TitleText(), year: ValueAt(m.detail, "year", invalid), percent: ResumePercentFor(m.resumable, id) }
    m.top.advance = "WatchScreen"
end sub

sub ResolveNextEpisode()
    watched = {}
    for each card in m.seasonCards
        if card.watched then watched[card.id] = true
    end for

    season = NextSeason(m.seasonCards, watched)
    if season = invalid
        RaiseToast("err", Phrase("empty.noEpisodes"))
        return
    end if

    m.nextSeasonId = TextOrBlank(ValueAt(season, "id", ""))
    m.nextSeasonNumber = ValueAt(season, "number", invalid)
    session = SessionFor(m.global)
    Ask(EpisodesRequest(session.serverUrl, session.token, m.id, m.nextSeasonId), "onNextEpisodes")
end sub

sub onNextEpisodes(event as object)
    parsed = Answered(event)
    if m.released then return

    if not parsed.ok or type(parsed.json) <> "roArray" or parsed.json.Count() = 0
        RaiseToast("err", Phrase("empty.noEpisodes"))
        return
    end if

    m.nextEpisodes = OrderedEpisodes(parsed.json)

    refs = []
    for each episode in m.nextEpisodes
        refs.Push({ "type": "episode", "id": TextOrBlank(ValueAt(episode, "id", "")) })
    end for

    session = SessionFor(m.global)
    Ask(StateBatchRequest(session.serverUrl, session.token, session.userId, refs), "onNextStates")
end sub

sub onNextStates(event as object)
    parsed = Answered(event)
    if m.released then return

    watched = {}
    if parsed.ok then watched = WatchedIdSet(parsed.json, "title")

    episode = FirstUnwatched(m.nextEpisodes, watched)
    if episode = invalid
        RaiseToast("err", Phrase("empty.noEpisodes"))
        return
    end if

    m.nextEpisode = episode
    session = SessionFor(m.global)
    Ask(EpisodeVersionsRequest(session.serverUrl, session.token, m.id, m.nextSeasonId, TextOrBlank(ValueAt(episode, "id", ""))), "onNextEpisodeVersions")
end sub

sub onNextEpisodeVersions(event as object)
    parsed = Answered(event)
    if m.released then return

    target = invalid
    if parsed.ok then target = EpisodePlayTarget(m.nextEpisode, m.id, m.nextSeasonId, parsed.json, TitleText(), m.nextSeasonNumber)

    if target = invalid
        RaiseToast("err", Phrase("empty.nothingToPlay"))
        return
    end if

    m.top.advanceTarget = target
    m.top.advance = "WatchScreen"
end sub

sub PressRandom()
    session = SessionFor(m.global)
    Ask(RandomInSeriesRequest(session.serverUrl, session.token, m.id), "onRandom")
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
    if NeedsWatchedConfirm(m.kind)
        m.pendingWatched = wanted
        m.top.choiceRequest = {
            field: "watched",
            title: Phrase("action.watchedLabel"),
            message: WatchedConfirmTitle(m.kind, wanted),
            options: WatchedConfirmOptions(wanted),
            selected: 0
        }
        return
    end if

    ApplyWatched(wanted)
end sub

sub ApplyWatched(wanted as boolean)
    SetWatchedLocal(wanted)
    BuildActions()

    session = SessionFor(m.global)
    Ask(SetWatchedRequest(session.serverUrl, session.token, session.userId, m.kind, m.id, wanted), "onWatchedDone")
end sub

sub onWatchedDone(event as object)
    parsed = Answered(event)
    if m.released then return

    if not parsed.ok
        SetWatchedLocal(not WatchedNow())
        BuildActions()
        RaiseActionFailed()
        return
    end if

    if WatchedNow()
        RaiseToast("ok", PhraseWith("toast.ok.markedWatched", { title: TitleText() }))
    else
        RaiseToast("ok", PhraseWith("toast.ok.markedUnwatched", { title: TitleText() }))
    end if
end sub

sub ToggleWatchlist()
    m.state.watchlisted = not m.state.watchlisted
    BuildActions()

    session = SessionFor(m.global)
    if m.state.watchlisted
        Ask(WatchlistAddRequest(session.serverUrl, session.token, session.userId, m.kind, m.id), "onWatchlistDone")
        return
    end if
    Ask(WatchlistRemoveRequest(session.serverUrl, session.token, session.userId, m.id), "onWatchlistDone")
end sub

sub onWatchlistDone(event as object)
    parsed = Answered(event)
    if m.released then return

    if not parsed.ok
        m.state.watchlisted = not m.state.watchlisted
        BuildActions()
        RaiseActionFailed()
        return
    end if

    if m.state.watchlisted
        RaiseToast("ok", PhraseWith("toast.ok.watchlistAdded", { title: TitleText() }))
    else
        RaiseToast("ok", PhraseWith("toast.ok.watchlistRemoved", { title: TitleText() }))
    end if
end sub

sub ToggleFavorite()
    m.state.favorite = not m.state.favorite
    BuildActions()

    session = SessionFor(m.global)
    if m.state.favorite
        Ask(FavoriteAddRequest(session.serverUrl, session.token, session.userId, m.kind, m.id), "onFavoriteDone")
        return
    end if
    Ask(FavoriteRemoveRequest(session.serverUrl, session.token, session.userId, m.id), "onFavoriteDone")
end sub

sub onFavoriteDone(event as object)
    parsed = Answered(event)
    if m.released then return

    if not parsed.ok
        m.state.favorite = not m.state.favorite
        BuildActions()
        RaiseActionFailed()
        return
    end if

    if m.state.favorite
        RaiseToast("ok", PhraseWith("toast.ok.favoriteAdded", { title: TitleText() }))
    else
        RaiseToast("ok", PhraseWith("toast.ok.favoriteRemoved", { title: TitleText() }))
    end if
end sub

sub onCastSelected(event as object)
    Advance(event.GetData())
end sub

sub onCrewActivated(event as object)
    id = TextOrBlank(event.GetData())
    if Left(id, 5) <> "crew:" then return
    if type(m.crewPeople) <> "roArray" then return

    index = Int(Val(Mid(id, 6)))
    if index < 0 or index >= m.crewPeople.Count() then return

    person = m.crewPeople[index]
    Advance({ kind: "person", id: person.id, title: person.name })
end sub

sub onSeasonSelected(event as object)
    Advance(event.GetData())
end sub

sub onCollectionSelected(event as object)
    Advance(event.GetData())
end sub

sub Advance(target as dynamic)
    if target = invalid or target.Count() = 0 then return

    screen = ScreenForKind(ValueAt(target, "kind", ""))
    if IsBlank(screen) then return

    if ValueAt(target, "kind", "") = "season" and IsBlank(TextOrBlank(ValueAt(target, "seriesId", "")))
        target.seriesId = m.id
    end if

    m.top.advanceTarget = target
    m.top.advance = screen
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

function onKeyEvent(key as string, press as boolean) as boolean
    if not KeysAreOurs() then return false
    if not press then return false
    if not m.published then return false

    if key = "options"
        if m.seasons.isInFocusChain() then return OpenedCardMenu(m.seasons.focusedCard)
        if m.collection.isInFocusChain() then return OpenedCardMenu(m.collection.focusedCard)
        if m.actorRail.isInFocusChain() then return OpenedCardMenu(m.actorRail.focusedCard)
        return false
    end if

    if HandledStepKey(key, HeroBars()) then return true

    return HandledSectionKey(key)
end function
