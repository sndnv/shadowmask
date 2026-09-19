sub init()
    m.slots = m.top.FindNode("slots")
    m.built = []
end sub

sub render()
    theme = m.top.theme
    buttons = m.top.buttons
    if theme = invalid or theme.Count() = 0 then return
    if type(buttons) <> "roArray" then return

    if buttons.Count() = 0
        Rebuild(0)
        m.top.barHeight = 0
        m.top.barSpan = 0
        return
    end if

    wanted = NearestEnabled(buttons, m.top.focusIndex)
    if wanted <> m.top.focusIndex
        m.top.focusIndex = wanted
        return
    end if

    Rebuild(buttons.Count())

    specs = []
    tallest = 0
    for index = 0 to buttons.Count() - 1
        spec = SlotSpec(buttons, index, theme)
        specs.Push(spec)
        if spec.height > tallest then tallest = spec.height
    end for

    space = SpacingScale()
    natural = 0
    for index = 0 to specs.Count() - 1
        natural = natural + specs[index].width + SlotGap(buttons, index, space)
    end for
    spread = SpreadOffset(buttons, Int(m.top.barWidth), natural)

    offset = 0
    spans = []
    for index = 0 to specs.Count() - 1
        if index > 0 and ValueAt(buttons[index], "spaceBefore", false) = true then offset = offset + spread
        PlaceSlot(m.built[index], specs[index], tallest, offset)
        spans.Push({ left: offset, width: specs[index].width })
        offset = offset + specs[index].width + SlotGap(buttons, index, space)
    end for

    m.top.barHeight = tallest
    m.top.barSpan = offset
    m.top.slotSpans = spans

    AnnounceFocused(buttons)
end sub

sub AnnounceFocused(buttons as object)
    if not m.top.hasFocus() then return

    at = ClampInt(m.top.focusIndex, 0, buttons.Count() - 1)
    button = buttons[at]

    spoken = TextOrBlank(ValueAt(button, "speech", ""))
    if IsBlank(spoken) then spoken = TextOrBlank(ValueAt(button, "label", ""))

    Speak(SlotSpeech(spoken, at, buttons.Count(), ValueAt(button, "disabled", false) = true))
end sub

function SlotSpec(buttons as object, index as integer, theme as object) as object
    sizes = TypeScale()
    space = SpacingScale()
    button = buttons[index]

    kind = TextOrBlank(ValueAt(button, "style", ""))
    tone = {
        primary: kind = "primary",
        joined: kind = "joined",
        link: kind = "link",
        glyph: kind = "glyph",
        danger: ValueAt(button, "danger", false) = true,
        pressed: ValueAt(button, "pressed", false) = true,
        disabled: ValueAt(button, "disabled", false) = true,
        focused: false
    }
    tone.focused = index = m.top.focusIndex and m.top.hasFocus() and not tone.disabled

    label = TextOrBlank(ValueAt(button, "label", ""))
    glyph = IconFor(button, tone.pressed)
    icon = ControlIconSize()

    compact = kind = "compact" or ValueAt(button, "iconOnly", false) = true
    if tone.joined and not IsBlank(glyph) then compact = true
    if ValueAt(button, "collapsible", false) = true and not tone.focused then compact = true

    height = ToggleControlHeight()
    size = sizes.textSm
    bold = false
    padding = space.s4

    if tone.primary or tone.joined
        height = ActionControlHeight()
        size = sizes.textBase
        bold = true
    end if
    if tone.primary then padding = space.s5
    if tone.joined then padding = space.s3
    if tone.link
        height = LinkControlHeight()
        icon = LinkIconSize()
        bold = true
        padding = LinkPadding()
    end if
    if tone.glyph
        compact = true
        height = GlyphControlHeight()
        icon = GlyphIconSize()
        padding = Int((height - icon) / 2)
    end if
    if button.DoesExist("bold") then bold = ValueAt(button, "bold", false) = true

    width = padding * 2
    if compact
        if not IsBlank(glyph) then width = width + icon
    else
        if not IsBlank(glyph) then width = width + icon + space.s2
        width = width + TextWidth(label, size, bold)
    end if

    return {
        width: width,
        height: height,
        size: size,
        bold: bold,
        padding: padding,
        icon: icon,
        glyph: glyph,
        label: label,
        compact: compact,
        rounded: SlotRounding(buttons, index),
        fill: ButtonFill(theme, tone),
        line: ButtonBorder(theme, tone),
        ring: FocusRingColor(theme, tone),
        disc: DiscColor(theme, tone),
        discLine: DiscBorderColor(theme, tone),
        iconColor: IconColor(theme, tone),
        labelColor: LabelColor(theme, tone)
    }
end function

function DiscColor(theme as object, tone as object) as string
    if not tone.glyph then return ""
    if tone.focused then return theme.surfaceAlt
    if tone.pressed then return AccentTint(theme)

    return ""
end function

function DiscBorderColor(theme as object, tone as object) as string
    if not tone.glyph then return ""
    if tone.focused then return theme.accent

    return ""
end function

sub PlaceSlot(slot as object, spec as object, tallest as integer, offset as integer)
    space = SpacingScale()

    slot.group.translation = [offset, Int((tallest - spec.height) / 2)]

    inset = 0
    if not IsBlank(spec.ring) then inset = FocusRingInset()
    slot.ring.boxWidth = spec.width + inset * 2
    slot.ring.boxHeight = spec.height + inset * 2
    slot.ring.rounded = spec.rounded
    slot.ring.fillColor = ""
    slot.ring.lineColor = spec.ring
    slot.ring.translation = [- inset, - inset]

    slot.box.boxWidth = spec.width
    slot.box.boxHeight = spec.height
    slot.box.rounded = spec.rounded
    slot.box.fillColor = spec.fill
    slot.box.lineColor = spec.line

    slot.group.muteAudioGuide = true

    discLeft = Int((spec.width - spec.height) / 2)
    edge = 0
    if not IsBlank(spec.discLine) then edge = BorderThickness()

    slot.discRing.visible = not IsBlank(spec.discLine)
    if slot.discRing.visible
        slot.discRing.uri = GlyphUri("disc")
        slot.discRing.width = spec.height
        slot.discRing.height = spec.height
        slot.discRing.blendColor = spec.discLine
        slot.discRing.translation = [discLeft, 0]
    end if

    slot.disc.visible = not IsBlank(spec.disc)
    if slot.disc.visible
        slot.disc.uri = GlyphUri("disc")
        slot.disc.width = spec.height - edge * 2
        slot.disc.height = spec.height - edge * 2
        slot.disc.blendColor = spec.disc
        slot.disc.translation = [discLeft + edge, edge]
    end if

    cursor = spec.padding
    slot.icon.visible = not IsBlank(spec.glyph)
    if slot.icon.visible
        slot.icon.uri = spec.glyph
        slot.icon.width = spec.icon
        slot.icon.height = spec.icon
        slot.icon.blendColor = spec.iconColor
        slot.icon.translation = [cursor, Int((spec.height - spec.icon) / 2)]
        cursor = cursor + spec.icon + space.s2
    end if

    slot.label.visible = not spec.compact
    if not spec.compact
        slot.label.text = spec.label
        slot.label.color = spec.labelColor
        if spec.bold
            slot.label.font = SizedBoldFont(spec.size)
        else
            slot.label.font = SizedFont(spec.size)
        end if
        slot.label.width = spec.width - cursor
        slot.label.height = spec.height
        slot.label.maxLines = 1
        slot.label.ellipsisText = "…"
        slot.label.vertAlign = "center"
        slot.label.translation = [cursor, 0]
    end if
end sub

function SlotGap(buttons as object, index as integer, space as object) as integer
    if index + 1 >= buttons.Count() then return 0
    if ValueAt(buttons[index + 1], "joined", false) = true then return 0
    if TextOrBlank(ValueAt(buttons[index + 1], "style", "")) = "link" then return 0
    return space.s3
end function

function SlotRounding(buttons as object, index as integer) as string
    if index + 1 < buttons.Count() and ValueAt(buttons[index + 1], "joined", false) = true then return "left"
    if ValueAt(buttons[index], "joined", false) = true then return "right"
    return "all"
end function

function SlotDisabled(buttons as object, index as integer) as boolean
    if index < 0 or index >= buttons.Count() then return true
    return ValueAt(buttons[index], "disabled", false) = true
end function

function NearestEnabled(buttons as object, from as integer) as integer
    at = ClampInt(from, 0, buttons.Count() - 1)
    if not SlotDisabled(buttons, at) then return at

    for away = 1 to buttons.Count()
        if not SlotDisabled(buttons, at + away) then return at + away
        if not SlotDisabled(buttons, at - away) then return at - away
    end for
    return at
end function

function SteppedIndex(buttons as object, from as integer, direction as integer) as integer
    at = from + direction
    while at >= 0 and at < buttons.Count()
        if not SlotDisabled(buttons, at) then return at
        at = at + direction
    end while
    return from
end function

function IconFor(button as object, pressed as boolean) as string
    if pressed
        name = TextOrBlank(ValueAt(button, "iconOn", ""))
        if not IsBlank(name) then return GlyphUri(name)
    end if

    return GlyphUri(TextOrBlank(ValueAt(button, "icon", "")))
end function

function ButtonFill(theme as object, tone as object) as string
    if tone.glyph then return ""
    if tone.link
        if tone.focused then return AccentTint(theme)
        return ""
    end if
    if tone.disabled then return theme.surface
    if tone.primary then return theme.accent
    if tone.joined then return DismissTint(theme)
    if tone.pressed
        if tone.focused then return theme.accent
        return AccentTint(theme)
    end if
    return theme.surface
end function

function ButtonBorder(theme as object, tone as object) as string
    if tone.glyph then return ""
    if tone.link then return ""
    if tone.disabled then return theme.border
    if tone.primary or tone.joined then return ""
    if tone.focused or tone.pressed then return theme.accent
    return theme.border
end function

function FocusRingColor(theme as object, tone as object) as string
    if tone.focused and (tone.primary or tone.joined) then return theme.accent
    return ""
end function

function IconColor(theme as object, tone as object) as string
    if tone.glyph
        if tone.disabled then return theme.border
        return theme.text
    end if
    if tone.link
        if tone.disabled then return theme.muted
        return theme.accent
    end if
    if tone.disabled then return theme.border
    if tone.primary then return theme.accentContrast
    if tone.joined then return theme.text
    if tone.pressed
        if tone.focused then return theme.accentContrast
        return theme.accent
    end if
    return theme.muted
end function

function LabelColor(theme as object, tone as object) as string
    if tone.link
        if tone.disabled then return theme.muted
        return theme.accent
    end if
    if tone.disabled then return theme.border
    if tone.danger then return theme.danger
    if tone.primary then return theme.accentContrast
    if tone.joined then return theme.text
    if tone.pressed
        if tone.focused then return theme.accentContrast
        return theme.accent
    end if
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
            ring: group.CreateChild("RoundedBox"),
            box: group.CreateChild("RoundedBox"),
            discRing: group.CreateChild("Poster"),
            disc: group.CreateChild("Poster"),
            icon: group.CreateChild("Poster"),
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

    buttons = m.top.buttons
    if type(buttons) <> "roArray" or buttons.Count() = 0 then return false

    if key = "left"
        at = SteppedIndex(buttons, m.top.focusIndex, -1)
        if at = m.top.focusIndex then return false
        m.top.focusIndex = at
        return true
    end if

    if key = "right"
        m.top.focusIndex = SteppedIndex(buttons, m.top.focusIndex, 1)
        return true
    end if

    if key = "OK"
        if SlotDisabled(buttons, m.top.focusIndex) then return true
        m.top.activated = TextOrBlank(ValueAt(buttons[m.top.focusIndex], "id", ""))
        return true
    end if

    return false
end function
