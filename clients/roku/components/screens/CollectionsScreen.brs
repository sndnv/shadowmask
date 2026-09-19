sub init()
    m.heading = m.top.FindNode("heading")
    m.grid = m.top.FindNode("grid")
    m.note = m.top.FindNode("note")
    m.more = m.top.FindNode("more")

    m.grid.ObserveField("selected", "onCardSelected")
    m.grid.ObserveField("focused", "onCardFocused")

    m.cards = []
    m.total = 0
    m.nextOffset = 0
    m.loadingMore = false
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

    m.heading.color = theme.text
    m.heading.font = SizedBoldFont(sizes.text2xl)
    DrawCrumbs(theme, [{ label: Phrase("nav.collections") }], left, top, ContentWidth())
    top = top + CrumbHeight()

    m.heading.translation = [left, top]
    RefreshHeading()

    gridTop = top + Int(sizes.text2xl * 1.4) + space.s4

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
        RequestPage(0)
    end if
end sub

sub RefreshHeading()
    m.heading.text = CountLabel(Phrase("nav.collections"), m.total)
end sub

sub RequestPage(offset as integer)
    session = SessionFor(m.global)

    if offset = 0 then ShowNote(Phrase("state.loading"), "loading")
    m.pageOffset = offset
    m.pageTask = SendRequest(CollectionsRequest(session.serverUrl, session.token, offset, GridPageLimit()), "onPage")
end sub

sub onPage(event as object)
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return

    if not parsed.ok
        m.loadingMore = false
        if m.pageOffset = 0
            ShowNote(Phrase("error.couldNotLoadCollections"), "error")
        else
            ShowMore(Phrase("error.couldNotLoadMore"), "error")
        end if
        return
    end if

    m.total = Int(ValueAt(parsed.json, "total", 0))
    RefreshHeading()

    page = CardsFrom(ValueAt(parsed.json, "items", []), CollectionCard)
    m.nextOffset = NextOffset(m.pageOffset, page.Count())

    if m.pageOffset = 0
        m.cards = page
        if page.Count() = 0
            ShowNote(Phrase("empty.noCollections"), "empty")
            return
        end if
        ShowNote("", "loading")
        m.grid.visible = true
        m.grid.cards = m.cards
        TakeFocus(m.grid)
        return
    end if

    m.loadingMore = false
    ShowMore("", "loading")
    m.cards.Append(page)
    m.grid.appendCards = page
end sub

sub RefreshScrollBar()
    metrics = CardMetrics(GridFittedCardWidth(), m.grid.aspect, GridCaptionRows())
    UpdateGridScrollBar(m.grid, GridColumns(), metrics.height, Int(m.grid.gridHeight))
end sub

sub onCardFocused()
    RefreshScrollBar()
    if m.loadingMore then return
    if not ShouldLoadMore(m.grid.focused, m.cards.Count(), m.total) then return

    m.loadingMore = true
    ShowMore(Phrase("state.loadingMore"), "loading")
    RequestPage(m.nextOffset)
end sub

sub ShowNote(message as string, kind as string)
    m.note.kind = kind
    m.note.message = message
    m.note.visible = not IsBlank(message)
    if not IsBlank(message) then m.grid.visible = false
end sub

sub onCardSelected()
    target = m.grid.selected
    if target = invalid or target.Count() = 0 then return

    m.top.advanceTarget = target
    m.top.advance = "CollectionScreen"
end sub

sub onChoice()
    HandledCardMenuChoice(m.top.choiceResult)
end sub

sub ReloadCardStates()
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
