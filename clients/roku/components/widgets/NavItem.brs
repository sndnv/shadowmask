sub init()
    m.icon = m.top.FindNode("icon")
    m.title = m.top.FindNode("title")
    m.rendered = false
    m.lit = false
    m.top.muteAudioGuide = true
    m.global.ObserveField("theme", "render")
end sub

sub onFocusChanged()
    lit = NavRowLit(m.top.focusPercent, m.top.listHasFocus)
    if m.rendered and m.lit = lit then return

    render()
end sub

sub render()
    theme = m.global.theme
    content = m.top.itemContent
    if theme = invalid or theme.Count() = 0 or content = invalid then return

    size = TypeScale().textLg
    space = SpacingScale()
    inset = space.s3
    icon = ControlIconSize()
    m.lit = NavRowLit(m.top.focusPercent, m.top.listHasFocus)
    m.rendered = true
    look = NavRowLook(theme, m.lit)

    m.icon.uri = TextOrBlank(content.navIcon)
    m.icon.width = icon
    m.icon.height = icon
    m.icon.blendColor = look.color
    m.icon.translation = [inset, Int((m.top.height - icon) / 2)]

    left = inset + icon + space.s4
    m.title.text = TextOrBlank(content.title)
    m.title.color = look.color
    if look.bold
        m.title.font = SizedBoldFont(size)
    else
        m.title.font = SizedFont(size)
    end if
    m.title.width = m.top.width - left - inset
    m.title.height = m.top.height
    m.title.translation = [left, 0]
    m.title.maxLines = 1
    m.title.vertAlign = "center"
end sub
