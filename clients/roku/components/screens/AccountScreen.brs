sub init()
    InitCrumbs()
    InitSections([
        { id: "watchlistRail" },
        { id: "favoritesRail" },
        { id: "historyRail" },
        { id: "emptyLibrary", focus: "libraryNote" },
        { id: "profileSection", focus: "supportActions" },
        { id: "playbackSection", focus: "autoplayActions" },
        { id: "appearanceSection", focus: "themeCards" },
        { id: "serverSection", focus: "serverActions" },
        { id: "sessionSection", focus: "sessionActions" }
    ])

    m.heading = m.top.FindNode("heading")
    m.tabs = m.top.FindNode("tabs")
    m.tabRule = m.top.FindNode("tabRule")
    m.tabMarker = m.top.FindNode("tabMarker")
    m.watchlistRail = m.top.FindNode("watchlistRail")
    m.favoritesRail = m.top.FindNode("favoritesRail")
    m.historyRail = m.top.FindNode("historyRail")
    m.libraryNote = m.top.FindNode("libraryNote")
    m.profileHeading = m.top.FindNode("profileHeading")
    m.profileFacts = m.top.FindNode("profileFacts")
    m.profileHelp = m.top.FindNode("profileHelp")
    m.supportHeading = m.top.FindNode("supportHeading")
    m.supportChanged = m.top.FindNode("supportChanged")
    m.supportFacts = m.top.FindNode("supportFacts")
    m.supportActions = m.top.FindNode("supportActions")
    m.playbackHeading = m.top.FindNode("playbackHeading")
    m.autoplayName = m.top.FindNode("autoplayName")
    m.autoplayActions = m.top.FindNode("autoplayActions")
    m.autoplayHelp = m.top.FindNode("autoplayHelp")
    m.autoplayRule = m.top.FindNode("autoplayRule")
    m.timeDisplayName = m.top.FindNode("timeDisplayName")
    m.timeDisplayActions = m.top.FindNode("timeDisplayActions")
    m.timeDisplayHelp = m.top.FindNode("timeDisplayHelp")
    m.timeDisplayRule = m.top.FindNode("timeDisplayRule")
    m.diagnosticsName = m.top.FindNode("diagnosticsName")
    m.diagnosticsActions = m.top.FindNode("diagnosticsActions")
    m.diagnosticsHelp = m.top.FindNode("diagnosticsHelp")
    m.appearanceHeading = m.top.FindNode("appearanceHeading")
    m.themeCards = m.top.FindNode("themeCards")
    m.contrastBar = m.top.FindNode("contrastBar")
    m.profilePanel = m.top.FindNode("profilePanel")
    m.supportPanel = m.top.FindNode("supportPanel")
    m.playbackPanel = m.top.FindNode("playbackPanel")
    m.appearancePanel = m.top.FindNode("appearancePanel")
    m.serverPanel = m.top.FindNode("serverPanel")
    m.sessionPanel = m.top.FindNode("sessionPanel")
    m.serverHeading = m.top.FindNode("serverHeading")
    m.serverAddress = m.top.FindNode("serverAddress")
    m.serverActions = m.top.FindNode("serverActions")
    m.sessionHeading = m.top.FindNode("sessionHeading")
    m.deviceLine = m.top.FindNode("deviceLine")
    m.sessionActions = m.top.FindNode("sessionActions")

    m.tabs.ObserveField("activated", "onTab")
    m.supportActions.ObserveField("activated", "onSupportAction")
    m.autoplayActions.ObserveField("activated", "onPlaybackAction")
    m.timeDisplayActions.ObserveField("activated", "onPlaybackAction")
    m.diagnosticsActions.ObserveField("activated", "onPlaybackAction")
    m.themeCards.ObserveField("activated", "onThemePick")
    m.contrastBar.ObserveField("activated", "onContrastToggle")
    m.serverActions.ObserveField("activated", "onServerAction")
    m.sessionActions.ObserveField("activated", "onSessionAction")
    m.watchlistRail.ObserveField("selected", "onCardSelected")
    m.favoritesRail.ObserveField("selected", "onCardSelected")
    m.historyRail.ObserveField("selected", "onCardSelected")

    m.tab = 0
    m.user = {}
    m.playbackChoices = []
    m.overrides = ReadCapabilityOverrides()
    m.railCards = { watchlist: [], favorites: [], history: [] }
    m.railRefs = { watchlist: [], favorites: [], history: [] }
    m.railEntries = { watchlist: {}, favorites: {}, history: {} }
    m.tasks = []
    m.leavingServer = false
    m.visited = false
    m.started = false
end sub

function LibraryRails() as object
    return [
        { kind: "watchlist", node: m.watchlistRail, id: "watchlistRail" },
        { kind: "favorites", node: m.favoritesRail, id: "favoritesRail" },
        { kind: "history", node: m.historyRail, id: "historyRail" }
    ]
end function

sub render()
    theme = m.top.theme
    if theme = invalid or theme.Count() = 0 then return

    sizes = TypeScale()
    space = SpacingScale()
    left = ContentLeft()

    DrawCrumbs(theme, [{ label: Phrase("nav.account") }], left, ContentTop(), ContentWidth())

    m.heading.text = Phrase("nav.account")
    m.heading.color = theme.text
    m.heading.font = SizedBoldFont(sizes.text2xl)
    m.heading.translation = [left, ContentTop() + CrumbHeight()]

    tabsTop = ContentTop() + CrumbHeight() + Int(sizes.text2xl * 1.4) + space.s3
    m.tabs.theme = theme
    m.tabs.barWidth = ContentWidth()
    m.tabs.translation = [left, tabsTop]
    RefreshTabs()

    m.ruleTop = tabsTop + BarHeight(m.tabs) + space.s2
    DrawTabRule(theme, left, m.ruleTop)

    m.viewportTop = m.ruleTop + TabRuleHeight() + space.s5
    m.top.FindNode("viewport").translation = [left, m.viewportTop]

    PlaceScrollBar(CanvasHeight() - m.viewportTop - space.s5)

    Layout()

    if not m.started
        m.started = true
        LoadAccount()
    end if
end sub

function BarHeight(bar as object) as integer
    height = Int(bar.barHeight)
    if height <= 0 then return LinkControlHeight()

    return height
end function

function TabRuleHeight() as integer
    return 4
end function

sub DrawTabRule(theme as object, left as integer, top as integer)
    m.tabRule.width = ContentWidth()
    m.tabRule.height = 2
    m.tabRule.color = theme.border
    m.tabRule.translation = [left, top + TabRuleHeight() - 2]

    spans = m.tabs.slotSpans
    if type(spans) <> "roArray" or m.tab >= spans.Count()
        m.tabMarker.visible = false
        return
    end if

    span = spans[m.tab]
    m.tabMarker.visible = true
    m.tabMarker.width = span.width
    m.tabMarker.height = TabRuleHeight()
    m.tabMarker.color = theme.accent
    m.tabMarker.translation = [left + span.left, top]
end sub

sub RefreshTabs()
    m.tabs.buttons = [
        { id: "library", label: Phrase("heading.library"), style: "joined", pressed: m.tab = 0 },
        { id: "profile", label: Phrase("heading.profile"), style: "joined", pressed: m.tab = 1 },
        { id: "signOut", label: Phrase("action.signOut"), danger: true }
    ]
end sub

sub Layout()
    theme = m.top.theme
    if theme = invalid or theme.Count() = 0 then return

    for each id in ["watchlistRail", "favoritesRail", "historyRail", "emptyLibrary", "profileSection", "appearanceSection", "serverSection", "sessionSection"]
        HideSection(id)
    end for

    if m.tab = 0
        LayoutLibrary(theme)
    else
        LayoutProfile(theme)
    end if

    LayoutSections(ContentWidth(), CanvasHeight() - m.viewportTop - SpacingScale().s5)
end sub

sub LayoutLibrary(theme as object)
    sizes = TypeScale()
    serverUrl = SessionFor(m.global).serverUrl
    shown = 0

    for each rail in LibraryRails()
        cards = m.railCards[rail.kind]
        if cards.Count() > 0
            shown = shown + 1

            rail.node.theme = theme
            rail.node.serverUrl = serverUrl
            rail.node.railWidth = ContentWidth()
            rail.node.cardWidth = RailCardWidth()
            rail.node.captionRows = GridCaptionRows()
            rail.node.heading = PersonalListDef(rail.kind).heading
            rail.node.cards = cards
            ShowSection(rail.id, Int(rail.node.railHeight))
        end if
    end for

    if shown > 0 then return

    m.libraryNote.theme = theme
    m.libraryNote.kind = "empty"
    m.libraryNote.fontSize = sizes.textBase
    m.libraryNote.noteWidth = ContentWidth()
    m.libraryNote.message = Phrase("empty.noLibraryItems")
    m.libraryNote.translation = [0, 0]
    if m.started then ShowSection("emptyLibrary", TextLinePitch(sizes.textBase), false)
end sub

function PanelPad() as integer
    return SpacingScale().s5
end function

sub PaintPanel(plate as object, theme as object, at as integer, width as integer, height as integer)
    plate.boxWidth = width
    plate.boxHeight = height
    plate.fillColor = theme.surface
    plate.lineColor = theme.border
    plate.translation = [at, 0]
end sub

function ToggleButton(id as string, label as string, on as boolean) as object
    return { id: id, label: label, icon: "", iconOn: "icon-check", pressed: on }
end function

sub LayoutProfile(theme as object)
    space = SpacingScale()
    pad = PanelPad()
    column = Int((ContentWidth() - space.s5) / 2)
    inner = column - pad * 2

    profile = DrawProfileColumn(theme, inner, pad, pad)
    support = DrawSupportColumn(theme, inner, column + space.s5 + pad, pad)

    content = profile
    if support > content then content = support

    PaintPanel(m.profilePanel, theme, 0, column, content + pad * 2)
    PaintPanel(m.supportPanel, theme, column + space.s5, column, content + pad * 2)
    ShowSection("profileSection", content + pad * 2)

    ShowSection("playbackSection", DrawPanelBlock(theme, m.playbackPanel, "playback"))
    ShowSection("appearanceSection", DrawPanelBlock(theme, m.appearancePanel, "appearance"))
    ShowSection("serverSection", DrawPanelBlock(theme, m.serverPanel, "server"))
    ShowSection("sessionSection", DrawPanelBlock(theme, m.sessionPanel, "session"))
end sub

function DrawPanelBlock(theme as object, panel as object, which as string) as integer
    pad = PanelPad()
    inner = ContentWidth() - pad * 2

    if which = "playback"
        height = DrawPlayback(theme, inner, pad, pad)
    else if which = "appearance"
        height = DrawAppearance(theme, inner, pad, pad)
    else if which = "server"
        height = DrawServer(theme, inner, pad, pad)
    else
        height = DrawSession(theme, inner, pad, pad)
    end if

    PaintPanel(panel, theme, 0, ContentWidth(), height + pad * 2)
    return height + pad * 2
end function

function DrawBlockHeading(node as object, theme as object, text as string, width as integer, at as integer, top as integer) as integer
    sizes = TypeScale()

    node.text = text
    node.color = theme.text
    node.font = SizedBoldFont(sizes.textLg)
    node.width = width
    node.maxLines = 1
    node.translation = [at, top]

    return TextLinePitch(sizes.textLg)
end function

function DrawProfileColumn(theme as object, width as integer, at as integer, top as integer) as integer
    sizes = TypeScale()
    space = SpacingScale()

    offset = top + DrawBlockHeading(m.profileHeading, theme, Phrase("heading.profile"), width, at, top)

    m.profileFacts.theme = theme
    m.profileFacts.listWidth = width
    m.profileFacts.columns = 2
    m.profileFacts.rows = ProfileRows(m.user)
    m.profileFacts.translation = [at, offset]
    offset = offset + Int(m.profileFacts.listHeight) + space.s3

    m.profileHelp.text = Phrase("help.profileReadOnly")
    m.profileHelp.color = theme.muted
    m.profileHelp.font = SizedFont(sizes.textSm)
    m.profileHelp.width = width
    m.profileHelp.maxLines = 2
    m.profileHelp.wrap = true
    m.profileHelp.translation = [at, offset]

    return offset - top + TextLinePitch(sizes.textSm) * 2
end function

function DrawSupportColumn(theme as object, width as integer, at as integer, top as integer) as integer
    sizes = TypeScale()
    space = SpacingScale()

    offset = top + DrawBlockHeading(m.supportHeading, theme, Phrase("heading.playbackSupport"), width, at, top)

    m.supportChanged.text = ""
    if OverridesChanged(m.overrides) then m.supportChanged.text = Phrase("caps.changed")
    m.supportChanged.color = theme.danger
    m.supportChanged.font = SizedBoldFont(sizes.textSm)
    m.supportChanged.width = width
    m.supportChanged.maxLines = 1
    m.supportChanged.horizAlign = "right"
    m.supportChanged.translation = [at, top]

    m.supportFacts.theme = theme
    m.supportFacts.listWidth = width
    m.supportFacts.columns = 2
    m.supportFacts.rows = DiagnosticRows(DeviceDecoding(), Int(m.global.profileVersion), m.global.negotiated)
    m.supportFacts.translation = [at, offset]
    offset = offset + Int(m.supportFacts.listHeight) + space.s3

    m.supportActions.theme = theme
    m.supportActions.barWidth = width
    m.supportActions.buttons = [{ id: "change", label: Phrase("caps.change") }]
    m.supportActions.translation = [at, offset]

    return offset - top + BarHeight(m.supportActions)
end function

function DrawPlayback(theme as object, width as integer, at as integer, top as integer) as integer
    sizes = TypeScale()
    space = SpacingScale()

    offset = top + DrawBlockHeading(m.playbackHeading, theme, Phrase("heading.playback"), width, at, top)

    rows = PlaybackRows()
    for index = 0 to rows.Count() - 1
        row = rows[index]

        row.bar.theme = theme
        row.bar.barWidth = width
        row.bar.buttons = [{ id: row.id, label: row.value, speech: SettingRow(row.label, row.value) }]

        control = Int(row.bar.barSpan)
        controlHeight = BarHeight(row.bar)
        headHeight = TextLinePitch(sizes.textBase)
        if controlHeight > headHeight then headHeight = controlHeight

        row.name.text = row.label
        row.name.color = theme.text
        row.name.font = SizedBoldFont(sizes.textBase)
        row.name.width = width - control - space.s5
        row.name.height = headHeight
        row.name.maxLines = 1
        row.name.ellipsisText = "…"
        row.name.vertAlign = "center"
        row.name.translation = [at, offset]

        row.bar.translation = [at + width - control, offset + Int((headHeight - controlHeight) / 2)]
        offset = offset + headHeight + space.s1

        helpWidth = width - control - space.s5
        row.help.text = row.hint
        row.help.color = theme.muted
        row.help.font = SizedFont(sizes.textSm)
        row.help.width = helpWidth
        row.help.maxLines = 2
        row.help.wrap = true
        row.help.translation = [at, offset]
        offset = offset + TextBlockHeight(sizes.textSm, TextLineCount(row.hint, helpWidth, sizes.textSm))

        if index >= rows.Count() - 1 then exit for

        offset = offset + space.s4
        row.rule.width = width
        row.rule.height = 2
        row.rule.color = theme.border
        row.rule.translation = [at, offset]
        offset = offset + 2 + space.s4
    end for

    return offset - top
end function

function PlaybackBars() as object
    return [m.autoplayActions, m.timeDisplayActions, m.diagnosticsActions]
end function

function PlaybackRows() as object
    return [
        {
            name: m.autoplayName,
            bar: m.autoplayActions,
            help: m.autoplayHelp,
            rule: m.autoplayRule,
            id: "autoplay",
            label: Phrase("player.autoplayNext"),
            value: AutoplayLabel(ReadAutoplayNext(), ReadAutoplayDelay()),
            hint: Phrase("help.autoplayNext")
        },
        {
            name: m.timeDisplayName,
            bar: m.timeDisplayActions,
            help: m.timeDisplayHelp,
            rule: m.timeDisplayRule,
            id: "timeDisplay",
            label: Phrase("player.timeDisplay"),
            value: TimeDisplayLabel(ReadRemainingTime()),
            hint: Phrase("help.timeDisplay")
        },
        {
            name: m.diagnosticsName,
            bar: m.diagnosticsActions,
            help: m.diagnosticsHelp,
            rule: invalid,
            id: "diagnostics",
            label: Phrase("player.diagnostics"),
            value: OnOrOff(ReadDiagnostics()),
            hint: Phrase("help.diagnostics")
        }
    ]
end function

function DrawAppearance(theme as object, width as integer, at as integer, top as integer) as integer
    space = SpacingScale()

    offset = top + DrawBlockHeading(m.appearanceHeading, theme, Phrase("heading.appearance"), width, at, top)

    m.themeCards.theme = theme
    m.themeCards.rowWidth = width
    m.themeCards.selected = m.top.themeName
    m.themeCards.translation = [at, offset]
    offset = offset + Int(m.themeCards.rowHeight) + space.s4

    m.contrastBar.theme = theme
    m.contrastBar.barWidth = width
    m.contrastBar.buttons = [ToggleButton("contrast", Phrase("action.highContrast"), ReadHighContrast())]
    m.contrastBar.translation = [at, offset]

    return offset - top + BarHeight(m.contrastBar)
end function

function DrawServer(theme as object, width as integer, at as integer, top as integer) as integer
    sizes = TypeScale()
    space = SpacingScale()

    offset = top + DrawBlockHeading(m.serverHeading, theme, Phrase("heading.server"), width, at, top)

    m.serverAddress.text = SessionFor(m.global).serverUrl
    m.serverAddress.color = theme.muted
    m.serverAddress.font = SizedFont(sizes.textSm)
    m.serverAddress.width = width
    m.serverAddress.maxLines = 1
    m.serverAddress.ellipsisText = "…"
    m.serverAddress.translation = [at, offset]
    offset = offset + TextLinePitch(sizes.textSm) + space.s3

    m.serverActions.theme = theme
    m.serverActions.barWidth = width
    m.serverActions.buttons = [{ id: "changeServer", label: Phrase("action.changeServer") }]
    m.serverActions.translation = [at, offset]

    return offset - top + BarHeight(m.serverActions)
end function

function DrawSession(theme as object, width as integer, at as integer, top as integer) as integer
    sizes = TypeScale()
    space = SpacingScale()

    offset = top + DrawBlockHeading(m.sessionHeading, theme, Phrase("heading.session"), width, at, top)

    m.deviceLine.text = ReadDeviceName() + "  ·  " + DeviceResolutionName()
    m.deviceLine.color = theme.muted
    m.deviceLine.font = SizedFont(sizes.textSm)
    m.deviceLine.width = width
    m.deviceLine.maxLines = 1
    m.deviceLine.ellipsisText = "…"
    m.deviceLine.translation = [at, offset]
    offset = offset + TextLinePitch(sizes.textSm) + space.s3

    m.sessionActions.theme = theme
    m.sessionActions.barWidth = width
    m.sessionActions.buttons = [{ id: "signOut", label: Phrase("action.signOut") }]
    m.sessionActions.translation = [at, offset]

    return offset - top + BarHeight(m.sessionActions)
end function

sub Ask(request as object, callback as string)
    m.tasks = LiveRequests(m.tasks)
    m.tasks.Push(SendRequest(request, callback))
end sub

sub LoadAccount()
    session = SessionFor(m.global)

    Ask(SelfRequest(session.serverUrl, session.token), "onSelf")
    LoadRail("watchlist")
    LoadRail("favorites")
    LoadRail("history")
end sub

sub onSelf(event as object)
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return
    if not parsed.ok or type(parsed.json) <> "roAssociativeArray" then return

    m.user = parsed.json
    Layout()
end sub

sub LoadRail(kind as string)
    session = SessionFor(m.global)

    callbacks = { watchlist: "onWatchlistEntries", favorites: "onFavoriteEntries", history: "onHistoryEntries" }
    request = PersonalListRequest(session.serverUrl, session.token, session.userId, kind, 0, RailPageLimit())
    Ask(request, callbacks[kind])
end sub

sub onWatchlistEntries(event as object)
    RailEntriesArrived("watchlist", event, "onWatchlistCards")
end sub

sub onFavoriteEntries(event as object)
    RailEntriesArrived("favorites", event, "onFavoriteCards")
end sub

sub onHistoryEntries(event as object)
    RailEntriesArrived("history", event, "onHistoryCards")
end sub

sub RailEntriesArrived(kind as string, event as object, callback as string)
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return

    entries = []
    if parsed.ok
        entries = parsed.json
        if PersonalListDef(kind).paged then entries = ValueAt(parsed.json, "items", [])
    end if
    if type(entries) <> "roArray" then entries = []

    refs = FirstRefs(TitleRefsFrom(entries), RailPageLimit())
    m.railRefs[kind] = refs
    m.railEntries[kind] = EntriesByRef(entries)

    if refs.Count() = 0
        Layout()
        return
    end if

    session = SessionFor(m.global)
    Ask(TitleCardsRequest(session.serverUrl, session.token, refs), callback)
end sub

sub onWatchlistCards(event as object)
    RailCardsArrived("watchlist", event)
end sub

sub onFavoriteCards(event as object)
    RailCardsArrived("favorites", event)
end sub

sub onHistoryCards(event as object)
    RailCardsArrived("history", event)
end sub

sub RailCardsArrived(kind as string, event as object)
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return

    built = []
    if parsed.ok and type(parsed.json) = "roArray"
        for each json in parsed.json
            built.Push(CardFromJson(json, true))
        end for
    end if

    cards = CardsForRefs(built, m.railRefs[kind])
    entries = m.railEntries[kind]
    for each card in cards
        if kind = "watchlist" then card.watchlisted = true
        if kind = "favorites" then card.favorite = true

        LibraryCard(card, entries[TitleKey(card.refType, card.refId)])
    end for

    m.railCards[kind] = cards
    Layout()
    LoadRailStates(cards)
end sub

sub LoadRailStates(cards as object)
    if cards.Count() = 0 then return

    session = SessionFor(m.global)
    for each chunk in ChunkRefs(LeafRefs(cards), MaxBatchSize())
        Ask(StateBatchRequest(session.serverUrl, session.token, session.userId, chunk), "onRailStates")
    end for
end sub

sub onRailStates(event as object)
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if not parsed.ok or type(parsed.json) <> "roArray" then return

    for each rail in LibraryRails()
        ApplyStates(m.railCards[rail.kind], parsed.json, [])
    end for
    Layout()
end sub

sub onTab(event as object)
    id = TextOrBlank(event.GetData())

    if id = "signOut"
        RequestSignOut()
        return
    end if

    wanted = 0
    if id = "profile" then wanted = 1
    if wanted = m.tab then return

    m.tab = wanted
    m.sectionIndex = 0
    RefreshTabs()
    DrawTabRule(m.top.theme, ContentLeft(), Int(m.ruleTop))
    Layout()
end sub

sub onCardSelected(event as object)
    target = event.GetData()
    if target = invalid or target.Count() = 0 then return

    screen = ScreenForKind(ValueAt(target, "kind", ""))
    if IsBlank(screen) then return

    m.top.advanceTarget = target
    m.top.advance = screen
end sub

sub onSupportAction(event as object)
    if TextOrBlank(event.GetData()) <> "change" then return

    RequestOverrideMenu()
end sub

sub onPlaybackAction(event as object)
    id = TextOrBlank(event.GetData())

    if id = "autoplay"
        RequestPlaybackChoice("autoplay", Phrase("player.autoplayNext"), AutoplayOptions(), AutoplayValue(ReadAutoplayNext(), ReadAutoplayDelay()))
        return
    end if

    if id = "timeDisplay"
        RequestPlaybackChoice("timeDisplay", Phrase("player.timeDisplay"), TimeDisplayOptions(), TimeDisplayValue(ReadRemainingTime()))
        return
    end if

    if id = "diagnostics" then WriteDiagnostics(not ReadDiagnostics())
    Layout()
end sub

sub RequestPlaybackChoice(field as string, title as string, options as object, current as dynamic)
    m.playbackChoices = options

    labels = []
    selected = 0
    for index = 0 to options.Count() - 1
        labels.Push(options[index].label)
        if options[index].value = current then selected = index
    end for

    m.top.choiceRequest = { field: field, title: title, kind: "choice", options: labels, selected: selected }
end sub

function ChosenPlaybackValue(result as object) as integer
    return Int(m.playbackChoices[ClampInt(result.index, 0, m.playbackChoices.Count() - 1)].value)
end function

sub onThemePick(event as object)
    name = TextOrBlank(event.GetData())
    if IsBlank(name) then return

    m.top.themeRequest = name
end sub

sub onContrastToggle()
    m.top.themeRequest = ContrastRequest()
end sub

sub onServerAction(event as object)
    if TextOrBlank(event.GetData()) <> "changeServer" then return

    m.top.choiceRequest = {
        field: "changeServer",
        title: Phrase("action.changeServer"),
        message: Phrase("confirm.changeServer"),
        options: ConfirmOptions(Phrase("action.changeServer"), true),
        selected: 1
    }
end sub

sub onSessionAction(event as object)
    if TextOrBlank(event.GetData()) <> "signOut" then return

    RequestSignOut()
end sub

sub RequestSignOut()
    m.top.choiceRequest = {
        field: "signOut",
        title: Phrase("action.signOut"),
        message: Phrase("confirm.signOut"),
        options: ConfirmOptions(Phrase("action.signOut"), true),
        selected: 1
    }
end sub

function OverrideMenuRows() as object
    rows = [
        { id: "picture", label: Phrase("caps.picture"), detail: OptionLabel(PictureHeightOptions(), Int(ValueAt(m.overrides, "maxHeight", 0)).ToStr()) },
        { id: "frameRate", label: Phrase("caps.frameRate"), detail: OptionLabel(FrameRateOptions(), Int(ValueAt(m.overrides, "maxFrameRate", 0)).ToStr()) }
    ]

    for each codec in KnownVideoCodecs()
        rows.Push({ id: "codec." + codec, label: UCase(codec), detail: OptionLabel(OverrideCodecOptions(), OverrideFor(m.overrides, codec)) })
    end for

    rows.Push({ id: "hdr", label: Phrase("diagnostics.hdr"), detail: OptionLabel(OverrideHdrOptions(), TextOrBlank(ValueAt(m.overrides, "hdr", AutoOverride()))) })
    if OverridesChanged(m.overrides) then rows.Push({ id: "reset", label: Phrase("caps.reset"), icon: "icon-replay" })

    return rows
end function

sub RequestOverrideMenu()
    m.overrideRows = OverrideMenuRows()
    m.top.choiceRequest = { field: "overrideMenu", title: Phrase("heading.playback"), kind: "menu", options: MenuOptions(m.overrideRows), selected: 0 }
end sub

function OverrideOptionsFor(id as string) as object
    if id = "picture" then return PictureHeightOptions()
    if id = "frameRate" then return FrameRateOptions()
    if id = "hdr" then return OverrideHdrOptions()

    return OverrideCodecOptions()
end function

function OverrideValueFor(id as string) as string
    if id = "picture" then return Int(ValueAt(m.overrides, "maxHeight", 0)).ToStr()
    if id = "frameRate" then return Int(ValueAt(m.overrides, "maxFrameRate", 0)).ToStr()
    if id = "hdr" then return TextOrBlank(ValueAt(m.overrides, "hdr", AutoOverride()))

    return OverrideFor(m.overrides, Mid(id, 7))
end function

sub RequestOverrideChoice(id as string)
    m.overrideField = id
    m.overrideChoices = OverrideOptionsFor(id)
    current = OverrideValueFor(id)

    labels = []
    selected = 0
    for index = 0 to m.overrideChoices.Count() - 1
        labels.Push(m.overrideChoices[index].label)
        if m.overrideChoices[index].value = current then selected = index
    end for

    m.top.choiceRequest = { field: "overrideValue", title: OverrideTitleFor(id), kind: "choice", options: labels, selected: selected }
end sub

function OverrideTitleFor(id as string) as string
    if id = "picture" then return Phrase("caps.picture")
    if id = "frameRate" then return Phrase("caps.frameRate")
    if id = "hdr" then return Phrase("diagnostics.hdr")

    return UCase(Mid(id, 7))
end function

sub StoreOverride(id as string, value as string)
    if id = "picture"
        m.overrides.maxHeight = Int(Val(value))
    else if id = "frameRate"
        m.overrides.maxFrameRate = Int(Val(value))
    else if id = "hdr"
        m.overrides.hdr = value
    else
        codec = Mid(id, 7)
        if value = AutoOverride()
            m.overrides.codecs.Delete(codec)
        else
            m.overrides.codecs[codec] = value
        end if
    end if

    WriteCapabilityOverrides(m.overrides)
    Layout()
end sub

sub ReloadCardStates()
    id = TextOrBlank(ValueAt(m.menuCard, "id", ""))
    action = TextOrBlank(m.menuAction)

    dropped = ""
    if action = "watchlist" and ValueAt(m.menuCard, "watchlisted", false) = true then dropped = "watchlist"
    if action = "favorite" and ValueAt(m.menuCard, "favorite", false) = true then dropped = "favorites"
    if action = "removeHistory" then dropped = "history"

    if not IsBlank(dropped)
        m.railCards[dropped] = CardsWithout(m.railCards[dropped], id)
        Layout()
        return
    end if

    if action = "clearHistory"
        m.railCards.history = []
        Layout()
        return
    end if

    for each rail in LibraryRails()
        LoadRailStates(m.railCards[rail.kind])
    end for
end sub

sub onChoice()
    result = m.top.choiceResult
    if HandledCardMenuChoice(result) then return
    if result = invalid then return

    field = TextOrBlank(ValueAt(result, "field", ""))

    if ValueAt(result, "cancelled", false) = true
        if field = "overrideValue" then RequestOverrideMenu()
        return
    end if

    if field = "autoplay"
        if type(m.playbackChoices) <> "roArray" then return

        StoreAutoplay(ChosenPlaybackValue(result))
        Layout()
        return
    end if

    if field = "timeDisplay"
        if type(m.playbackChoices) <> "roArray" then return

        WriteRemainingTime(ChosenPlaybackValue(result) = 1)
        Layout()
        return
    end if

    if field = "overrideMenu"
        if type(m.overrideRows) <> "roArray" then return

        id = m.overrideRows[ClampInt(result.index, 0, m.overrideRows.Count() - 1)].id
        if id = "reset"
            m.overrides = CapabilityOverrides()
            WriteCapabilityOverrides(m.overrides)
            Layout()
            RequestOverrideMenu()
            return
        end if

        RequestOverrideChoice(id)
        return
    end if

    if field = "overrideValue"
        if type(m.overrideChoices) <> "roArray" then return

        StoreOverride(m.overrideField, m.overrideChoices[ClampInt(result.index, 0, m.overrideChoices.Count() - 1)].value)
        RequestOverrideMenu()
        return
    end if

    if field = "signOut" and ConfirmAccepted(result)
        LeaveServer(false)
        return
    end if

    if field = "changeServer" and ConfirmAccepted(result)
        LeaveServer(true)
    end if
end sub

sub LeaveServer(forgetServer as boolean)
    m.leavingServer = forgetServer

    session = SessionFor(m.global)
    deviceId = ReadDeviceId()

    if IsBlank(deviceId) or IsBlank(TextOrBlank(session.userId))
        FinishLeaving()
        return
    end if

    RaiseToast("ok", Phrase("form.signingOut"))
    Ask(RevokeDeviceRequest(session.serverUrl, session.token, session.userId, deviceId), "onDeviceRevoked")
end sub

sub onDeviceRevoked()
    FinishLeaving()
end sub

sub FinishLeaving()
    if m.leavingServer
        ClearSession()
    else
        ClearToken()
    end if

    m.top.advance = "reset:SetupScreen"
end sub

sub onFocus()
    if not m.top.hasFocus() then return

    if not m.visited
        m.visited = true
        m.tabs.SetFocus(true)
        return
    end if

    if not FocusSection(m.sectionIndex) then m.tabs.SetFocus(true)
end sub

function onKeyEvent(key as string, press as boolean) as boolean
    if not KeysAreOurs() then return false
    if not press then return false

    if m.tabs.isInFocusChain()
        if key = "up" then return FocusCrumbs()
        if key = "down" then return FocusSection(0)
        return false
    end if

    if CrumbsFocused() and key = "down"
        m.tabs.SetFocus(true)
        return true
    end if

    if key = "options"
        for each rail in LibraryRails()
            if rail.node.isInFocusChain() then return OpenedCardMenu(rail.node.focusedCard, rail.kind)
        end for
    end if

    if HandledNodeStep(key, PlaybackBars()) then return true
    if HandledNodeStep(key, [m.themeCards, m.contrastBar]) then return true

    if key = "up" and m.sectionIndex <= 0 and not CrumbsFocused()
        m.tabs.SetFocus(true)
        return true
    end if

    return HandledSectionKey(key)
end function
