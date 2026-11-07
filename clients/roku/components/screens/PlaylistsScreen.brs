sub init()
    m.heading = m.top.FindNode("heading")
    m.actions = m.top.FindNode("actions")
    m.rows = m.top.FindNode("rows")
    m.note = m.top.FindNode("note")

    m.actions.ObserveField("activated", "onAction")
    m.rows.ObserveField("selected", "onCardSelected")
    m.rows.ObserveField("scrollOffset", "onRowsScrolled")
    m.rows.ObserveField("contentHeight", "onRowsScrolled")

    m.queueCards = []
    m.playlistCards = []
    m.serial = 0
    m.playlistTask = invalid
    m.clearTask = invalid
    m.reloadOnPlaylistAdd = true
    m.started = false

    m.top.ObserveField("focusedChild", "onFocus")
end sub

sub render()
    theme = m.top.theme
    if theme = invalid or theme.Count() = 0 then return

    sizes = TypeScale()
    space = SpacingScale()
    left = ContentLeft()
    top = ContentTop()

    DrawCrumbs(theme, [{ label: Phrase("nav.playlists") }], left, top, ContentWidth())
    top = top + CrumbHeight()

    m.heading.text = Phrase("nav.playlists")
    m.heading.color = theme.text
    m.heading.font = SizedBoldFont(sizes.text2xl)
    m.heading.translation = [left, top]
    top = top + Int(sizes.text2xl * 1.4) + space.s4

    m.actions.theme = theme
    m.actions.barWidth = ContentWidth()
    m.actions.buttons = PlaylistsBarButtons(m.queueCards.Count())
    m.actions.translation = [left, top]
    rowsTop = top + Int(m.actions.barHeight) + space.s5

    m.rows.pinned = true
    m.rows.peek = true
    m.rows.theme = theme
    m.rows.serverUrl = SessionFor(m.global).serverUrl
    m.rows.rowsWidth = CanvasWidth() - left * 2
    m.rows.rowsHeight = CanvasHeight() - rowsTop
    m.rows.cardWidth = RailCardWidth()
    m.rows.translation = [left, rowsTop]

    PlaceScrollBar(CanvasHeight() - rowsTop - space.s5)
    UpdateRowsScrollBar(m.rows)

    m.note.theme = theme
    m.note.fontSize = sizes.textBase
    m.note.noteWidth = ContentWidth()
    m.note.translation = [left, rowsTop]

    if not m.started
        m.started = true
        Load()
    end if
end sub

sub Load(quiet = false as boolean)
    session = SessionFor(m.global)

    m.serial = m.serial + 1
    m.queueJson = invalid
    m.listsJson = invalid
    m.pending = 2
    m.failed = false
    m.quiet = quiet

    if not quiet then ShowNote(Phrase("state.loading"), "loading")
    m.queueTask = SendRequest(QueueRequest(session.serverUrl, session.token, session.userId), "onQueue")
    m.listsTask = SendRequest(PlaylistsRequest(session.serverUrl, session.token, session.userId), "onLists")
end sub

sub onQueue(event as object)
    if m.queueTask = invalid or not m.queueTask.IsSameNode(event.GetRoSGNode()) then return
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return

    if parsed.ok then m.queueJson = parsed.json
    if not parsed.ok then m.failed = true
    ListArrived()
end sub

sub onLists(event as object)
    if m.listsTask = invalid or not m.listsTask.IsSameNode(event.GetRoSGNode()) then return
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return

    if parsed.ok then m.listsJson = parsed.json
    if not parsed.ok then m.failed = true
    ListArrived()
end sub

sub ListArrived()
    m.pending = m.pending - 1
    if m.pending > 0 then return
    if KeepsShownOnReload(m.quiet and m.rows.visible, m.failed)
        RaiseToast("err", Phrase("error.couldNotLoadPlaylists"))
        return
    end if

    if m.failed
        ShowNote(Phrase("error.couldNotLoadPlaylists"), "error")
        TakeFocus(m.actions)
        return
    end if

    m.queueCards = QueueCards(m.queueJson)
    m.playlistCards = CardsFrom(ListItems(m.listsJson), PlaylistCard)
    LoadStates()
end sub

sub LoadStates()
    session = SessionFor(m.global)

    m.states = []
    chunks = ChunkRefs(LeafRefs(m.queueCards), MaxBatchSize())
    m.pendingStates = chunks.Count()
    if m.pendingStates = 0
        Publish()
        return
    end if

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

    ApplyStates(m.queueCards, m.states, [])
    Publish()
end sub

sub Publish()
    m.actions.buttons = PlaylistsBarButtons(m.queueCards.Count())

    rows = PlaylistsRows(m.queueCards, m.playlistCards)
    if rows.Count() = 0
        ShowNote(Phrase("empty.noPlaylists"), "empty")
        TakeFocus(m.actions)
        return
    end if

    ShowNote("", "loading")
    m.rows.visible = true
    m.rows.rows = rows
    if not m.actions.isInFocusChain() then TakeFocus(m.rows)
end sub

sub ShowNote(message as string, kind as string)
    m.note.kind = kind
    m.note.message = message
    m.note.visible = not IsBlank(message)
    if not IsBlank(message) then m.rows.visible = false
end sub

sub onAction(event as object)
    id = TextOrBlank(event.GetData())

    if id = "playQueue"
        if m.queueCards.Count() > 0 then PlayQueued(m.queueCards[0])
    else if id = "clearQueue"
        AskClearQueue()
    else if id = "newPlaylist"
        AskNewPlaylist()
    end if
end sub

sub PlayQueued(card as object)
    m.menuCard = card
    AskCardVersions(card, SessionFor(m.global))
end sub

sub AskClearQueue()
    if m.queueCards.Count() = 0 then return

    m.top.choiceRequest = {
        field: "clearQueue",
        title: Phrase("action.clearQueue"),
        message: Phrase("confirm.clearQueue"),
        options: ConfirmOptions(Phrase("action.clearQueue"), true),
        selected: 1
    }
end sub

sub CommitClearQueue()
    if m.clearTask <> invalid then return

    session = SessionFor(m.global)
    m.clearTask = SendRequest(QueueClearRequest(session.serverUrl, session.token, session.userId), "onQueueCleared")
end sub

sub onQueueCleared(event as object)
    m.clearTask = invalid
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return

    if not parsed.ok
        RaiseActionFailed()
        return
    end if

    RaiseToast("ok", Phrase("toast.ok.queueCleared"))
    Load(true)
end sub

sub onCardSelected()
    target = m.rows.selected
    if target = invalid or target.Count() = 0 then return

    if not IsBlank(TextOrBlank(ValueAt(target, "queueEntryId", "")))
        PlayQueued(target)
        return
    end if

    screen = ScreenForKind(ValueAt(target, "kind", ""))
    if IsBlank(screen) then return

    m.top.advanceTarget = target
    m.top.advance = screen
end sub

sub onRowsScrolled()
    UpdateRowsScrollBar(m.rows)
end sub

sub onChoice()
    result = m.top.choiceResult
    if TextOrBlank(ValueAt(result, "field", "")) = "clearQueue"
        if ConfirmAccepted(result) then CommitClearQueue()
        return
    end if

    if HandledPlaylistChoice(result) then return
    if HandledPlaylistMenu(result) then return
    HandledCardMenuChoice(result)
end sub

sub onKeyboard()
    result = m.top.keyboardResult
    if HandledCardMenuKeyboard(result) then return
    HandledPlaylistKeyboard(result)
end sub

sub PlaylistQueued()
    Load(true)
end sub

sub PlaylistCreated()
    Load(true)
end sub

sub PlaylistRenamed()
    Load(true)
end sub

sub PlaylistDeleted()
    Load(true)
end sub

sub ReloadCardStates()
    Load(true)
end sub

sub onRevealed()
    Load(true)
end sub

sub onFocus()
    if not m.top.hasFocus() then return
    if m.rows.visible
        m.rows.SetFocus(true)
        return
    end if
    m.actions.SetFocus(true)
end sub

function onKeyEvent(key as string, press as boolean) as boolean
    if not KeysAreOurs() then return false
    if not press then return false

    if key = "options" and m.rows.visible and m.rows.isInFocusChain()
        return OpenedCardMenu(m.rows.focusedCard)
    end if

    if key = "up" and m.rows.isInFocusChain()
        m.actions.SetFocus(true)
        return true
    end if

    if key = "down" and m.actions.isInFocusChain() and m.rows.visible
        m.rows.SetFocus(true)
        return true
    end if

    return false
end function
