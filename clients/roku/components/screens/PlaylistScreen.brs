sub init()
    m.heading = m.top.FindNode("heading")
    m.actions = m.top.FindNode("actions")
    m.titlesHeading = m.top.FindNode("titlesHeading")
    m.grid = m.top.FindNode("grid")
    m.note = m.top.FindNode("note")

    m.actions.ObserveField("activated", "onAction")
    m.grid.ObserveField("selected", "onCardSelected")
    m.grid.ObserveField("focused", "onCardFocused")

    m.id = ""
    m.name = ""
    m.cards = []
    m.serial = 0
    m.loaded = false
    m.started = false
    m.detailTask = invalid
    m.editTask = invalid
    m.playlistTask = invalid
    m.focusAfter = -1

    m.top.ObserveField("focusedChild", "onFocus")
end sub

sub render()
    theme = m.top.theme
    target = m.top.target
    if theme = invalid or theme.Count() = 0 then return
    if target = invalid or target.Count() = 0 then return

    if IsBlank(m.id)
        m.id = TextOrBlank(ValueAt(target, "id", ""))
        m.name = TextOrBlank(ValueAt(target, "title", ""))
    end if

    sizes = TypeScale()
    space = SpacingScale()
    left = ContentLeft()
    top = ContentTop()

    DrawCrumbs(theme, [{ label: Phrase("nav.playlists"), screen: "PlaylistsScreen", target: invalid }, { label: m.name }], left, top, ContentWidth())
    top = top + CrumbHeight()

    m.heading.text = m.name
    m.heading.color = theme.text
    m.heading.font = SizedBoldFont(sizes.text2xl)
    m.heading.width = ContentWidth()
    m.heading.maxLines = 1
    m.heading.ellipsisText = "…"
    m.heading.translation = [left, top]
    top = top + Int(sizes.text2xl * 1.4) + space.s4

    m.actions.theme = theme
    m.actions.barWidth = ContentWidth()
    m.actions.buttons = PlaylistMenuActions(m.cards.Count())
    m.actions.translation = [left, top]
    gridTop = top + Int(m.actions.barHeight) + space.s5

    m.titlesHeading.visible = m.loaded
    if m.titlesHeading.visible
        m.titlesHeading.text = CountedHeading(Phrase("heading.titles"), m.cards.Count())
        m.titlesHeading.color = theme.text
        m.titlesHeading.font = SizedBoldFont(sizes.textXl)
        m.titlesHeading.width = ContentWidth()
        m.titlesHeading.maxLines = 1
        m.titlesHeading.translation = [left, gridTop]
        gridTop = gridTop + Int(sizes.textXl * 1.35) + space.s3
    end if

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

sub Load(quiet = false as boolean)
    session = SessionFor(m.global)

    if not quiet then ShowNote(Phrase("state.loading"), "loading")
    m.detailTask = SendRequest(PlaylistRequest(session.serverUrl, session.token, session.userId, m.id), "onDetail")
end sub

sub onDetail(event as object)
    if m.detailTask = invalid or not m.detailTask.IsSameNode(event.GetRoSGNode()) then return
    m.detailTask = invalid

    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return

    if not parsed.ok
        m.focusAfter = -1
        m.loaded = false
        render()
        ShowNote(Phrase("error.couldNotLoadPlaylist"), "error")
        TakeFocus(m.actions)
        return
    end if

    m.name = TextOrBlank(ValueAt(parsed.json, "name", m.name))
    m.cards = PlaylistItemCards(parsed.json)
    m.loaded = true
    render()

    if m.cards.Count() = 0
        m.focusAfter = -1
        ShowNote(Phrase("empty.playlist"), "empty")
        TakeFocus(m.actions)
        return
    end if

    ShowNote("", "loading")
    m.grid.visible = true
    m.grid.cards = m.cards
    if m.focusAfter >= 0
        m.grid.jumpTo = ClampInt(m.focusAfter, 0, m.cards.Count() - 1)
        m.focusAfter = -1
        TakeFocus(m.grid)
    else if not m.actions.isInFocusChain()
        TakeFocus(m.grid)
    end if

    LoadStates()
end sub

sub LoadStates()
    session = SessionFor(m.global)

    m.serial = m.serial + 1
    m.states = []
    chunks = ChunkRefs(LeafRefs(m.cards), MaxBatchSize())
    m.pendingStates = chunks.Count()

    for each chunk in chunks
        task = SendRequest(StateBatchRequest(session.serverUrl, session.token, session.userId, chunk), "onStates")
        task.AddFields({ serial: m.serial })
    end for
end sub

sub onStates(event as object)
    if Int(event.GetRoSGNode().serial) <> m.serial then return

    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if parsed.ok and type(parsed.json) = "roArray" then m.states.Append(parsed.json)

    m.pendingStates = m.pendingStates - 1
    if m.pendingStates > 0 then return

    ApplyStates(m.cards, m.states, [])
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

function CurrentPlaylist() as object
    return { id: m.id, title: m.name }
end function

function Settling() as boolean
    return m.editTask <> invalid or m.detailTask <> invalid
end function

sub onAction(event as object)
    RunPlaylistAction(TextOrBlank(event.GetData()), CurrentPlaylist())
end sub

sub OpenItemMenu()
    index = Int(m.grid.focused)
    if index < 0 or index >= m.cards.Count() then return

    m.itemIndex = index
    m.itemActions = PlaylistItemMenuActions(m.cards.Count(), index)
    m.top.choiceRequest = {
        field: "playlistItem",
        title: CardMenuTitle(m.cards[index]),
        kind: "menu",
        options: MenuOptions(m.itemActions),
        selected: 0
    }
end sub

sub RunItemAction(id as string)
    if Settling() then return

    index = m.itemIndex
    if index < 0 or index >= m.cards.Count() then return

    card = m.cards[index]
    session = SessionFor(m.global)
    m.itemCard = card

    if id = "removeItem"
        m.focusAfter = index
        m.editTask = SendRequest(PlaylistRemoveRequest(session.serverUrl, session.token, session.userId, m.id, card.id), "onItemRemoved")
        return
    end if

    stepBy = 1
    if id = "moveUp" then stepBy = -1
    position = MovePosition(m.cards, index, stepBy)
    if position < 0 then return

    m.focusAfter = index + stepBy
    m.editTask = SendRequest(PlaylistMoveRequest(session.serverUrl, session.token, session.userId, m.id, card.id, position), "onItemMoved")
end sub

function ItemAnswer(event as object) as dynamic
    m.editTask = invalid
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return invalid

    if not parsed.ok
        RaiseActionFailed()
        m.focusAfter = m.itemIndex
    end if
    return parsed
end function

sub onItemMoved(event as object)
    parsed = ItemAnswer(event)
    if parsed = invalid then return

    Load(true)
end sub

sub onItemRemoved(event as object)
    parsed = ItemAnswer(event)
    if parsed = invalid then return

    if parsed.ok then RaiseToast("ok", PhraseWith("toast.ok.removedFromPlaylist", { title: TextOrBlank(ValueAt(m.itemCard, "title", "")) }))
    Load(true)
end sub

sub onChoice()
    result = m.top.choiceResult
    if TextOrBlank(ValueAt(result, "field", "")) = "playlistItem"
        if ValueAt(result, "cancelled", false) = true or type(m.itemActions) <> "roArray" or m.itemActions.Count() = 0 then return
        RunItemAction(m.itemActions[ClampInt(result.index, 0, m.itemActions.Count() - 1)].id)
        return
    end if

    if HandledPlaylistChoice(result) then return
    HandledCardMenuChoice(result)
end sub

sub onKeyboard()
    result = m.top.keyboardResult
    if HandledCardMenuKeyboard(result) then return
    HandledPlaylistKeyboard(result)
end sub

sub PlaylistQueued()
    m.top.advance = "back"
end sub

sub PlaylistCreated()
    Load(true)
end sub

sub PlaylistRenamed()
    m.name = m.playlistName
    render()
end sub

sub PlaylistDeleted()
    m.top.advance = "back"
end sub

sub ReloadCardStates()
    LoadStates()
end sub

sub onRevealed()
    if m.started then Load(true)
end sub

sub onFocus()
    if not m.top.hasFocus() then return
    if m.grid.visible
        m.grid.SetFocus(true)
        return
    end if
    m.actions.SetFocus(true)
end sub

function onKeyEvent(key as string, press as boolean) as boolean
    if not KeysAreOurs() then return false
    if not press then return false

    if key = "options" and m.grid.visible and m.grid.isInFocusChain()
        if not Settling() then OpenItemMenu()
        return true
    end if

    if key = "up" and m.grid.isInFocusChain()
        m.actions.SetFocus(true)
        return true
    end if

    if key = "down" and m.actions.isInFocusChain() and m.grid.visible
        m.grid.SetFocus(true)
        return true
    end if

    return false
end function
