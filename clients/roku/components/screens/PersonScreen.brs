sub init()
    InitHero()
    InitCrumbs()
    InitLayoutTick()

    m.filmography = m.top.FindNode("filmography")
    m.filmographyHeading = m.top.FindNode("filmographyHeading")
    m.filmographyNote = m.top.FindNode("filmographyNote")
    m.note = m.top.FindNode("note")

    m.overviewToggle = m.hero.overviewToggle
    m.overviewToggle.ObserveField("activated", "onAction")
    m.filmography.ObserveField("selected", "onTitleSelected")

    InitSections([{ id: "hero", focus: "overviewToggle" }, { id: "filmographySection", focus: "filmography" }])

    m.id = ""
    m.person = invalid
    m.cards = []
    m.tasks = []
    m.filled = false
    m.released = false
    m.started = false
    m.published = false
end sub

sub render()
    theme = m.top.theme
    target = m.top.target
    if theme = invalid or theme.Count() = 0 then return
    if target = invalid or target.Count() = 0 then return

    if IsBlank(m.id) then m.id = TextOrBlank(ValueAt(target, "id", ""))

    Layout()

    if not m.started
        m.started = true
        Load()
    end if
end sub

sub Layout()
    theme = m.top.theme
    space = SpacingScale()
    left = ContentLeft()
    top = ContentTop()

    width = CanvasWidth() - left - space.s6

    DrawCrumbs(theme, [{ label: Phrase("nav.people") }, { label: TitleText() }], left, top, width)
    top = top + CrumbHeight()
    height = CanvasHeight() - top - space.s5

    m.viewport.translation = [left, top]

    m.note.theme = theme
    m.note.fontSize = TypeScale().textBase
    m.note.noteWidth = width
    m.note.translation = [left, top]

    heroHeight = DrawHero(theme, HeroContent(), width)
    ShowSection("hero", heroHeight)
    LayoutFilmography(theme, width)
    PlaceScrollBar(height)
    LayoutSections(width, height)
end sub

function HeroContent() as object
    return {
        aspect: PosterAspect(),
        imageUri: HeroImageUri(),
        placeholderUri: PersonPlaceholder(),
        heading: TitleText(),
        facts: LabelledFactsText(PersonFacts(m.person)),
        crew: AlsoKnownAsText(),
        overview: BiographyText()
    }
end function

function HeroImageUri() as string
    if m.person <> invalid
        card = { aspect: PosterAspect(), artwork: ValueAt(m.person, "artwork", invalid) }
        url = CardImageUrl(SessionFor(m.global).serverUrl, card, HeroArtWidth(PosterAspect()))
        if not IsBlank(url) then return url
    end if
    return TextOrBlank(ValueAt(m.top.target, "imageUri", ""))
end function

function TitleText() as string
    name = TextOrBlank(ValueAt(m.person, "name", ""))
    if not IsBlank(name) then return name
    return TextOrBlank(ValueAt(m.top.target, "title", ""))
end function

function AlsoKnownAsText() as string
    names = ValueAt(m.person, "also_known_as", invalid)
    if type(names) <> "roArray" or names.Count() = 0 then return ""

    kept = []
    for each name in names
        text = TextOrBlank(name)
        if not IsBlank(text) and kept.Count() < 3 then kept.Push(text)
    end for

    if kept.Count() = 0 then return ""
    return kept.Join(", ")
end function

function BiographyText() as string
    biography = TextOrBlank(ValueAt(m.person, "biography", ""))
    if not IsBlank(biography) then return biography
    if m.person = invalid then return ""
    return Phrase("empty.noBiography")
end function

sub LayoutFilmography(theme as object, width as integer)
    if m.person = invalid
        HideSection("filmographySection")
        return
    end if

    space = SpacingScale()
    headingHeight = SectionHeadingHeight()
    DrawSectionHeading(m.filmographyHeading, theme, CountLabel(Phrase("heading.filmography"), m.cards.Count()), width)

    if m.cards.Count() = 0
        m.filmography.visible = false
        m.filmographyNote.visible = true
        m.filmographyNote.theme = theme
        m.filmographyNote.fontSize = TypeScale().textBase
        m.filmographyNote.noteWidth = width
        m.filmographyNote.message = Phrase("empty.noFilmography")
        m.filmographyNote.translation = [0, headingHeight]

        ShowSection("filmographySection", headingHeight + Int(TypeScale().textBase * 1.6), false)
        return
    end if

    columns = ColumnsThatFit(width, GridCardWidth(), space.s4)
    cardWidth = FittedColumnWidth(width, columns, space.s4)
    metrics = CardMetrics(cardWidth, PosterAspect(), GridCaptionRows())
    gridHeight = GridContentHeight(m.cards.Count(), columns, metrics.height, space.s5)

    m.filmographyNote.visible = false
    m.filmography.visible = true
    m.filmography.theme = theme
    m.filmography.serverUrl = SessionFor(m.global).serverUrl
    m.filmography.cardWidth = cardWidth
    m.filmography.columns = columns
    m.filmography.captionRows = GridCaptionRows()
    m.filmography.gridHeight = gridHeight
    m.filmography.translation = [0, headingHeight]

    if not m.filled
        m.filled = true
        m.filmography.cards = m.cards
    end if

    ShowSection("filmographySection", headingHeight + gridHeight)
end sub

sub Load()
    session = SessionFor(m.global)
    ShowNote(Phrase("state.loading"), "loading")

    Ask(PersonRequest(session.serverUrl, session.token, m.id), "onPerson")
end sub

function Ask(request as object, callback as string) as object
    task = SendRequest(request, callback)
    m.tasks.Push(task)
    return task
end function

function Answered(event as object) as object
    if m.released then return { ok: false, json: invalid, status: 0 }

    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return { ok: false, json: invalid, status: parsed.status }
    return parsed
end function

sub onPerson(event as object)
    parsed = Answered(event)
    if m.released then return

    if not parsed.ok
        ShowNote(Phrase("error.couldNotLoadPerson"), "error")
        return
    end if

    m.person = parsed.json
    m.cards = CardsFrom(ValueAt(m.person, "filmography", invalid), FilmographyCard)

    PublishBackdropUrl("")
    LoadStates()
    Refresh()
end sub

sub LoadStates()
    session = SessionFor(m.global)

    leaves = LeafRefs(m.cards)
    rollups = RollupTargets(m.cards)
    m.pendingStates = 0

    if leaves.Count() > 0
        m.pendingStates = m.pendingStates + 1
        Ask(StateBatchRequest(session.serverUrl, session.token, session.userId, leaves), "onStates")
    end if
    if rollups.Count() > 0
        m.pendingStates = m.pendingStates + 1
        Ask(StateRollupRequest(session.serverUrl, session.token, session.userId, rollups), "onRollups")
    end if

    m.states = []
    m.rollups = []
end sub

sub onStates(event as object)
    parsed = Answered(event)
    if m.released then return

    if parsed.ok and type(parsed.json) = "roArray" then m.states.Append(parsed.json)
    StateArrived()
end sub

sub onRollups(event as object)
    parsed = Answered(event)
    if m.released then return

    if parsed.ok and type(parsed.json) = "roArray" then m.rollups.Append(parsed.json)
    StateArrived()
end sub

sub StateArrived()
    m.pendingStates = m.pendingStates - 1
    if m.pendingStates > 0 then return

    ApplyStates(m.cards, m.states, m.rollups)
    m.filmography.cardStates = CardStateList(m.cards)
end sub

sub Refresh()
    CoalesceLayout()
end sub

sub PaintScreen()
    if m.person = invalid then return

    ShowNote("", "loading")
    m.viewport.visible = true
    Layout()

    if not m.published
        m.published = true
        FocusSection(0)
    end if
end sub

sub ShowNote(message as string, kind as string)
    m.note.kind = kind
    m.note.message = message
    m.note.visible = not IsBlank(message)
    if not IsBlank(message) then m.viewport.visible = false
end sub

sub onTitleSelected(event as object)
    target = event.GetData()
    if target = invalid or target.Count() = 0 then return

    screen = ScreenForKind(ValueAt(target, "kind", ""))
    if IsBlank(screen) then return

    m.top.advanceTarget = target
    m.top.advance = screen
end sub

sub onRelease()
    if not m.top.release then return

    m.released = true
    for each task in m.tasks
        if task <> invalid then task.control = "STOP"
    end for
    m.tasks = []
end sub

sub onFocus()
    if not m.top.hasFocus() then return
    if m.published then FocusSection(m.sectionIndex)
end sub

sub onAction(event as object)
    if TextOrBlank(event.GetData()) <> "overview" then return

    ShowOverviewDialog(TitleText(), BiographyText())
end sub

sub onChoice()
    HandledCardMenuChoice(m.top.choiceResult)
end sub

sub ReloadCardStates()
    LoadStates()
end sub

function onKeyEvent(key as string, press as boolean) as boolean
    if not KeysAreOurs() then return false
    if not press then return false
    if not m.published then return false

    if key = "options" and m.filmography.isInFocusChain()
        return OpenedCardMenu(m.filmography.focusedCard)
    end if

    return HandledSectionKey(key)
end function
