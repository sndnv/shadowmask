sub init()
    m.heading = m.top.FindNode("heading")
    m.keyboard = m.top.FindNode("keyboard")
    m.actions = m.top.FindNode("actions")
    m.grid = m.top.FindNode("grid")
    m.note = m.top.FindNode("note")
    m.searchTick = m.top.FindNode("searchTick")

    m.keyboard.ObserveField("text", "onTyped")
    m.actions.ObserveField("activated", "onAction")
    m.grid.ObserveField("selected", "onCardSelected")
    m.grid.ObserveField("focused", "onCardFocused")

    m.searchTick.duration = SearchDebounceSeconds()
    m.searchTick.repeat = false
    m.searchTick.ObserveField("fire", "onSearchTick")

    m.term = ""
    m.cards = []
    m.total = 0
    m.nextOffset = 0
    m.loadingMore = false
    m.generation = 0
end sub

function ResultColumns(width as integer) as integer
    return ColumnsThatFit(width, GridCardWidth(), SpacingScale().s4)
end function

function ResultCardWidth(width as integer) as integer
    return FittedColumnWidth(width, ResultColumns(width), SpacingScale().s4)
end function

sub render()
    theme = m.top.theme
    if theme = invalid or theme.Count() = 0 then return

    sizes = TypeScale()
    space = SpacingScale()
    left = ContentLeft()
    top = ContentTop()

    m.heading.color = theme.text
    m.heading.font = SizedBoldFont(sizes.text2xl)
    m.heading.width = ContentWidth()
    m.heading.maxLines = 1
    m.heading.ellipsisText = "…"
    DrawCrumbs(theme, [{ label: Phrase("nav.search") }], left, top, ContentWidth())
    top = top + CrumbHeight()

    m.heading.translation = [left, top]
    RefreshHeading()

    bodyTop = top + Int(sizes.text2xl * 1.4) + space.s3

    m.keyboard.translation = [left, bodyTop]

    resultsLeft = left + SearchKeyboardWidth() + space.s5
    span = ResultsWidth()

    gridTop = bodyTop

    m.actions.theme = theme
    m.actions.barWidth = span
    m.actions.buttons = [{ id: "more", label: Phrase("action.loadMore"), icon: "chevron-down", style: "primary" }]

    actionsHeight = 0
    if m.actions.visible then actionsHeight = Int(sizes.textBase * 2.6) + space.s4

    gridHeight = CanvasHeight() - gridTop - space.s6 - actionsHeight

    m.grid.theme = theme
    m.grid.serverUrl = SessionFor(m.global).serverUrl
    m.grid.cardWidth = ResultCardWidth(span)
    m.grid.columns = ResultColumns(span)
    m.grid.captionRows = GridCaptionRows()
    m.grid.gridHeight = gridHeight
    m.grid.translation = [resultsLeft, gridTop]

    m.actions.translation = [resultsLeft, gridTop + gridHeight + space.s4]

    PlaceScrollBar(gridHeight)
    RefreshScrollBar()

    m.note.theme = theme
    m.note.fontSize = sizes.textBase
    m.note.noteWidth = span
    m.note.translation = [resultsLeft, gridTop]

    if IsBlank(m.term) then ShowNote(Phrase("search.opening"), "empty")
end sub

sub RefreshHeading()
    if IsBlank(m.term)
        m.heading.text = Phrase("nav.search")
        return
    end if

    if m.total > 0
        m.heading.text = CountLabel(Phrase("nav.search") + ": " + m.term, m.total)
        return
    end if
    m.heading.text = Phrase("nav.search") + ": " + m.term
end sub

sub onTyped()
    m.term = TextOrBlank(m.keyboard.text).Trim()
    m.searchTick.control = "stop"
    RefreshHeading()

    if IsBlank(m.term)
        Clear(Phrase("search.opening"))
        return
    end if

    if Len(m.term) < MinSearchLength()
        Clear(Phrase("search.keepTyping"))
        return
    end if

    m.searchTick.control = "start"
end sub

sub onSearchTick()
    Search()
end sub

sub Clear(message as string)
    m.generation = m.generation + 1
    m.cards = []
    m.total = 0
    m.nextOffset = 0
    m.loadingMore = false
    ShowMoreButton(false)
    ShowNote(message, "empty")
end sub

sub Search()
    m.generation = m.generation + 1
    m.cards = []
    m.total = 0
    m.nextOffset = 0
    m.loadingMore = false
    ShowMoreButton(false)
    ShowNote(Phrase("state.loading"), "loading")

    RequestPage(0)
end sub

sub RequestPage(offset as integer)
    session = SessionFor(m.global)

    m.pageGeneration = m.generation
    m.pageOffset = offset
    m.searchTask = SendRequest(SearchRequest(session.serverUrl, session.token, m.term, offset, SearchPageLimit()), "onResults")
end sub

sub LoadMoreResults()
    if m.loadingMore then return
    if not HasMore(m.cards.Count(), m.total) then return

    m.loadingMore = true
    RequestPage(m.nextOffset)
end sub

sub onResults(event as object)
    if m.pageGeneration <> m.generation then return

    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return

    if not parsed.ok
        m.loadingMore = false
        if m.pageOffset > 0
            RaiseToast("err", Phrase("error.couldNotLoadMore"))
            return
        end if
        ShowNote(Phrase("error.couldNotLoadSearch"), "error")
        return
    end if

    m.loadingMore = false
    m.total = Int(ValueAt(parsed.json, "total", 0))
    RefreshHeading()

    page = CardsFrom(ValueAt(parsed.json, "items", []), PosterCardFromJson)
    m.nextOffset = NextOffset(m.pageOffset, page.Count())
    m.cards.Append(page)

    if m.cards.Count() = 0
        ShowMoreButton(false)
        ShowNote(Phrase("empty.noResults"), "empty")
        return
    end if

    LoadStates(page)
end sub

sub LoadStates(cards as object)
    session = SessionFor(m.global)

    m.states = []
    m.rollups = []
    m.stateTasks = []
    m.statePage = cards
    m.stateGeneration = m.generation

    leafChunks = ChunkRefs(LeafRefs(cards), MaxBatchSize())
    rollupChunks = ChunkRefs(RollupTargets(cards), MaxBatchSize())
    m.pendingStates = leafChunks.Count() + rollupChunks.Count()

    if m.pendingStates = 0
        Publish()
        return
    end if

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
    Publish()
end sub

sub Publish()
    ShowMoreButton(HasMore(m.cards.Count(), m.total))
    ShowNote("", "loading")
    m.grid.visible = true

    held = ""
    focused = m.grid.focusedCard
    if focused <> invalid then held = TextOrBlank(ValueAt(focused, "id", ""))

    ordered = SortedByKind(m.cards)
    m.grid.cards = ordered

    at = IndexOfCard(ordered, held)
    if at > 0 then m.grid.jumpTo = at

    RefreshScrollBar()
end sub

sub RefreshScrollBar()
    span = ResultsWidth()
    metrics = CardMetrics(ResultCardWidth(span), m.grid.aspect, GridCaptionRows())
    UpdateGridScrollBar(m.grid, ResultColumns(span), metrics.height, Int(m.grid.gridHeight))
end sub

function ResultsWidth() as integer
    return CanvasWidth() - ContentLeft() - SearchKeyboardWidth() - SpacingScale().s5 - SpacingScale().s6
end function

sub ShowMoreButton(wanted as boolean)
    if m.actions.visible = wanted then return

    if not wanted and m.actions.isInFocusChain() then TakeFocus(m.grid)
    m.actions.visible = wanted
    render()
end sub

sub ShowNote(message as string, kind as string)
    m.note.kind = kind
    m.note.message = message
    m.note.visible = not IsBlank(message)
    if not IsBlank(message) then m.grid.visible = false
end sub

sub onAction()
    LoadMoreResults()
end sub

sub onCardSelected()
    target = m.grid.selected
    if target = invalid or target.Count() = 0 then return

    screen = ScreenForKind(ValueAt(target, "kind", ""))
    if IsBlank(screen) then return

    m.top.advanceTarget = target
    m.top.advance = screen
end sub

sub onCardFocused()
    RefreshScrollBar()
end sub

sub onFocus()
    if not m.top.hasFocus() then return

    if m.grid.visible
        m.grid.SetFocus(true)
        return
    end if
    m.keyboard.SetFocus(true)
end sub

sub onChoice()
    if HandledCardMenuChoice(m.top.choiceResult) then return
end sub

sub ReloadCardStates()
    if Len(m.term) < MinSearchLength() then return
    Search()
end sub

function onKeyEvent(key as string, press as boolean) as boolean
    if not KeysAreOurs() then return false
    if not press then return false

    if key = "options" and m.grid.isInFocusChain()
        return OpenedCardMenu(m.grid.focusedCard)
    end if

    if key = "right" and m.keyboard.isInFocusChain()
        if not m.grid.visible then return true
        m.grid.SetFocus(true)
        return true
    end if

    if key = "left" and not m.keyboard.isInFocusChain()
        m.keyboard.SetFocus(true)
        return true
    end if

    return HandledNodeStep(key, [m.grid, m.actions])
end function
