sub init()
    m.slots = m.top.FindNode("slots")
    m.built = []
end sub

function CardPadding() as integer
    return SpacingScale().s3
end function

function SwatchSize() as integer
    return Int(TypeScale().textSm * 0.8)
end function

function PreviewHeight() as integer
    return SwatchSize() + CardPadding() * 2
end function

sub render()
    theme = m.top.theme
    if theme = invalid or theme.Count() = 0 then return

    sizes = TypeScale()
    space = SpacingScale()
    names = ThemeNames()

    Rebuild(names.Count())

    gap = space.s3
    width = Int((m.top.rowWidth - gap * (names.Count() - 1)) / names.Count())
    height = PreviewHeight() + CardPadding() * 2 + TextLinePitch(sizes.textSm)

    for index = 0 to names.Count() - 1
        PaintCard(m.built[index], names[index], theme, index, width, height, index * (width + gap))
    end for

    m.top.rowHeight = height

    if m.top.hasFocus()
        at = ClampInt(m.top.focusIndex, 0, names.Count() - 1)
        parts = [ThemeLabel(names[at]), PhraseWith("speech.slotOf", { index: at + 1, count: names.Count() })]
        if names[at] = m.top.selected then parts.Push(Phrase("speech.selected"))
        Speak(SpokenLine(parts))
    end if
end sub

sub PaintCard(parts as object, name as string, theme as object, index as integer, width as integer, height as integer, at as integer)
    sizes = TypeScale()
    preview = ThemeTokens(name)
    chosen = name = m.top.selected
    focused = index = m.top.focusIndex and m.top.hasFocus()

    parts.group.translation = [at, 0]

    parts.box.boxWidth = width
    parts.box.boxHeight = height
    parts.box.fillColor = theme.surfaceAlt
    parts.box.lineColor = theme.border
    if chosen then parts.box.lineColor = theme.accent

    parts.ring.visible = focused
    parts.ring.boxWidth = width + SpacingScale().s2
    parts.ring.boxHeight = height + SpacingScale().s2
    parts.ring.fillColor = "0x00000000"
    parts.ring.lineColor = theme.accent
    parts.ring.translation = [- Int(SpacingScale().s2 / 2), - Int(SpacingScale().s2 / 2)]

    inner = width - CardPadding() * 2

    parts.preview.boxWidth = inner
    parts.preview.boxHeight = PreviewHeight()
    parts.preview.fillColor = preview.bg
    parts.preview.lineColor = preview.border
    parts.preview.translation = [CardPadding(), CardPadding()]

    swatches = [preview.surface, preview.accent, preview.text]
    size = SwatchSize()
    for at2 = 0 to swatches.Count() - 1
        swatch = parts.swatches[at2]
        swatch.boxWidth = size
        swatch.boxHeight = size
        swatch.fillColor = swatches[at2]
        swatch.lineColor = preview.border
        swatch.translation = [CardPadding() * 2 + at2 * (size + CardPadding()), CardPadding() * 2]
    end for

    parts.label.text = ThemeLabel(name)
    parts.label.color = theme.text
    parts.label.font = SizedBoldFont(sizes.textSm)
    parts.label.width = inner
    parts.label.maxLines = 1
    parts.label.translation = [CardPadding(), CardPadding() * 2 + PreviewHeight()]

    parts.tick.uri = GlyphUri("icon-check")
    parts.tick.blendColor = theme.accent
    parts.tick.width = size
    parts.tick.height = size
    parts.tick.visible = chosen
    parts.tick.translation = [width - CardPadding() - size, CardPadding() * 2 + PreviewHeight()]
end sub

sub Rebuild(count as integer)
    if m.built.Count() = count then return

    while m.slots.GetChildCount() > 0
        m.slots.RemoveChildIndex(0)
    end while
    m.built = []

    for index = 0 to count - 1
        group = m.slots.CreateChild("Group")
        parts = {
            group: group,
            ring: group.CreateChild("RoundedBox"),
            box: group.CreateChild("RoundedBox"),
            preview: group.CreateChild("RoundedBox"),
            swatches: [],
            label: group.CreateChild("Label"),
            tick: group.CreateChild("Poster")
        }
        for swatch = 0 to 2
            parts.swatches.Push(group.CreateChild("RoundedBox"))
        end for
        m.built.Push(parts)
    end for
end sub

function onKeyEvent(key as string, press as boolean) as boolean
    if not KeysAreOurs() then return false
    if not press then return false

    names = ThemeNames()

    if key = "left"
        if m.top.focusIndex <= 0 then return false
        m.top.focusIndex = m.top.focusIndex - 1
        return true
    end if

    if key = "right"
        if m.top.focusIndex >= names.Count() - 1 then return true
        m.top.focusIndex = m.top.focusIndex + 1
        return true
    end if

    if key = "OK"
        m.top.activated = names[ClampInt(m.top.focusIndex, 0, names.Count() - 1)]
        return true
    end if

    return false
end function
