sub init()
    m.bar = m.top.FindNode("bar")
    m.message = m.top.FindNode("message")
end sub

sub render()
    theme = m.top.theme
    if theme = invalid or theme.Count() = 0 then return

    size = m.top.fontSize
    barWidth = size / 4
    messageWidth = m.top.noteWidth - barWidth * 3

    lines = ClampInt(TextLineCount(m.top.message, Int(messageWidth), Int(size)), 1, StateNoteMaxLines())
    height = StateNoteHeight(Int(size), lines)

    m.bar.visible = not IsBlank(m.top.message)
    m.bar.width = barWidth
    m.bar.height = height
    m.bar.color = AccentFor(m.top.kind, theme)

    m.message.translation = [barWidth * 3, 0]
    m.message.width = messageWidth
    m.message.height = height
    m.message.vertAlign = "center"
    m.message.wrap = true
    m.message.maxLines = lines
    m.message.ellipsisText = "…"
    m.message.color = TextFor(m.top.kind, theme)
    m.message.text = m.top.message

    Speak(m.top.message)
    m.message.font = SizedFont(size)

    m.top.noteHeight = height
end sub

function AccentFor(kind as string, theme as object) as string
    if kind = "error" then return theme.danger
    if kind = "empty" then return theme.border
    return theme.accent
end function

function TextFor(kind as string, theme as object) as string
    if kind = "error" then return theme.text
    return theme.muted
end function
