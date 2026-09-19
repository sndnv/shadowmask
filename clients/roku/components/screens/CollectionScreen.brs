sub init()
    m.heading = m.top.FindNode("heading")
    m.overview = m.top.FindNode("overview")
    m.grid = m.top.FindNode("grid")
    m.note = m.top.FindNode("note")

    m.grid.ObserveField("selected", "onCardSelected")
    m.grid.ObserveField("focused", "onCardFocused")

    m.cards = []
    m.started = false
end sub

sub render()
    theme = m.top.theme
    target = m.top.target
    if theme = invalid or theme.Count() = 0 then return
    if target = invalid or target.Count() = 0 then return

    sizes = TypeScale()
    space = SpacingScale()
    left = ContentLeft()

    m.heading.text = TextOrBlank(ValueAt(target, "title", ""))
    m.heading.color = theme.text
    m.heading.font = SizedBoldFont(sizes.text2xl)
    m.heading.width = ContentWidth()
    m.heading.maxLines = 1
    m.heading.ellipsisText = "…"
    DrawCrumbs(theme, [{ label: Phrase("nav.collections"), screen: "CollectionsScreen", target: invalid }, { label: m.heading.text }], left, ContentTop(), ContentWidth())
    headingTop = ContentTop() + CrumbHeight()

    m.heading.translation = [left, headingTop]
    headingBottom = headingTop + Int(sizes.text2xl * 1.4)

    overview = TextOrBlank(m.overview.text)
    lines = 0
    if not IsBlank(overview) then lines = 2

    m.overview.visible = lines > 0
    m.overview.color = theme.muted
    m.overview.font = SizedFont(sizes.textSm)
    m.overview.width = ContentWidth()
    m.overview.wrap = true
    m.overview.maxLines = 2
    m.overview.ellipsisText = "…"
    m.overview.translation = [left, headingBottom + space.s2]

    gridTop = headingBottom + space.s5
    if lines > 0 then gridTop = headingBottom + space.s2 + Int(sizes.textSm * 1.3) * lines + space.s5

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

    if not m.started
        m.started = true
        Load()
    end if
end sub

sub Load()
    session = SessionFor(m.global)

    ShowNote(Phrase("state.loading"), "loading")
    m.detailTask = SendRequest(CollectionRequest(session.serverUrl, session.token, ValueAt(m.top.target, "id", "")), "onDetail")
end sub

sub onDetail(event as object)
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return

    if not parsed.ok
        ShowNote(Phrase("error.couldNotLoadCollection"), "error")
        return
    end if

    m.heading.text = TextOrBlank(ValueAt(parsed.json, "name", ""))
    m.overview.text = TextOrBlank(ValueAt(parsed.json, "overview", ""))
    render()

    PublishBackdrop(ValueAt(parsed.json, "artwork", invalid))

    m.cards = CardsFrom(ValueAt(parsed.json, "items", []), MovieCard)
    if m.cards.Count() = 0
        ShowNote(Phrase("empty.noResults"), "empty")
        return
    end if

    ShowNote("", "loading")
    m.grid.visible = true
    m.grid.cards = m.cards
    TakeFocus(m.grid)

    LoadStates()
end sub

sub LoadStates()
    session = SessionFor(m.global)

    m.states = []
    m.rollups = []
    m.stateTasks = []

    leafChunks = ChunkRefs(LeafRefs(m.cards), MaxBatchSize())
    m.pendingStates = leafChunks.Count()
    if m.pendingStates = 0 then return

    for each chunk in leafChunks
        m.stateTasks.Push(SendRequest(StateBatchRequest(session.serverUrl, session.token, session.userId, chunk), "onStates"))
    end for
end sub

sub onStates(event as object)
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if parsed.ok and type(parsed.json) = "roArray" then m.states.Append(parsed.json)

    m.pendingStates = m.pendingStates - 1
    if m.pendingStates > 0 then return

    ApplyStates(m.cards, m.states, m.rollups)
    m.grid.cardStates = CardStateList(m.cards)
end sub

sub ShowNote(message as string, kind as string)
    m.note.kind = kind
    m.note.message = message
    m.note.visible = not IsBlank(message)
    if not IsBlank(message) then m.grid.visible = false
end sub

sub RefreshScrollBar()
    metrics = CardMetrics(GridFittedCardWidth(), m.grid.aspect, GridCaptionRows())
    UpdateGridScrollBar(m.grid, GridColumns(), metrics.height, Int(m.grid.gridHeight))
end sub

sub onCardFocused()
    RefreshScrollBar()
end sub

sub onCardSelected()
    target = m.grid.selected
    if target = invalid or target.Count() = 0 then return

    screen = ScreenForKind(ValueAt(target, "kind", ""))
    if IsBlank(screen) then return

    m.top.advanceTarget = target
    m.top.advance = screen
end sub

sub onChoice()
    HandledCardMenuChoice(m.top.choiceResult)
end sub

sub ReloadCardStates()
    LoadStates()
end sub

sub onFocus()
    if not m.top.hasFocus() then return
    if m.grid.visible then m.grid.SetFocus(true)
end sub

function onKeyEvent(key as string, press as boolean) as boolean
    if not KeysAreOurs() then return false
    if not press or key <> "options" then return false
    if not m.grid.visible then return false

    return OpenedCardMenu(m.grid.focusedCard)
end function
