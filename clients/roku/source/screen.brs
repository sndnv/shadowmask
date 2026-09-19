function SessionFor(shared as object) as object
    return {
        serverUrl: shared.serverUrl,
        token: shared.token,
        userId: shared.userId,
        username: shared.username,
        role: shared.role
    }
end function

function ContentLeft() as integer
    space = SpacingScale()
    return space.s6 + space.s5
end function

function ContentTop() as integer
    space = SpacingScale()
    return space.s6 + space.s5
end function

function ContentWidth() as integer
    return CanvasWidth() - ContentLeft() - SpacingScale().s6
end function

function ContentBottomPad() as integer
    return SpacingScale().s6
end function

sub PublishBackdropUrl(url as string)
    m.top.backdropUrl = url
    m.global.backdropUrl = url
end sub

sub PublishBackdrop(artwork as dynamic)
    PublishBackdropFrom([artwork])
end sub

sub PublishBackdropFrom(candidates as dynamic)
    PublishBackdropUrl(BackdropUrlFrom(m.global.serverUrl, candidates))
end sub

sub ShowOverviewDialog(title as string, text as string)
    if IsBlank(text) then return

    m.top.choiceRequest = {
        field: "overview",
        title: title,
        message: text,
        options: [Phrase("action.ok")],
        selected: 0
    }
end sub

sub RaiseToast(kind as string, message as string)
    if IsBlank(message) then return
    m.top.toastRequest = { kind: kind, message: message }
end sub

function AudioGuideOn() as boolean
    device = DeviceInfo()
    if device = invalid then return false

    return device.IsAudioGuideEnabled() = true
end function

sub Speak(text as dynamic)
    line = TextOrBlank(text)
    if IsBlank(line) then return
    if line = TextOrBlank(m.spokenLast) then return

    if m.audioGuide = invalid then m.audioGuide = CreateObject("roAudioGuide")
    if m.audioGuide = invalid then return

    m.spokenLast = line
    m.audioGuide.Say(line, true, false)
end sub

sub SpeakAgain(text as dynamic)
    m.spokenLast = ""
    Speak(text)
end sub

sub Announce(text as dynamic)
    line = TextOrBlank(text)
    if IsBlank(line) then return
    if line = TextOrBlank(m.announcedLast) then return

    if m.audioGuide = invalid then m.audioGuide = CreateObject("roAudioGuide")
    if m.audioGuide = invalid then return

    m.announcedLast = line
    m.audioGuide.Say(line, false, true)
end sub

sub RaiseActionFailed()
    RaiseToast("err", Phrase("toast.err.actionFailed"))
end sub

function HandledUnauthorised(top as object, status as integer) as boolean
    if not IsUnauthorised(status) then return false

    ClearToken()
    top.advance = "reset:SetupScreen"
    return true
end function

function CrumbHeight() as integer
    return LinkControlHeight() + SpacingScale().s2
end function

sub InitCrumbs()
    node = m.top.FindNode("crumbs")
    if node = invalid then return

    m.crumbEntries = []
    node.ObserveField("activated", "onCrumbActivated")
end sub

sub DrawCrumbs(theme as object, crumbs as dynamic, left as integer, top as integer, width as integer)
    node = m.top.FindNode("crumbs")
    if node = invalid then return

    m.crumbEntries = CrumbEntries(crumbs)

    node.theme = theme
    node.barWidth = width
    node.buttons = CrumbButtons(m.crumbEntries)
    node.translation = [left - LinkPadding(), top]

    if m.top.hasFocus() then Announce(CrumbTrailSpeech(m.crumbEntries))
end sub

sub onCrumbActivated(event as object)
    RunCrumb(TextOrBlank(event.GetData()))
end sub

sub RunCrumb(id as string)
    if Left(id, 6) <> "crumb:" then return
    if type(m.crumbEntries) <> "roArray" then return

    index = Int(Val(Mid(id, 7)))
    if index < 0 or index >= m.crumbEntries.Count() then return

    crumb = m.crumbEntries[index]
    screen = TextOrBlank(ValueAt(crumb, "screen", ""))
    if IsBlank(screen) then return

    target = ValueAt(crumb, "target", invalid)
    if target = invalid
        m.top.advance = "reset:" + screen
        return
    end if

    m.top.advanceTarget = target
    m.top.advance = screen
end sub

function CrumbsNode() as dynamic
    return m.top.FindNode("crumbs")
end function

function CrumbsFocused() as boolean
    node = CrumbsNode()
    if node = invalid then return false
    return node.isInFocusChain()
end function

function FocusCrumbs() as boolean
    node = CrumbsNode()
    if node = invalid or node.barHeight <= 0 then return false

    m.sectionOffset = 0
    if m.stack <> invalid then m.stack.translation = [0, 0]
    UpdateScrollBar()

    node.SetFocus(true)
    return true
end function

sub InitSections(specs as object)
    m.viewport = m.top.FindNode("viewport")
    m.stack = m.top.FindNode("stack")

    m.sections = []
    for each spec in specs
        id = TextOrBlank(ValueAt(spec, "id", ""))
        focusId = TextOrBlank(ValueAt(spec, "focus", id))
        node = m.top.FindNode(id)
        m.sections.Push({ id: id, node: node, focus: m.top.FindNode(focusId), height: 0, shown: false, focusable: false })
    end for

    m.sectionIndex = 0
    m.sectionOffset = 0
    m.sectionViewport = CanvasHeight()
    m.sectionTops = []
    m.sectionHeights = []
    m.sectionOrder = []
end sub

function SectionAt(id as string) as dynamic
    if type(m.sections) <> "roArray" then return invalid

    for each section in m.sections
        if section.id = id then return section
    end for
    return invalid
end function

sub ShowSection(id as string, height as integer, focusable = true as boolean)
    section = SectionAt(id)
    if section = invalid then return

    section.height = height
    section.shown = true
    section.focusable = focusable
    if section.node <> invalid then section.node.visible = true
end sub

sub HideSection(id as string)
    section = SectionAt(id)
    if section = invalid then return

    section.shown = false
    section.focusable = false
    if section.node <> invalid then section.node.visible = false
end sub

sub LayoutSections(viewportWidth as integer, viewportHeight as integer)
    gap = SpacingScale().s5

    m.sectionTops = []
    m.sectionHeights = []
    m.sectionOrder = []
    m.sectionViewport = viewportHeight

    offset = 0
    for each section in m.sections
        if section.shown
            if section.node <> invalid then section.node.translation = [0, offset]

            if section.focusable then m.sectionOrder.Push(m.sectionTops.Count())
            m.sectionTops.Push(offset)
            m.sectionHeights.Push(section.height)
            offset = offset + section.height + gap
        end if
    end for

    m.viewport.clippingRect = [0, 0, viewportWidth, viewportHeight]

    if m.sectionOrder.Count() = 0
        m.sectionOffset = 0
        m.stack.translation = [0, 0]
        UpdateScrollBar()
        return
    end if

    m.sectionIndex = ClampInt(m.sectionIndex, 0, m.sectionOrder.Count() - 1)
    ScrollSections()
end sub

sub ScrollSections()
    if m.sectionOrder.Count() = 0 then return

    at = m.sectionOrder[m.sectionIndex]
    m.sectionOffset = RevealOffset(at, m.sectionTops, m.sectionHeights, m.sectionViewport, m.sectionOffset)
    m.stack.translation = [0, - m.sectionOffset]
    UpdateScrollBar()
end sub

function SectionsTotalHeight() as integer
    if type(m.sectionTops) <> "roArray" or m.sectionTops.Count() = 0 then return 0

    last = m.sectionTops.Count() - 1
    return m.sectionTops[last] + m.sectionHeights[last] + ContentBottomPad()
end function

sub PlaceScrollBar(height as integer)
    bar = m.top.FindNode("scrollBar")
    if bar = invalid then return

    bar.trackHeight = CanvasHeight()
    bar.viewportHeight = height
    bar.translation = [CanvasWidth() - ScrollBarWidth(), 0]
end sub

sub UpdateScrollBar()
    bar = m.top.FindNode("scrollBar")
    if bar = invalid then return

    bar.theme = m.top.theme
    bar.viewportHeight = m.sectionViewport
    bar.contentHeight = SectionsTotalHeight()
    bar.offset = m.sectionOffset
end sub

function FocusSection(index as integer) as boolean
    if m.sectionOrder.Count() = 0 then return false

    m.sectionIndex = ClampInt(index, 0, m.sectionOrder.Count() - 1)
    ScrollSections()

    section = ShownSectionAt(m.sectionOrder[m.sectionIndex])
    if section = invalid or section.focus = invalid then return false

    section.focus.SetFocus(true)
    return true
end function

function ShownSectionAt(position as integer) as dynamic
    seen = 0
    for each section in m.sections
        if section.shown
            if seen = position then return section
            seen = seen + 1
        end if
    end for
    return invalid
end function

function GridRowCount(count as integer, columns as integer) as integer
    if count <= 0 or columns <= 0 then return 0
    return Int((count + columns - 1) / columns)
end function

function GridContentHeight(count as integer, columns as integer, cardHeight as integer, gap as integer) as integer
    rows = GridRowCount(count, columns)
    if rows = 0 then return 0
    return rows * cardHeight + (rows - 1) * gap
end function

sub UpdateRowsScrollBar(rows as dynamic)
    bar = m.top.FindNode("scrollBar")
    if bar = invalid or rows = invalid then return

    bar.theme = m.top.theme
    bar.viewportHeight = rows.rowsHeight
    bar.contentHeight = rows.contentHeight
    bar.offset = rows.scrollOffset
end sub

sub UpdateGridScrollBar(grid as dynamic, columns as integer, cardHeight as integer, viewport as integer)
    bar = m.top.FindNode("scrollBar")
    if bar = invalid or grid = invalid then return

    space = SpacingScale()
    content = GridContentHeight(grid.loaded, columns, cardHeight, space.s5)

    limit = content - viewport
    if limit < 0 then limit = 0

    at = 0
    if columns > 0 then at = Int(grid.focused / columns) * (cardHeight + space.s5)

    bar.theme = m.top.theme
    bar.viewportHeight = viewport
    bar.contentHeight = content
    bar.offset = ClampInt(at, 0, limit)
end sub

function KeysAreOurs() as boolean
    return m.top.isInFocusChain()
end function

sub TakeFocus(node as dynamic)
    if node = invalid then return
    if not m.top.isInFocusChain() then return

    node.SetFocus(true)
end sub

function ChipLabel(chip as dynamic) as string
    if type(chip) = "roAssociativeArray" then return TextOrBlank(ValueAt(chip, "label", ""))
    return TextOrBlank(chip)
end function

function ChipGlyph(chip as dynamic) as string
    if type(chip) <> "roAssociativeArray" then return ""
    return GlyphUri(TextOrBlank(ValueAt(chip, "icon", "")))
end function

function ChipOutlined(chip as dynamic) as boolean
    if type(chip) <> "roAssociativeArray" then return false
    return ValueAt(chip, "outline", false) = true
end function

function SpreadOffset(buttons as dynamic, barWidth as integer, natural as integer) as integer
    if type(buttons) <> "roArray" then return 0

    marks = 0
    for index = 1 to buttons.Count() - 1
        if ValueAt(buttons[index], "spaceBefore", false) = true then marks = marks + 1
    end for
    if marks = 0 or barWidth <= natural then return 0

    return Int((barWidth - natural) / marks)
end function

function HandledStepKey(key as string, bars as dynamic) as boolean
    if type(bars) <> "roArray" then return false
    if key <> "up" and key <> "down" then return false

    live = []
    for each bar in bars
        if bar <> invalid and bar.barHeight > 0 then live.Push(bar)
    end for
    if live.Count() < 2 then return false

    at = -1
    for index = 0 to live.Count() - 1
        if live[index].isInFocusChain() then at = index
    end for
    if at < 0 then return false

    if key = "up"
        if at <= 0 then return false
        live[at - 1].SetFocus(true)
        return true
    end if

    if at >= live.Count() - 1 then return false
    live[at + 1].SetFocus(true)
    return true
end function

function HandledNodeStep(key as string, nodes as dynamic) as boolean
    if type(nodes) <> "roArray" then return false
    if key <> "up" and key <> "down" then return false

    live = []
    for each node in nodes
        if node <> invalid and node.visible then live.Push(node)
    end for
    if live.Count() < 2 then return false

    at = -1
    for index = 0 to live.Count() - 1
        if live[index].isInFocusChain() then at = index
    end for
    if at < 0 then return false

    if key = "up"
        if at <= 0 then return false
        live[at - 1].SetFocus(true)
        return true
    end if

    if at >= live.Count() - 1 then return false

    live[at + 1].SetFocus(true)
    return true
end function

function HandledSectionKey(key as string) as boolean
    if m.sectionOrder.Count() = 0 then return false

    if key = "down"
        if CrumbsFocused() then return FocusSection(m.sectionIndex)
        if m.sectionIndex >= m.sectionOrder.Count() - 1 then return false
        return FocusSection(m.sectionIndex + 1)
    end if

    if key = "up"
        if CrumbsFocused() then return false
        if m.sectionIndex <= 0 then return FocusCrumbs()
        return FocusSection(m.sectionIndex - 1)
    end if

    return false
end function

function ScreenForKind(kind as dynamic) as string
    screens = {
        "movie": "TitleScreen",
        "series": "TitleScreen",
        "season": "SeasonScreen",
        "episode": "EpisodeScreen",
        "person": "PersonScreen",
        "collection": "CollectionScreen"
    }
    key = LCase(TextOrBlank(kind))
    if screens.DoesExist(key) then return screens[key]
    return ""
end function

function HeroPosterWidth() as integer
    return 300
end function

function HeroLandscapeWidth() as integer
    return 480
end function

function RetainedScreens() as integer
    return 2
end function

sub CollectBySubtype(root as dynamic, wanted as object, found as object)
    if root = invalid then return

    children = root.GetChildren(-1, 0)
    if type(children) <> "roArray" then return

    for each child in children
        for each name in wanted
            if child.subtype() = name
                found.Push(child)
                exit for
            end if
        end for
        CollectBySubtype(child, wanted, found)
    end for
end sub

function ContentHolders(screen as dynamic) as object
    found = []
    CollectBySubtype(screen, ["CardRail", "CardGrid"], found)
    return found
end function

function StateNoteMaxLines() as integer
    return 3
end function

function StateNoteHeight(size as integer, lines as integer) as integer
    if lines <= 1 then return Int(size * 1.6)
    return TextBlockHeight(size, lines) + Int(size * 0.6)
end function

function HeroArtWidth(aspect as string) as integer
    if aspect = LandscapeAspect() then return HeroLandscapeWidth()
    return HeroPosterWidth()
end function

function HeroArtHeight(aspect as string) as integer
    width = HeroArtWidth(aspect)
    if aspect = LandscapeAspect() then return Int(width * 9 / 16)
    return Int(width * 3 / 2)
end function

function RailCardWidth() as integer
    return 260
end function

function LandscapeCardWidth() as integer
    return 400
end function

function CastCardWidth() as integer
    return 170
end function

function CardWidthFor(aspect as dynamic) as integer
    if TextOrBlank(aspect) = LandscapeAspect() then return LandscapeCardWidth()
    return RailCardWidth()
end function

function GridCardWidth() as integer
    return RailCardWidth()
end function

function GridCaptionRows() as integer
    return 3
end function

function ColumnsThatFit(available as integer, preferredWidth as integer, gap as integer) as integer
    if preferredWidth <= 0 then return 1
    return ClampInt(Int((available + gap) / (preferredWidth + gap)), 1, 16)
end function

function RailViewportWidth(available as integer, cardWidth as integer, gap as integer) as integer
    if cardWidth <= 0 or available <= 0 then return available
    return GroupNaturalWidth(ColumnsThatFit(available, cardWidth, gap), cardWidth, gap)
end function

function RailOverflows(count as integer, viewport as integer, cardWidth as integer, gap as integer) as boolean
    if count <= 0 or viewport <= 0 or cardWidth <= 0 then return false

    return count > ColumnsThatFit(viewport, cardWidth, gap)
end function

function RailFocusRange(viewport as integer, cardWidth as integer) as object
    last = viewport - cardWidth
    if last <= 0 then return [0, 0]

    return [0, last]
end function

function FittedColumnWidth(available as integer, columns as integer, gap as integer) as integer
    if columns <= 0 then return available
    return Int((available - gap * (columns - 1)) / columns)
end function

function GridColumns() as integer
    return ColumnsThatFit(ContentWidth(), GridCardWidth(), SpacingScale().s4)
end function

function GridFittedCardWidth() as integer
    return FittedColumnWidth(ContentWidth(), GridColumns(), SpacingScale().s4)
end function
