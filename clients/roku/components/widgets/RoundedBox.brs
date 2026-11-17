sub init()
    m.clip = m.top.FindNode("clip")
    m.fill = m.top.FindNode("fill")
    m.line = m.top.FindNode("line")
end sub

sub WatchTheme()
    if m.DoesExist("themeWatched") then return
    if not m.global.HasField("theme") then return

    m.themeWatched = true
    m.global.ObserveField("theme", "onThemeChanged")
end sub

sub onThemeChanged()
    hidden = not m.top.visible
    render()
    if hidden then m.top.visible = false
end sub

sub render()
    WatchTheme()
    width = Int(m.top.boxWidth)
    height = Int(m.top.boxHeight)

    m.top.visible = width > 0 and height > 0
    if not m.top.visible then return

    theme = m.global.theme
    radius = CornerRadius()
    lineUri = BoxLineUri(theme)
    if m.top.focusRing
        radius = FocusRingRadius()
        lineUri = FocusRingUri(theme)
    end if
    side = LCase(TextOrBlank(m.top.rounded))

    drawn = width
    bleed = 0
    if side = "left" or side = "right" then drawn = width + radius
    if side = "right" then bleed = - radius

    m.clip.clippingRect = [0, 0, width, height]

    Paint(m.fill, BoxFillUri(theme), m.top.fillColor, drawn, height, bleed)
    Paint(m.line, lineUri, m.top.lineColor, drawn, height, bleed)
end sub

sub Paint(node as object, uri as string, color as string, width as integer, height as integer, bleed as integer)
    node.visible = not IsBlank(color)
    if not node.visible then return

    node.uri = uri
    node.width = width
    node.height = height
    node.blendColor = color
    node.translation = [bleed, 0]
end sub
