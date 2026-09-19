sub init()
    m.heading = m.top.FindNode("heading")
    m.toolbar = m.top.FindNode("toolbar")
    m.grid = m.top.FindNode("grid")
    m.note = m.top.FindNode("note")
    m.more = m.top.FindNode("more")

    m.toolbar.ObserveField("activated", "onToolbar")
    m.grid.ObserveField("selected", "onCardSelected")
    m.grid.ObserveField("focused", "onCardFocused")

    m.kind = CatalogKindFor(m.top.subtype())
    m.query = ReadListQuery(m.kind)
    m.genres = []
    m.libraries = []
    m.libraryCount = -1
    m.cards = []
    m.total = 0
    m.nextOffset = 0
    m.loadingMore = false
    m.generation = 0
    m.started = false

    ShowMore("", "loading")
end sub

sub ShowMore(message as string, kind as string)
    m.more.kind = kind
    m.more.message = message
end sub

sub render()
    theme = m.top.theme
    if theme = invalid or theme.Count() = 0 then return

    sizes = TypeScale()
    space = SpacingScale()
    left = ContentLeft()
    top = ContentTop()

    DrawCrumbs(theme, [{ label: CatalogNoun() }], left, top, ContentWidth())
    top = top + CrumbHeight()

    m.heading.color = theme.text
    m.heading.font = SizedBoldFont(sizes.text2xl)
    m.heading.translation = [left, top]
    RefreshHeading()

    toolbarTop = top + Int(sizes.text2xl * 1.4) + space.s3
    m.toolbar.theme = theme
    m.toolbar.barWidth = ContentWidth()
    m.toolbar.translation = [left, toolbarTop]
    RefreshToolbar()

    gridTop = toolbarTop + Int(sizes.textBase * 1.9) + space.s5

    m.grid.theme = theme
    m.grid.serverUrl = SessionFor(m.global).serverUrl
    m.grid.cardWidth = GridFittedCardWidth()
    m.grid.columns = GridColumns()
    m.grid.captionRows = GridCaptionRows()
    gridHeight = CanvasHeight() - gridTop - space.s6
    m.grid.gridHeight = gridHeight
    m.grid.translation = [left, gridTop]

    PlaceScrollBar(gridHeight)
    RefreshScrollBar()

    m.note.theme = theme
    m.note.fontSize = sizes.textBase
    m.note.noteWidth = ContentWidth()
    m.note.translation = [left, gridTop]

    m.more.theme = theme
    m.more.fontSize = sizes.textSm
    m.more.noteWidth = ContentWidth()
    m.more.translation = [left, CanvasHeight() - space.s6 - space.s4]

    if not m.started
        m.started = true
        Load()
    end if
end sub

function CatalogKindFor(subtype as string) as string
    if subtype = "SeriesScreen" then return "series"
    return "movie"
end function

function CatalogNoun() as string
    if m.kind = "series" then return Phrase("nav.series")
    return Phrase("nav.movies")
end function

function CatalogEmptyPhrase() as string
    if m.kind = "series" then return Phrase("empty.noSeries")
    return Phrase("empty.noMovies")
end function

function CatalogErrorPhrase() as string
    if m.kind = "series" then return Phrase("error.couldNotLoadSeries")
    return Phrase("error.couldNotLoadMovies")
end function

sub RefreshHeading()
    m.heading.text = CountLabel(CatalogNoun(), m.total)
end sub

sub RefreshToolbar()
    m.toolbar.buttons = [
        { icon: "icon-sort", label: Phrase("list.sort") + ": " + OptionLabel(SortOptions(), m.query.sort) },
        { icon: OrderGlyphName(m.query.order), label: Phrase("list.order") + ": " + OptionLabel(OrderOptions(), m.query.order) },
        { icon: "icon-filter", label: Phrase("list.genres") + ": " + FilterLabel(m.genres, GenreFilter()) },
        { icon: "icon-library", label: Phrase("list.library") + ": " + FilterLabel(m.libraries, m.query.library) },
        { icon: "icon-shuffle", label: Phrase("action.random"), outline: true }
    ]
end sub

function GenreFilter() as string
    if m.query.genres.Count() = 0 then return ""
    return m.query.genres[0]
end function

function FilterLabel(options as object, value as dynamic) as string
    if IsBlank(value) then return Phrase("list.all")

    label = OptionLabel(options, value)
    if IsBlank(label) then return AsText(value)
    return label
end function

sub Load()
    LoadFilters()
    Reload()
end sub

sub LoadFilters()
    session = SessionFor(m.global)
    m.genreTask = SendRequest(GenresRequest(session.serverUrl, session.token, GenreKindFor(m.kind)), "onGenres")
    m.libraryTask = SendRequest(LibrariesRequest(session.serverUrl, session.token), "onLibraries")
end sub

sub onGenres(event as object)
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if not parsed.ok or type(parsed.json) <> "roArray" then return

    m.genres = []
    for each genre in parsed.json
        m.genres.Push({ value: TextOrBlank(ValueAt(genre, "name", "")), label: TextOrBlank(ValueAt(genre, "name", "")) })
    end for
    RefreshToolbar()
end sub

sub onLibraries(event as object)
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if not parsed.ok or type(parsed.json) <> "roArray" then return

    m.libraryCount = parsed.json.Count()
    m.libraries = []
    for each library in parsed.json
        m.libraries.Push({ value: TextOrBlank(ValueAt(library, "id", "")), label: TextOrBlank(ValueAt(library, "name", "")) })
    end for
    RefreshToolbar()

    if m.awaitingLibraries = true then CheckLibraryAccess()
end sub

sub Reload()
    m.generation = m.generation + 1
    m.cards = []
    m.total = 0
    m.nextOffset = 0
    m.loadingMore = false
    ShowMore("", "loading")

    ShowNote(Phrase("state.loading"), "loading")
    RequestPage(0, m.generation)
end sub

sub RequestPage(offset as integer, generation as integer)
    session = SessionFor(m.global)

    if m.kind = "series"
        request = SeriesListRequest(session.serverUrl, session.token, m.query, offset, GridPageLimit())
    else
        request = MoviesRequest(session.serverUrl, session.token, m.query, offset, GridPageLimit())
    end if

    m.pageGeneration = generation
    m.pageOffset = offset
    m.pageTask = SendRequest(request, "onPage")
end sub

sub onPage(event as object)
    if m.pageGeneration <> m.generation then return

    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return

    if not parsed.ok
        if m.pageOffset > 0
            m.loadingMore = false
            ShowMore(Phrase("error.couldNotLoadMore"), "error")
            return
        end if
        ShowNote(CatalogErrorPhrase(), "error")
        return
    end if

    items = ValueAt(parsed.json, "items", [])
    m.total = Int(ValueAt(parsed.json, "total", 0))
    RefreshHeading()

    if m.kind = "series"
        page = CardsFrom(items, SeriesCard)
    else
        page = CardsFrom(items, MovieCard)
    end if

    m.nextOffset = NextOffset(m.pageOffset, page.Count())

    start = m.cards.Count()

    if m.pageOffset = 0
        if page.Count() = 0
            CheckLibraryAccess()
            return
        end if
        m.cards.Append(page)
        ShowNote("", "loading")
        m.grid.visible = true
        m.grid.cards = m.cards
        TakeFocus(m.grid)
    else
        m.loadingMore = false
        ShowMore("", "loading")
        m.cards.Append(page)
        m.grid.appendCards = page
    end if

    LoadStates(page, start)
end sub

sub LoadMore()
    if m.loadingMore then return
    if not HasMore(m.cards.Count(), m.total) then return

    m.loadingMore = true
    ShowMore(Phrase("state.loadingMore"), "loading")
    RequestPage(m.nextOffset, m.generation)
end sub

sub onCardFocused()
    RefreshScrollBar()
    if ShouldLoadMore(m.grid.focused, m.cards.Count(), m.total) then LoadMore()
end sub

sub RefreshScrollBar()
    metrics = CardMetrics(GridFittedCardWidth(), m.grid.aspect, GridCaptionRows())
    UpdateGridScrollBar(m.grid, GridColumns(), metrics.height, Int(m.grid.gridHeight))
end sub

sub LoadStates(cards as object, start as integer)
    session = SessionFor(m.global)

    m.states = []
    m.rollups = []
    m.stateTasks = []
    m.statePage = cards
    m.stateStart = start
    m.stateGeneration = m.generation

    leafChunks = ChunkRefs(LeafRefs(cards), MaxBatchSize())
    rollupChunks = ChunkRefs(RollupTargets(cards), MaxBatchSize())
    m.pendingStates = leafChunks.Count() + rollupChunks.Count()
    if m.pendingStates = 0 then return

    for each chunk in leafChunks
        m.stateTasks.Push(SendRequest(StateBatchRequest(session.serverUrl, session.token, session.userId, chunk), "onStates"))
    end for
    for each chunk in rollupChunks
        m.stateTasks.Push(SendRequest(StateRollupRequest(session.serverUrl, session.token, session.userId, chunk), "onRollups"))
    end for
end sub

sub onStates(event as object)
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if parsed.ok and type(parsed.json) = "roArray" then m.states.Append(parsed.json)
    StateArrived()
end sub

sub onRollups(event as object)
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if parsed.ok and type(parsed.json) = "roArray" then m.rollups.Append(parsed.json)
    StateArrived()
end sub

sub StateArrived()
    m.pendingStates = m.pendingStates - 1
    if m.pendingStates > 0 then return
    if m.stateGeneration <> m.generation then return

    ApplyStates(m.statePage, m.states, m.rollups)
    m.grid.stateOffset = m.stateStart
    m.grid.cardStates = CardStateList(m.statePage)
end sub

sub CheckLibraryAccess()
    m.awaitingLibraries = m.libraryCount < 0

    if m.libraryCount = 0
        ShowNote(Phrase("empty.noLibrariesShared"), "empty")
        return
    end if
    ShowNote(CatalogEmptyPhrase(), "empty")
end sub

sub ShowNote(message as string, kind as string)
    m.note.kind = kind
    m.note.message = message
    m.note.visible = not IsBlank(message)
    if not IsBlank(message) then m.grid.visible = false
end sub

sub onToolbar()
    index = m.toolbar.activated

    if index = 0
        RequestChoice("sort", Phrase("list.sort"), "icon-sort", SortOptions(), m.query.sort, false)
    else if index = 1
        RequestChoice("order", Phrase("list.order"), OrderGlyphName(m.query.order), OrderOptions(), m.query.order, false)
    else if index = 2
        RequestChoice("genre", Phrase("list.genres"), "icon-filter", m.genres, GenreFilter(), true)
    else if index = 3
        RequestChoice("library", Phrase("list.library"), "icon-library", m.libraries, m.query.library, true)
    else
        PressRandom()
    end if
end sub

sub PressRandom()
    session = SessionFor(m.global)

    if m.kind = "series"
        request = RandomEpisodeRequest(session.serverUrl, session.token, m.query)
    else
        request = RandomMovieRequest(session.serverUrl, session.token, m.query)
    end if

    m.randomTask = SendRequest(request, "onRandom")
end sub

sub onRandom(event as object)
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return

    versionId = TextOrBlank(ValueAt(parsed.json, "version_id", ""))
    if not parsed.ok or IsBlank(versionId)
        RaiseToast("err", Phrase("error.couldNotPickRandom"))
        return
    end if

    m.top.advanceTarget = { versionId: versionId, resolve: true, percent: 0 }
    m.top.advance = "WatchScreen"
end sub

sub RequestChoice(field as string, title as string, icon as string, options as object, current as dynamic, withAll as boolean)
    choices = []
    if withAll then choices.Push({ value: "", label: Phrase("list.all") })
    for each option in options
        choices.Push(option)
    end for

    if choices.Count() = 0 then return

    labels = []
    selected = 0
    for index = 0 to choices.Count() - 1
        labels.Push(choices[index].label)
        if choices[index].value = AsText(current) then selected = index
    end for

    m.choices = choices
    m.top.choiceRequest = { field: field, title: title, icon: icon, kind: "choice", options: labels, selected: selected }
end sub

sub onChoice()
    result = m.top.choiceResult
    if HandledCardMenuChoice(result) then return
    if result = invalid or result.cancelled then return
    if m.choices = invalid then return

    choice = m.choices[ClampInt(result.index, 0, m.choices.Count() - 1)]

    if result.field = "sort"
        m.query.sort = choice.value
    else if result.field = "order"
        m.query.order = choice.value
    else if result.field = "genre"
        if IsBlank(choice.value)
            m.query.genres = []
        else
            m.query.genres = [choice.value]
        end if
    else if result.field = "library"
        m.query.library = choice.value
    end if

    WriteListQuery(m.kind, m.query)
    RefreshToolbar()
    Reload()
end sub

sub onCardSelected()
    target = m.grid.selected
    if target = invalid or target.Count() = 0 then return

    screen = ScreenForKind(ValueAt(target, "kind", ""))
    if IsBlank(screen) then return

    m.top.advanceTarget = target
    m.top.advance = screen
end sub

sub onFocus()
    if not m.top.hasFocus() then return
    if m.grid.visible
        m.grid.SetFocus(true)
        return
    end if
    m.toolbar.SetFocus(true)
end sub

sub ReloadCardStates()
    LoadStates(m.cards, 0)
end sub

function onKeyEvent(key as string, press as boolean) as boolean
    if not KeysAreOurs() then return false
    if not press then return false

    if key = "options" and m.grid.isInFocusChain()
        return OpenedCardMenu(m.grid.focusedCard)
    end if

    if key = "up" and m.grid.isInFocusChain()
        m.toolbar.SetFocus(true)
        return true
    end if

    if key = "down" and m.toolbar.isInFocusChain() and m.grid.visible
        m.grid.SetFocus(true)
        return true
    end if

    return false
end function
