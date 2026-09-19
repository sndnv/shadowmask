sub init()
    m.slots = m.top.FindNode("slots")
    m.built = []
end sub

sub render()
    theme = m.top.theme
    chips = m.top.chips
    if theme = invalid or theme.Count() = 0 then return
    if type(chips) <> "roArray" then return

    if chips.Count() = 0
        Rebuild(0)
        m.top.rowHeight = 0
        return
    end if

    sizes = TypeScale()
    space = SpacingScale()

    height = ChipHeight()
    capWidth = Int(height / 2)
    size = sizes.textSm
    line = BorderThickness()

    Rebuild(chips.Count())

    left = 0
    top = 0
    rows = 1

    for index = 0 to m.built.Count() - 1
        slot = m.built[index]
        chip = chips[index]

        label = TextOrBlank(ValueAt(chip, "label", ""))
        value = TextOrBlank(ValueAt(chip, "value", ""))

        labelWidth = TextWidth(label, size)
        valueWidth = TextWidth(value, size, true)

        inner = labelWidth + valueWidth
        if not IsBlank(value) and not IsBlank(label) then inner = inner + space.s1

        width = capWidth * 2 + inner
        if left > 0 and left + width > m.top.rowWidth
            left = 0
            top = top + height + space.s2
            rows = rows + 1
        end if

        slot.group.translation = [left, top]

        fill = theme.surfaceAlt
        stroke = ChipBorderColor(theme, ValueAt(chip, "border", ""))

        DrawChipCap(slot.capLeft, ChipCapUri("left"), fill, capWidth, height, 0)
        DrawChipCap(slot.capLeftLine, ChipCapLineUri("left"), stroke, capWidth, height, 0)
        DrawChipCap(slot.capRight, ChipCapUri("right"), fill, capWidth, height, width - capWidth)
        DrawChipCap(slot.capRightLine, ChipCapLineUri("right"), stroke, capWidth, height, width - capWidth)

        slot.middle.width = inner
        slot.middle.height = height
        slot.middle.color = fill
        slot.middle.translation = [capWidth, 0]

        slot.topLine.width = inner
        slot.topLine.height = line
        slot.topLine.color = stroke
        slot.topLine.translation = [capWidth, 0]

        slot.bottomLine.width = inner
        slot.bottomLine.height = line
        slot.bottomLine.color = stroke
        slot.bottomLine.translation = [capWidth, height - line]

        cursor = capWidth
        cursor = cursor + ChipText(slot.label, label, size, ChipTextColor(theme, ValueAt(chip, "tone", "")), false, labelWidth, height, cursor)
        if not IsBlank(label) then cursor = cursor + space.s1
        ChipText(slot.value, value, size, theme.text, true, valueWidth, height, cursor)

        left = left + width + space.s2
    end for

    m.top.rowHeight = rows * height + (rows - 1) * space.s2
end sub

sub DrawChipCap(node as object, uri as string, color as string, width as integer, height as integer, left as integer)
    node.uri = uri
    node.width = width
    node.height = height
    node.blendColor = color
    node.translation = [left, 0]
end sub

function ChipText(node as object, text as string, size as integer, color as string, bold as boolean, width as integer, height as integer, left as integer) as integer
    node.visible = not IsBlank(text)
    if IsBlank(text) then return 0

    node.text = text
    node.color = color
    if bold
        node.font = SizedBoldFont(size)
    else
        node.font = SizedFont(size)
    end if
    node.width = width + SpacingScale().s1
    node.height = height
    node.maxLines = 1
    node.vertAlign = "center"
    node.translation = [left, 0]

    return width
end function

function ChipHeight() as integer
    return Int(TypeScale().textSm * 1.8)
end function

function ChipBorderColor(theme as object, border as dynamic) as string
    if TextOrBlank(border) = "accent" then return theme.accent
    return theme.border
end function

function ChipTextColor(theme as object, tone as dynamic) as string
    name = TextOrBlank(tone)
    if name = "warn" then return theme.warn
    if name = "danger" then return theme.danger
    if name = "ok" then return theme.ok
    return theme.text
end function

sub Rebuild(count as integer)
    if m.built.Count() = count then return

    while m.slots.GetChildCount() > 0
        m.slots.RemoveChildIndex(0)
    end while
    m.built = []

    for index = 0 to count - 1
        group = m.slots.CreateChild("Group")
        m.built.Push({
            group: group,
            capLeft: group.CreateChild("Poster"),
            middle: group.CreateChild("Rectangle"),
            capRight: group.CreateChild("Poster"),
            topLine: group.CreateChild("Rectangle"),
            bottomLine: group.CreateChild("Rectangle"),
            capLeftLine: group.CreateChild("Poster"),
            capRightLine: group.CreateChild("Poster"),
            label: group.CreateChild("Label"),
            value: group.CreateChild("Label")
        })
    end for
end sub
