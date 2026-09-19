sub InitHero()
    m.hero = {
        art: m.top.FindNode("art"),
        poster: m.top.FindNode("poster"),
        glyph: m.top.FindNode("glyph"),
        heading: m.top.FindNode("heading"),
        facts: m.top.FindNode("facts"),
        genreChips: m.top.FindNode("genreChips"),
        ratingChips: m.top.FindNode("ratingChips"),
        crew: m.top.FindNode("crew"),
        crewLinks: m.top.FindNode("crewLinks"),
        navActions: m.top.FindNode("navActions"),
        actions: m.top.FindNode("actions"),
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
    m.hero.poster.uri = artUri
    m.hero.poster.width = artWidth
    m.hero.poster.height = artHeight
    m.hero.poster.loadDisplayMode = "scaleToFill"
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
    if column < 400 then column = 400

    offset = HeroLine(m.hero.heading, ValueAt(content, "heading", ""), sizes.text2xl, theme.text, true, textLeft, column, 0, 1)
    offset = HeroLine(m.hero.facts, ValueAt(content, "facts", ""), sizes.textBase, theme.muted, false, textLeft, column, offset, 1)
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

function SectionHeadingHeight() as integer
    return Int(TypeScale().textXl * 1.35) + SpacingScale().s3
end function

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
