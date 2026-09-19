sub init()
    m.track = m.top.FindNode("track")
    m.fill = m.top.FindNode("fill")
    m.preview = m.top.FindNode("preview")
    m.thumb = m.top.FindNode("thumb")
    m.scrub = m.top.FindNode("scrub")
end sub

function TimelineThumbSize(focused as boolean) as integer
    if focused then return 28
    return 16
end function

sub render()
    theme = m.top.theme
    if theme = invalid or theme.Count() = 0 then return

    width = Int(m.top.barWidth)
    if width <= 0 then return

    focused = m.top.hasFocus()
    thumb = TimelineThumbSize(focused)
    height = ProgressBarHeight()
    top = Int((thumb - height) / 2)

    span = TimelineSpan(m.top.positionMs, m.top.scrubMs, m.top.durationMs, width)

    m.track.width = width
    m.track.height = height
    m.track.color = theme.border
    m.track.translation = [0, top]

    m.fill.width = span.fillTo
    m.fill.height = height
    m.fill.color = theme.accent
    m.fill.translation = [0, top]

    m.preview.visible = span.previewTo > span.fillTo
    if m.preview.visible
        m.preview.width = span.previewTo - span.fillTo
        m.preview.height = height
        m.preview.color = WithAlpha(theme.accent, TimelinePreviewAlpha())
        m.preview.translation = [span.fillTo, top]
    end if

    m.thumb.uri = GlyphUri("disc")
    m.thumb.width = thumb
    m.thumb.height = thumb
    m.thumb.blendColor = theme.accent
    m.thumb.translation = [ClampInt(span.thumbAt - Int(thumb / 2), 0, width - thumb), 0]

    m.scrub.visible = span.ghost
    if m.scrub.visible
        m.scrub.uri = GlyphUri("disc")
        m.scrub.width = thumb
        m.scrub.height = thumb
        m.scrub.blendColor = theme.text
        m.scrub.opacity = 0.65
        m.scrub.translation = [ClampInt(span.ghostAt - Int(thumb / 2), 0, width - thumb), 0]
    end if

    m.top.barHeight = thumb

    if focused
        Speak(PhraseWith("speech.at", { position: FormatDuration(Int(m.top.positionMs)), duration: FormatDuration(Int(m.top.durationMs)) }))
    end if
end sub

sub onTimelineFocusChanged()
    render()
end sub
