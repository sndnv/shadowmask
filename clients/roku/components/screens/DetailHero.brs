sub InitHero()
    m.hero = {
        art: m.top.FindNode("art"),
        poster: m.top.FindNode("poster"),
        glyph: m.top.FindNode("glyph"),
        heading: m.top.FindNode("heading"),
        facts: m.top.FindNode("facts"),
        readouts: m.top.FindNode("readouts"),
        genreChips: m.top.FindNode("genreChips"),
        ratingChips: m.top.FindNode("ratingChips"),
        crew: m.top.FindNode("crew"),
        crewLinks: m.top.FindNode("crewLinks"),
        navActions: m.top.FindNode("navActions"),
        actions: m.top.FindNode("actions"),
        actionLead: m.top.FindNode("actionLead"),
        actionNote: m.top.FindNode("actionNote"),
        overview: m.top.FindNode("overview"),
        overviewToggle: m.top.FindNode("overviewToggle")
    }
end sub

function HeroBars() as object
    return [m.hero.crewLinks, m.hero.navActions, m.hero.actions, m.hero.overviewToggle]
end function

function OverviewMaxLines() as integer
    return 2
end function

function StepButtons(neighbours as object) as object
    if neighbours.previous = invalid and neighbours.following = invalid then return []

    return [
        { id: "previous", icon: "chevron-left", iconOnly: true, disabled: neighbours.previous = invalid },
        { id: "following", icon: "chevron-right", iconOnly: true, disabled: neighbours.following = invalid }
    ]
end function

function DrawHero(theme as object, content as object, width as integer) as integer
    sizes = TypeScale()
    space = SpacingScale()

    aspect = TextOrBlank(ValueAt(content, "aspect", PosterAspect()))
    artWidth = HeroArtWidth(aspect)
    artHeight = HeroArtHeight(aspect)

    m.hero.art.width = artWidth
    m.hero.art.height = artHeight
    m.hero.art.color = theme.artBg
    m.hero.art.muteAudioGuide = true
    m.hero.art.translation = [0, 0]

    m.hero.poster.muteAudioGuide = true
    m.hero.glyph.muteAudioGuide = true

    artUri = TextOrBlank(ValueAt(content, "imageUri", ""))
    hasArt = not IsBlank(artUri)

    m.hero.poster.visible = hasArt
    m.hero.poster.loadDisplayMode = "scaleToZoom"
    m.hero.poster.uri = artUri
    m.hero.poster.width = artWidth
    m.hero.poster.height = artHeight
    m.hero.poster.translation = [0, 0]

    glyph = PlaceholderGlyphSize()
    m.hero.glyph.visible = not hasArt
    m.hero.glyph.uri = TextOrBlank(ValueAt(content, "placeholderUri", PosterPlaceholder()))
    m.hero.glyph.width = glyph
    m.hero.glyph.height = glyph
    m.hero.glyph.blendColor = theme.border
    m.hero.glyph.translation = [(artWidth - glyph) / 2, (artHeight - glyph) / 2]

    textLeft = artWidth + space.s6
    column = width - textLeft
    if column < Px(400) then column = Px(400)

    offset = HeroLine(m.hero.heading, ValueAt(content, "heading", ""), sizes.text2xl, theme.text, true, textLeft, column, 0, 1)
    offset = HeroFactRow(theme, content, textLeft, column, offset)
    offset = HeroLine(m.hero.crew, ValueAt(content, "crew", ""), sizes.textSm, theme.muted, false, textLeft, column, offset, 2)
    offset = HeroLinks(m.hero.crewLinks, theme, ValueAt(content, "crewButtons", invalid), textLeft, column, offset)
    offset = HeroChips(m.hero.genreChips, theme, ValueAt(content, "genreChips", invalid), textLeft, column, offset)
    offset = HeroChips(m.hero.ratingChips, theme, ValueAt(content, "ratingChips", invalid), textLeft, column, offset)

    if m.hero.navActions <> invalid and m.hero.navActions.barHeight > 0
        m.hero.navActions.translation = [textLeft, offset + space.s3]
        offset = offset + space.s3 + Int(m.hero.navActions.barHeight)
    end if

    if m.hero.actions <> invalid and m.hero.actions.barHeight > 0
        m.hero.actions.translation = [textLeft, offset + space.s3]
        offset = offset + space.s3 + Int(m.hero.actions.barHeight) + space.s3
    end if

    offset = HeroNote(theme, ValueAt(content, "actionNote", invalid), textLeft, column, offset)
    offset = HeroOverview(theme, content, textLeft, column, offset)

    Announce(HeroSpeech(content))

    if offset < artHeight then return artHeight
    return offset
end function

function HeroSpeech(content as dynamic) as string
    return SpokenLine([
        ValueAt(content, "heading", ""),
        ValueAt(content, "facts", ""),
        ValueAt(content, "crew", ""),
        ValueAt(content, "overview", "")
    ])
end function

function HeroOverview(theme as object, content as object, left as integer, width as integer, offset as integer) as integer
    node = m.hero.overview
    toggle = m.hero.overviewToggle
    text = TextOrBlank(ValueAt(content, "overview", ""))

    if node = invalid then return offset

    node.visible = not IsBlank(text)
    if IsBlank(text)
        if toggle <> invalid then toggle.buttons = []
        return offset
    end if

    space = SpacingScale()
    size = TypeScale().textSm
    lineHeight = FontLineHeight(size)
    clamp = OverviewMaxLines()
    gap = SpaceWidth(size)

    measured = MeasuredWords(text, size)
    words = measured.words
    widths = measured.widths

    whole = FittedLines(widths, gap, LineBudgets(width, clamp, 0))
    if whole.taken >= words.Count()
        if toggle <> invalid then toggle.buttons = []
        DrawOverviewText(node, theme, text, size, left, width, offset, OverviewLines(whole))
        return offset + TextBlockHeight(size, OverviewLines(whole)) + space.s2
    end if

    toggle.theme = theme
    toggle.barWidth = width
    toggle.buttons = [OverviewLink()]

    margin = OverviewMargin()
    inset = Int(toggle.barSpan) + gap

    fitted = FittedLines(widths, gap, LineBudgets(width - margin, clamp, inset))
    lines = OverviewLines(fitted)

    last = LineWords(words, fitted.counts, lines - 1)
    tail = WithoutTrailingMark(JoinWords(last, last.Count()))
    DrawOverviewText(node, theme, WithoutTrailingMark(JoinWords(words, fitted.taken)), size, left, width, offset, lines)

    top = offset + TextLinePitch(size) * (lines - 1) - Int((Int(toggle.barHeight) - lineHeight) / 2)
    toggle.translation = [left + TextWidth(tail, size) + gap - LinkPadding(), top]

    bottom = offset + TextBlockHeight(size, lines)
    if top + Int(toggle.barHeight) > bottom then bottom = top + Int(toggle.barHeight)

    return bottom + space.s2
end function

function OverviewMargin() as integer
    return SpacingScale().s3
end function

function OverviewLines(fitted as object) as integer
    if fitted.lines.Count() < 1 then return 1
    return fitted.lines.Count()
end function

function OverviewLink() as object
    return { id: "overview", label: Phrase("action.more"), style: "link", bold: false }
end function

sub DrawOverviewText(node as object, theme as object, text as string, size as integer, left as integer, width as integer, offset as integer, lines as integer)
    node.text = text
    node.color = theme.text
    node.font = SizedFont(size)
    node.width = width
    node.wrap = lines > 1
    node.lineSpacing = TextLineSpacing()
    node.maxLines = lines
    node.ellipsisText = "…"
    node.translation = [left, offset]
end sub

function HeroLine(node as dynamic, text as dynamic, size as integer, color as string, bold as boolean, left as integer, width as integer, offset as integer, maxLines as integer) as integer
    if node = invalid then return offset

    value = TextOrBlank(text)
    node.visible = not IsBlank(value)
    if IsBlank(value) then return offset

    node.text = value
    node.color = color
    if bold
        node.font = SizedBoldFont(size)
    else
        node.font = SizedFont(size)
    end if
    lines = 1
    if maxLines > 1 then lines = HeroLineCount(value, width, size, maxLines)

    node.width = width
    node.wrap = lines > 1
    node.lineSpacing = TextLineSpacing()
    node.maxLines = lines
    node.ellipsisText = "…"
    node.translation = [left, offset]

    return offset + TextBlockHeight(size, lines) + SpacingScale().s2
end function

function ReadoutSize() as integer
    return Px(77)
end function

function ReadoutLineWidth() as integer
    return Px(21)
end function

function ReadoutDotSize() as integer
    return Px(10)
end function

function HexApothemRatio() as float
    return 0.866
end function

function HeroFactRow(theme as object, content as object, left as integer, width as integer, offset as integer) as integer
    size = TypeScale().textBase
    space = SpacingScale()
    row = m.hero.readouts
    split = ReadoutSplit(ValueAt(content, "factList", invalid))

    shown = row <> invalid and IsCut(theme) and split.readouts.Count() > 0
    if row <> invalid then row.visible = shown
    if not shown then return HeroLine(m.hero.facts, ValueAt(content, "facts", ""), size, theme.muted, false, left, width, offset, 1)

    row.translation = [left, offset]
    used = DrawReadouts(row, theme, split.readouts)
    hex = ReadoutSize()
    rest = FactsText(split.rest)
    restLeft = left + used + space.s5

    if restLeft + TextWidth(rest, size) > left + width
        return HeroLine(m.hero.facts, rest, size, theme.muted, false, left, width, offset + hex + space.s3, 1)
    end if

    HeroLine(m.hero.facts, rest, size, theme.muted, false, restLeft, left + width - restLeft, offset + Int((hex - TextBlockHeight(size, 1)) / 2), 1)
    return offset + hex + space.s2
end function

function DrawReadouts(row as object, theme as object, readouts as object) as integer
    while row.GetChildCount() < readouts.Count()
        BuildReadout(row.CreateChild("Group"))
    end while

    gap = SpacingScale().s5
    used = 0
    for index = 0 to row.GetChildCount() - 1
        slot = row.GetChild(index)
        slot.visible = index < readouts.Count()
        if slot.visible
            if used > 0 then used = used + gap
            slot.translation = [used, 0]
            used = used + DrawReadout(slot, theme, readouts[index])
        end if
    end for
    return used
end function

sub BuildReadout(slot as object)
    for each kind in ["Poster", "Poster", "Label", "Rectangle", "Poster", "Label"]
        part = slot.CreateChild(kind)
        if kind = "Poster" then part.muteAudioGuide = true
    end for
end sub

function DrawReadout(slot as object, theme as object, fact as object) as integer
    space = SpacingScale()
    sizes = TypeScale()
    hex = ReadoutSize()
    edge = BorderThickness()
    inset = Int(edge / HexApothemRatio() + 0.5)
    line = ReadoutLineWidth()
    dot = ReadoutDotSize()
    unit = UCase(TextOrBlank(fact.label))

    outer = slot.GetChild(0)
    outer.uri = DiscUri(theme)
    outer.width = hex
    outer.height = hex
    outer.blendColor = theme.line

    inner = slot.GetChild(1)
    inner.uri = DiscUri(theme)
    inner.width = hex - inset * 2
    inner.height = hex - inset * 2
    inner.blendColor = theme.surface
    inner.translation = [inset, inset]

    value = slot.GetChild(2)
    value.text = TextOrBlank(fact.value)
    value.color = theme.text
    value.font = SizedBoldFont(sizes.textSm)
    value.width = hex
    value.height = hex
    value.maxLines = 1
    value.horizAlign = "center"
    value.vertAlign = "center"

    lineLeft = hex + space.s2
    rule = slot.GetChild(3)
    rule.width = line
    rule.height = edge
    rule.color = theme.line
    rule.translation = [lineLeft, Int((hex - edge) / 2)]

    mark = slot.GetChild(4)
    mark.uri = DiscUri(theme)
    mark.width = dot
    mark.height = dot
    mark.blendColor = ReadoutDotColor(theme)
    mark.translation = [lineLeft + line, Int((hex - dot) / 2)]

    unitLeft = lineLeft + line + dot + space.s1
    caption = slot.GetChild(5)
    caption.text = unit
    caption.color = theme.accent
    caption.font = SizedFont(sizes.textXs)
    caption.maxLines = 1
    caption.translation = [unitLeft, Int((hex - TextBlockHeight(sizes.textXs, 1)) / 2)]

    return unitLeft + TextWidth(unit, sizes.textXs)
end function

function HeroNote(theme as object, note as dynamic, left as integer, width as integer, offset as integer) as integer
    lead = m.hero.actionLead
    node = m.hero.actionNote
    if lead = invalid or node = invalid then return offset

    shown = type(note) = "roAssociativeArray"
    lead.visible = shown
    node.visible = shown
    if not shown then return offset

    size = TypeScale().textSm
    leadText = TextOrBlank(ValueAt(note, "lead", ""))
    leadWidth = 0
    if not IsBlank(leadText) then leadWidth = TextWidth(leadText, size, false) + SpacingScale().s1

    lead.text = leadText
    lead.color = theme.muted
    lead.font = SizedFont(size)
    lead.maxLines = 1
    lead.translation = [left, offset]

    node.text = TextOrBlank(ValueAt(note, "episode", ""))
    node.color = theme.muted
    node.font = SizedBoldFont(size)
    node.width = width - leadWidth
    node.maxLines = 1
    node.ellipsisText = "…"
    node.translation = [left + leadWidth, offset]

    return offset + TextBlockHeight(size, 1) + SpacingScale().s2
end function

function HeroChips(node as dynamic, theme as object, chips as dynamic, left as integer, width as integer, offset as integer) as integer
    if node = invalid then return offset

    empty = type(chips) <> "roArray" or chips.Count() = 0
    node.visible = not empty
    if empty then return offset

    node.theme = theme
    node.rowWidth = width
    node.chips = chips
    node.translation = [left, offset]

    return offset + Int(node.rowHeight) + SpacingScale().s2
end function

function HeroLinks(node as dynamic, theme as object, buttons as dynamic, left as integer, width as integer, offset as integer) as integer
    if node = invalid then return offset

    empty = type(buttons) <> "roArray" or buttons.Count() = 0
    node.visible = not empty
    if empty then return offset

    node.theme = theme
    node.barWidth = width
    node.buttons = buttons
    node.translation = [left - LinkPadding(), offset]

    return offset + Int(node.barHeight) + SpacingScale().s2
end function

function HeroLineCount(text as string, width as integer, size as integer, maxLines as integer) as integer
    if maxLines <= 1 then return 1
    return ClampInt(TextLineCount(text, width, size), 1, maxLines)
end function

function SectionHeadingHeight(theme = invalid as dynamic) as integer
    return ThemedHeadingHeight(theme, Int(TypeScale().textXl * 1.35)) + SpacingScale().s3
end function

sub DrawSectionTab(node as dynamic, theme as object, noun as string, count as integer, width as integer)
    if node = invalid then return

    node.size = TypeScale().textXl
    node.headingWidth = width
    node.count = count
    node.text = noun
    node.theme = theme
    node.translation = [0, 0]
end sub

sub DrawSectionHeading(node as dynamic, theme as object, text as string, width as integer)
    if node = invalid then return

    node.text = text
    node.color = theme.text
    node.font = SizedBoldFont(TypeScale().textXl)
    node.width = width
    node.maxLines = 1
    node.ellipsisText = "…"
    node.translation = [0, 0]
end sub
