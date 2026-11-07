sub init()
    m.heading = m.top.FindNode("heading")
    m.rows = m.top.FindNode("rows")
    m.note = m.top.FindNode("note")
    m.summary = m.top.FindNode("summary")

    m.rows.ObserveField("selected", "onCardSelected")
    m.rows.ObserveField("scrollOffset", "onRowsScrolled")
    m.rows.ObserveField("contentHeight", "onRowsScrolled")
    m.rows.ObserveField("focusedCard", "onCardFocused")

    m.rails = []
    m.resumeCard = {}
    m.started = false
    m.summaries = {}
    m.summaryAsked = {}
    m.summarySerial = 0
    m.restTimer = invalid

    m.top.ObserveField("focusedChild", "onFocus")
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

    summaryTop = top + Int(sizes.text2xl * 1.4) + space.s4
    rowsTop = summaryTop + SummaryPanelHeight() + space.s5

    m.summary.theme = theme
    m.summary.summaryWidth = SummaryColumnWidth()
    m.summary.summaryHeight = SummaryPanelHeight()
    m.summary.translation = [left, summaryTop]

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
    m.note.translation = [left, summaryTop]

    if not m.started
        m.started = true
        Load()
    end if
end sub

sub Load(quiet = false as boolean)
    session = SessionFor(m.global)

    m.hubJson = invalid
    m.continueJson = invalid
    m.pendingFeeds = 2
    m.failed = false
    m.quiet = quiet

    if not quiet then ShowNote(Phrase("state.loading"), "loading")
    m.hubTask = SendRequest(HubRequest(session.serverUrl, session.token, session.userId), "onHub")
    m.continueTask = SendRequest(ContinueRequest(session.serverUrl, session.token, session.userId), "onContinue")
end sub

sub onHub(event as object)
    if m.hubTask = invalid or not m.hubTask.IsSameNode(event.GetRoSGNode()) then return
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return

    if parsed.ok then m.hubJson = parsed.json
    if not parsed.ok then m.failed = true
    FeedArrived()
end sub

sub onContinue(event as object)
    if m.continueTask = invalid or not m.continueTask.IsSameNode(event.GetRoSGNode()) then return
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return

    if parsed.ok then m.continueJson = parsed.json
    if not parsed.ok then m.failed = true
    FeedArrived()
end sub

sub FeedArrived()
    m.pendingFeeds = m.pendingFeeds - 1
    if m.pendingFeeds > 0 then return
    if KeepsShownOnReload(m.quiet and m.rows.visible, m.failed) then return

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
    m.summary.visible = true
    m.rows.rows = m.rails
    TakeFocus(m.rows)
    if m.rows.isInFocusChain() then return

    card = PublishedSummaryCard(m.rows.focusedCard, m.rails)
    if card <> invalid then ShowSummary(card)
end sub

sub onCardFocused()
    card = m.rows.focusedCard
    ShowSummary(card)

    m.summarySerial = m.summarySerial + 1
    StopTimer(m.restTimer)
    m.restTimer = ArmTimer(SummaryRestSeconds(), "onSummaryRest", { serial: m.summarySerial })
end sub

function CachedSummary(card as dynamic) as dynamic
    key = SummaryKey(card)
    if IsBlank(key) or not m.summaries.DoesExist(key) then return invalid
    return m.summaries[key]
end function

sub ShowSummary(card as dynamic)
    cached = CachedSummary(card)
    if cached <> invalid
        m.summary.summary = cached.content
        return
    end if
    m.summary.summary = SummaryContent(card, invalid)
end sub

function SummaryBackdropOf(card as dynamic) as string
    cached = CachedSummary(card)
    if cached <> invalid then return cached.backdrop
    return SummaryBackdrop(SessionFor(m.global).serverUrl, card, invalid)
end function

sub onSummaryRest(event as object)
    if Int(TimerData(event).serial) <> m.summarySerial then return
    if not m.rows.visible then return

    card = m.rows.focusedCard
    PublishBackdropUrl(SummaryBackdropOf(card))
    AskSummary(card)
end sub

sub AskSummary(card as dynamic)
    key = SummaryKey(card)
    if IsBlank(key) or m.summaries.DoesExist(key) or m.summaryAsked.DoesExist(key) then return

    session = SessionFor(m.global)
    task = SendRequest(SummaryRequest(session.serverUrl, session.token, card), "onSummary")
    task.AddFields({ summaryKey: key, summaryCard: card })
    m.summaryAsked[key] = true
end sub

sub onSummary(event as object)
    task = event.GetRoSGNode()
    key = TextOrBlank(task.summaryKey)
    m.summaryAsked.Delete(key)

    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return
    if not parsed.ok then return

    asked = task.summaryCard
    m.summaries[key] = {
        content: SummaryContent(asked, parsed.json),
        backdrop: SummaryBackdrop(SessionFor(m.global).serverUrl, asked, parsed.json)
    }

    card = m.rows.focusedCard
    if SummaryKey(card) <> key or not m.rows.visible then return

    ShowSummary(card)
    PublishBackdropUrl(SummaryBackdropOf(card))
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
    if IsBlank(message) then return

    m.rows.visible = false
    m.summary.visible = false
    if kind <> "loading" then PublishBackdropUrl("")
end sub

sub onCardSelected()
    target = m.rows.selected
    if target = invalid or target.Count() = 0 then return

    if not IsBlank(TextOrBlank(ValueAt(target, "queueEntryId", "")))
        m.menuCard = target
        AskCardVersions(target, SessionFor(m.global))
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

sub onRevealed()
    Load(true)
end sub
