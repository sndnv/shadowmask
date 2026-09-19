sub init()
    m.track = m.top.FindNode("track")
    m.thumb = m.top.FindNode("thumb")
end sub

sub render()
    theme = m.top.theme
    if theme = invalid or theme.Count() = 0 then return

    track = Int(m.top.trackHeight)
    viewport = Int(m.top.viewportHeight)
    content = Int(m.top.contentHeight)
    width = ScrollBarWidth()

    if viewport <= 0 then viewport = track

    if track <= 0 or viewport <= 0 or content <= viewport
        m.top.visible = false
        return
    end if

    m.top.visible = true
    metrics = ScrollThumb(content, viewport, Int(m.top.offset), ScrollThumbMinimum(), track)

    m.track.width = width
    m.track.height = track
    m.track.color = theme.surfaceAlt
    m.track.translation = [0, 0]

    m.thumb.width = width
    m.thumb.height = metrics.height
    m.thumb.color = theme.muted
    m.thumb.translation = [0, metrics.top]
end sub
