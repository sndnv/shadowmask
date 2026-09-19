sub init()
    m.heading = m.top.FindNode("heading")
    m.rows = m.top.FindNode("rows")
    m.note = m.top.FindNode("note")

    m.rows.ObserveField("selected", "onCardSelected")
    m.rows.ObserveField("scrollOffset", "onRowsScrolled")
    m.rows.ObserveField("contentHeight", "onRowsScrolled")

    m.rails = []
    m.resumeCard = {}
    m.started = false
end sub

sub render()
    theme = m.top.theme
    if theme = invalid or theme.Count() = 0 then return

    sizes = TypeScale()
    space = SpacingScale()
    left = ContentLeft()
    top = ContentTop()

    m.heading.text = Phrase("nav.home")
    m.heading.color = theme.text
    m.heading.font = SizedBoldFont(sizes.text2xl)
    m.heading.translation = [left, top]

    rowsTop = top + Int(sizes.text2xl * 1.4) + space.s4

    m.rows.theme = theme
    m.rows.serverUrl = SessionFor(m.global).serverUrl
    m.rows.rowsWidth = CanvasWidth() - left
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

sub Load()
    session = SessionFor(m.global)

    m.hubJson = invalid
    m.continueJson = invalid
    m.pendingFeeds = 2
    m.failed = false

    ShowNote(Phrase("state.loading"), "loading")
    m.hubTask = SendRequest(HubRequest(session.serverUrl, session.token, session.userId), "onHub")
    m.continueTask = SendRequest(ContinueRequest(session.serverUrl, session.token, session.userId), "onContinue")
end sub

sub onHub(event as object)
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return

    if parsed.ok then m.hubJson = parsed.json
    if not parsed.ok then m.failed = true
    FeedArrived()
end sub

sub onContinue(event as object)
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return

    if parsed.ok then m.continueJson = parsed.json
    if not parsed.ok then m.failed = true
    FeedArrived()
end sub

sub FeedArrived()
    m.pendingFeeds = m.pendingFeeds - 1
    if m.pendingFeeds > 0 then return

    if m.failed and m.hubJson = invalid and m.continueJson = invalid
        ShowNote(Phrase("error.couldNotLoadHome"), "error")
        return
    end if

    m.rails = []

    watching = ContinueCards(m.continueJson)
    if watching.Count() > 0 then m.rails.Push({ heading: Phrase("home.continueWatching"), cards: watching })

    upNext = UpNextCards(m.continueJson)
    if upNext.Count() > 0 then m.rails.Push({ heading: Phrase("home.upNext"), cards: upNext })

    for each rail in HubRails(m.hubJson)
        m.rails.Push(rail)
    end for

    if m.rails.Count() = 0
        CheckLibraryAccess()
        return
    end if

    LoadStates(AllCards(m.rails))
end sub

function AllCards(rails as object) as object
    cards = []
    for each rail in rails
        for each card in rail.cards
            cards.Push(card)
        end for
    end for
    return cards
end function

sub LoadStates(cards as object)
    session = SessionFor(m.global)

    m.states = []
    m.rollups = []
    m.stateTasks = []

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

    for each rail in m.rails
        ApplyStates(rail.cards, m.states, m.rollups)
    end for
    Publish()
end sub

sub Publish()
    ShowNote("", "loading")
    m.rows.visible = true
    m.rows.rows = m.rails
    TakeFocus(m.rows)
end sub

sub CheckLibraryAccess()
    session = SessionFor(m.global)
    m.librariesTask = SendRequest(LibrariesRequest(session.serverUrl, session.token), "onLibraries")
end sub

sub onLibraries(event as object)
    parsed = ParseResponse(event.GetData().status, event.GetData().body)

    if parsed.ok and type(parsed.json) = "roArray" and parsed.json.Count() = 0
        ShowNote(Phrase("empty.noLibrariesShared"), "empty")
        return
    end if

    ShowNote(Phrase("empty.nothingToShowYet"), "empty")
end sub

sub ShowNote(message as string, kind as string)
    m.note.kind = kind
    m.note.message = message
    m.note.visible = not IsBlank(message)
    if not IsBlank(message) then m.rows.visible = false
end sub

sub onCardSelected()
    target = m.rows.selected
    if target = invalid or target.Count() = 0 then return

    screen = ScreenForKind(ValueAt(target, "kind", ""))
    if IsBlank(screen) then return

    m.top.advanceTarget = target
    m.top.advance = screen
end sub

sub onRowsScrolled()
    UpdateRowsScrollBar(m.rows)
end sub

sub onFocus()
    if not m.top.hasFocus() then return
    if m.rows.visible then m.rows.SetFocus(true)
end sub

function onKeyEvent(key as string, press as boolean) as boolean
    if not KeysAreOurs() then return false
    if not press or key <> "options" then return false
    if not m.rows.visible then return false

    return OpenedCardMenu(m.rows.focusedCard)
end function

sub onChoice()
    HandledCardMenuChoice(m.top.choiceResult)
end sub

sub ReloadCardStates()
    Load()
end sub
