sub init()
    m.chips = m.top.FindNode("chips")
    m.built = []
end sub

sub render()
    theme = m.top.theme
    chips = m.top.buttons
    if theme = invalid or theme.Count() = 0 then return
    if type(chips) <> "roArray" or chips.Count() = 0 then return

    sizes = TypeScale()
    space = SpacingScale()

    height = Int(sizes.textBase * 1.9)
    width = FittedColumnWidth(Int(m.top.barWidth), chips.Count(), space.s3)
    glyphSize = LinkIconSize()

    Rebuild(chips.Count())

    for index = 0 to m.built.Count() - 1
        chip = m.built[index]
        focused = index = m.top.focusIndex and m.top.hasFocus()
        label = ChipLabel(chips[index])
        glyph = ChipGlyph(chips[index])
        outline = ChipOutlined(chips[index])

        chip.group.translation = [index * (width + space.s3), 0]

        chip.fill.width = width
        chip.fill.height = height
        chip.fill.color = ChipFill(theme, focused, outline)

        chip.ring.theme = theme
        chip.ring.fill = ""
        chip.ring.lineColor = ChipLine(theme, focused)
        chip.ring.ringWidth = width
        chip.ring.ringHeight = height
        chip.ring.thickness = BorderThickness()
        chip.ring.visible = focused or outline

        lead = 0
        if not IsBlank(glyph) then lead = glyphSize + space.s2

        inner = width - space.s3 * 2 - lead
        shown = ClampInt(TextWidth(label, sizes.textSm), 0, inner)
        origin = Int((width - lead - shown) / 2)

        chip.icon.visible = not IsBlank(glyph)
        if chip.icon.visible
            chip.icon.uri = glyph
            chip.icon.width = glyphSize
            chip.icon.height = glyphSize
            chip.icon.blendColor = ChipText(theme, focused)
            chip.icon.translation = [origin, Int((height - glyphSize) / 2)]
        end if

        chip.label.text = label
        chip.label.color = ChipText(theme, focused)
        chip.label.font = SizedFont(sizes.textSm)
        chip.label.width = inner
        chip.label.height = height
        chip.label.maxLines = 1
        chip.label.ellipsisText = "…"
        chip.label.horizAlign = "left"
        chip.label.vertAlign = "center"
        chip.label.translation = [origin + lead, 0]
    end for

    m.top.barHeight = height

    if m.top.hasFocus()
        at = ClampInt(m.top.focusIndex, 0, chips.Count() - 1)
        Speak(SlotSpeech(ChipLabel(chips[at]), at, chips.Count()))
    end if
end sub

function ChipFill(theme as object, focused as boolean, outline as boolean) as string
    if focused then return theme.accent
    if outline then return "0x00000000"
    return theme.surfaceAlt
end function

function ChipLine(theme as object, focused as boolean) as string
    if focused then return theme.accent
    return theme.border
end function

function ChipText(theme as object, focused as boolean) as string
    if focused then return theme.accentContrast
    return theme.text
end function

sub Rebuild(count as integer)
    if m.built.Count() = count then return

    while m.chips.GetChildCount() > 0
        m.chips.RemoveChildIndex(0)
    end while
    m.built = []

    for index = 0 to count - 1
        group = m.chips.CreateChild("Group")
        fill = group.CreateChild("Rectangle")
        ring = group.CreateChild("FocusRing")
        icon = group.CreateChild("Poster")
        icon.muteAudioGuide = true

        m.built.Push({
            group: group,
            fill: fill,
            ring: ring,
            icon: icon,
            label: group.CreateChild("Label")
        })
    end for
end sub

sub onFocusChanged()
    render()
end sub

function onKeyEvent(key as string, press as boolean) as boolean
    if not KeysAreOurs() then return false
    if not press then return false

    labels = m.top.buttons
    if type(labels) <> "roArray" or labels.Count() = 0 then return false

    if key = "left"
        if m.top.focusIndex <= 0 then return false
        m.top.focusIndex = m.top.focusIndex - 1
        return true
    end if

    if key = "right"
        if m.top.focusIndex >= labels.Count() - 1 then return true
        m.top.focusIndex = m.top.focusIndex + 1
        return true
    end if

    if key = "OK"
        m.top.activated = m.top.focusIndex
        return true
    end if

    return false
end function
