sub init()
    m.surface = m.top.FindNode("surface")
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
    width = ToastWidth()
    textLeft = space.s4 + size + space.s3
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
    m.glyph.blendColor = palette.ink
    m.glyph.translation = [space.s4, Int((height - size) / 2)]

    m.surface.color = palette.ground
    m.surface.width = width
    m.surface.height = height

    Enter(- width, - height)

    m.hold.duration = ToastHoldSeconds(m.top.kind)
    m.hold.control = "start"

    SpeakAgain(m.top.message)
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
