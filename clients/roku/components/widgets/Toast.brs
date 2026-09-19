sub init()
    m.surface = m.top.FindNode("surface")
    m.edge = m.top.FindNode("edge")
    m.message = m.top.FindNode("message")
    m.hold = m.top.FindNode("hold")
    m.hold.ObserveField("fire", "hide")
end sub

sub show()
    theme = m.top.theme
    if theme = invalid or theme.Count() = 0 then return
    if IsBlank(m.top.message)
        hide()
        return
    end if

    size = m.top.fontSize
    padding = size

    messageWidth = TextWidth(m.top.message, size)

    m.message.font = SizedFont(size)
    m.message.color = theme.text
    m.message.text = m.top.message
    m.message.translation = [padding * 1.5, padding / 2]
    m.message.width = messageWidth
    m.message.height = size * 1.6
    m.message.vertAlign = "center"

    m.surface.color = theme.surface
    m.surface.width = messageWidth + padding * 3
    m.surface.height = size * 1.6 + padding
    m.surface.visible = true

    m.edge.width = size / 4
    m.edge.height = m.surface.height
    m.edge.color = EdgeColor(m.top.kind, theme)

    m.hold.duration = m.top.holdSeconds
    m.hold.control = "start"

    SpeakAgain(m.top.message)
end sub

sub hide()
    m.surface.visible = false
end sub

function EdgeColor(kind as string, theme as object) as string
    if kind = "err" then return theme.danger
    return theme.ok
end function
