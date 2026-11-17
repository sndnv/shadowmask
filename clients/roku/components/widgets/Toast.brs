sub init()
    m.surface = m.top.FindNode("surface")
    m.plate = m.top.FindNode("plate")
    m.node = m.top.FindNode("node")
    m.glyph = m.top.FindNode("glyph")
    m.message = m.top.FindNode("message")
    m.hold = m.top.FindNode("hold")
    m.hold.ObserveField("fire", "Dismiss")
    m.enter = m.top.FindNode("enter")
    m.enterSlide = m.top.FindNode("enterSlide")
    m.enter.ObserveField("state", "onEnterState")
    m.shake = m.top.FindNode("shake")
    m.shakeSlide = m.top.FindNode("shakeSlide")
    m.dismiss = m.top.FindNode("dismiss")
    m.dismissSlide = m.top.FindNode("dismissSlide")
    m.dismiss.ObserveField("state", "onDismissState")
    m.entering = false
    m.dismissing = false
end sub

sub show()
    theme = m.top.theme
    if theme = invalid or theme.Count() = 0 then return
    if IsBlank(m.top.message)
        hide()
        return
    end if

    size = m.top.fontSize
    space = SpacingScale()
    palette = ToastPalette(m.top.kind, theme)
    cut = IsCut(theme)
    node = ToastNodeSize()
    glyphLeft = space.s4
    if cut then glyphLeft = space.s4 + node + space.s2
    width = ToastWidth()
    textLeft = glyphLeft + size + space.s3
    inner = width - textLeft - space.s4
    lines = ClampInt(TextLineCount(m.top.message, inner - SpaceWidth(size), size), 1, ToastMaxLines())
    height = TextBlockHeight(size, lines) + space.s3 * 2

    m.message.font = SizedFont(size)
    m.message.color = palette.ink
    m.message.text = m.top.message
    m.message.width = inner
    m.message.height = TextBlockHeight(size, lines)
    m.message.vertAlign = "center"
    m.message.maxLines = lines
    m.message.wrap = true
    m.message.lineSpacing = TextLineSpacing()
    m.message.ellipsisText = "…"
    m.message.translation = [textLeft, space.s3]

    m.glyph.uri = GlyphUri(palette.glyph)
    m.glyph.width = size
    m.glyph.height = size
    m.glyph.blendColor = ValueAt(palette, "tone", palette.ink)
    m.glyph.translation = [glyphLeft, Int((height - size) / 2)]

    m.node.visible = cut
    if cut
        m.node.uri = DiscUri(theme)
        m.node.width = node
        m.node.height = node
        m.node.blendColor = palette.tone
        m.node.translation = [space.s4, Int((height - node) / 2)]
    end if

    m.surface.width = width
    m.surface.height = height
    RenderGround(palette, cut, width, height)

    Enter(- width, - height)

    m.hold.duration = ToastHoldSeconds(m.top.kind)
    m.hold.control = "start"

    SpeakAgain(m.top.message)
end sub

sub RenderGround(palette as object, cut as boolean, width as integer, height as integer)
    if not cut
        m.surface.color = palette.ground
        m.plate.boxWidth = 0
        m.plate.boxHeight = 0
        return
    end if

    m.surface.color = "0x00000000"
    m.plate.fillColor = palette.ground
    m.plate.lineColor = palette.edge
    m.plate.boxWidth = width
    m.plate.boxHeight = height
end sub

sub Enter(restX as float, restY as float)
    m.entering = false
    m.dismissing = false
    m.enter.control = "stop"
    m.shake.control = "stop"
    m.dismiss.control = "stop"

    m.enterSlide.keyValue = [[0.0, restY], [restX, restY]]
    m.dismissSlide.keyValue = [[restX, restY], [0.0, restY]]
    swings = []
    for each offset in ToastShakeStops()
        swings.Push([restX + offset, restY])
    end for
    m.shakeSlide.keyValue = swings

    m.surface.translation = [0.0, restY]
    m.surface.opacity = 0
    m.surface.visible = true
    m.entering = true
    m.enter.control = "start"
end sub

sub onEnterState()
    if not m.entering or m.enter.state <> "stopped" then return
    m.entering = false
    if m.top.kind = "err" then m.shake.control = "start"
end sub

sub Dismiss()
    if not m.surface.visible then return
    m.entering = false
    m.enter.control = "stop"
    m.shake.control = "stop"
    m.dismissing = true
    m.dismiss.control = "start"
end sub

sub onDismissState()
    if not m.dismissing or m.dismiss.state <> "stopped" then return
    m.dismissing = false
    hide()
end sub

sub hide()
    m.surface.visible = false
end sub
