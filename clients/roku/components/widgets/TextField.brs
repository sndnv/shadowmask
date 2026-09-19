sub init()
    m.box = m.top.FindNode("box")
    m.caption = m.top.FindNode("caption")
    m.value = m.top.FindNode("value")
end sub

sub render()
    theme = m.top.theme
    if theme = invalid or theme.Count() = 0 then return

    sizes = TypeScale()
    space = SpacingScale()
    width = Int(m.top.fieldWidth)
    height = ActionControlHeight()

    captionColor = theme.muted
    valueColor = theme.text
    if m.top.disabled
        captionColor = theme.border
        valueColor = theme.border
    end if

    m.caption.text = m.top.label
    m.caption.color = captionColor
    m.caption.font = SizedFont(sizes.textXs)
    m.caption.width = width
    m.caption.maxLines = 1
    m.caption.ellipsisText = "…"
    m.caption.translation = [0, 0]

    boxTop = TextLinePitch(sizes.textXs)

    m.box.boxWidth = width
    m.box.boxHeight = height
    m.box.rounded = "all"
    m.box.fillColor = theme.surface
    m.box.lineColor = BorderColorFor(theme, m.top.hasFocus() and not m.top.disabled)
    m.box.translation = [0, boxTop]

    m.value.text = m.top.text
    m.value.color = valueColor
    m.value.font = SizedFont(sizes.textBase)
    m.value.width = width - space.s4 * 2
    m.value.height = height
    m.value.maxLines = 1
    m.value.ellipsisText = "…"
    m.value.vertAlign = "center"
    m.value.translation = [space.s4, boxTop]

    m.top.barHeight = boxTop + height

    if m.top.hasFocus()
        parts = [m.top.label, m.top.text]
        if m.top.disabled then parts.Push(Phrase("speech.unavailable"))
        Speak(SpokenLine(parts))
    end if
end sub

sub onFieldFocusChanged()
    render()
end sub

function onKeyEvent(key as string, press as boolean) as boolean
    if not KeysAreOurs() then return false
    if not press then return false
    if key <> "OK" then return false
    if m.top.disabled then return true

    m.top.activated = m.top.id
    return true
end function
