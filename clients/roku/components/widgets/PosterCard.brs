sub init()
    m.global.ObserveField("theme", "render")

    m.plate = m.top.FindNode("plate")
    m.art = m.top.FindNode("art")
    m.poster = m.top.FindNode("poster")
    m.glyph = m.top.FindNode("glyph")
    m.ring = m.top.FindNode("ring")

    m.top.muteAudioGuide = true
    m.progressTrack = m.top.FindNode("progressTrack")
    m.progressFill = m.top.FindNode("progressFill")
    m.watchedRing = m.top.FindNode("watchedRing")
    m.watchedDisc = m.top.FindNode("watchedDisc")
    m.dismiss = m.top.FindNode("dismiss")
    m.badgePlate = m.top.FindNode("badgePlate")
    m.badge = m.top.FindNode("badge")
    m.title = m.top.FindNode("title")
    m.titleTicker = m.top.FindNode("titleTicker")
    m.subtitle = m.top.FindNode("subtitle")
    m.subtitleTicker = m.top.FindNode("subtitleTicker")
    m.caption = m.top.FindNode("caption")
    m.captionTicker = m.top.FindNode("captionTicker")
    m.tracked = invalid
    m.rendered = false
    m.focused = false
end sub

sub onFocusChanged()
    wanted = m.top.focusPercent > 0.5 and m.top.rowListHasFocus
    if m.rendered and m.focused = wanted then return

    render()
end sub

sub showContent()
    if m.tracked <> invalid
        m.tracked.UnobserveField("watched")
        m.tracked.UnobserveField("progressPercent")
        m.tracked.UnobserveField("imageUri")
        m.tracked = invalid
    end if

    content = m.top.itemContent
    if content = invalid then return

    content.ObserveField("watched", "render")
    content.ObserveField("progressPercent", "render")
    content.ObserveField("imageUri", "render")
    m.tracked = content

    m.top.aspect = content.aspect
    render()
end sub

sub render()
    theme = m.global.theme
    content = m.top.itemContent
    if theme = invalid or theme.Count() = 0 or content = invalid then return

    sizes = TypeScale()
    space = SpacingScale()
    metrics = CardMetrics(m.top.width, content.aspect, content.captionRows)

    edge = Int(m.top.thickness)
    inner = metrics.width - edge * 2
    artHeight = metrics.artHeight - edge
    focused = m.top.focusPercent > 0.5 and m.top.rowListHasFocus
    m.focused = focused
    m.rendered = true

    m.plate.width = metrics.width
    m.plate.height = metrics.height
    m.plate.color = theme.surface

    m.art.width = inner
    m.art.height = artHeight
    m.art.color = theme.artBg
    m.art.translation = [edge, edge]

    hasArt = not IsBlank(content.imageUri)

    m.poster.visible = hasArt
    m.poster.width = inner
    m.poster.height = artHeight
    m.poster.loadWidth = inner
    m.poster.loadHeight = artHeight
    m.poster.uri = content.imageUri
    m.poster.loadDisplayMode = "scaleToZoom"
    m.poster.translation = [edge, edge]

    glyph = PlaceholderGlyphSize()
    m.glyph.visible = not hasArt
    m.glyph.uri = content.placeholderUri
    m.glyph.width = glyph
    m.glyph.height = glyph
    m.glyph.blendColor = theme.border
    m.glyph.translation = [edge + (inner - glyph) / 2, edge + (artHeight - glyph) / 2]

    m.ring.theme = theme
    m.ring.fill = ""
    m.ring.lineColor = BorderColorFor(theme, focused)
    m.ring.ringWidth = metrics.width
    m.ring.ringHeight = metrics.height
    m.ring.thickness = edge
    m.ring.translation = [0, 0]

    RenderProgress(theme, edge, inner, metrics.artHeight)
    RenderMarker(theme, space, edge, inner, focused, content.watched and not content.plain)
    RenderBadge(theme, sizes, space, edge, inner, content)
    RenderText(theme, sizes, space, metrics, edge, inner, content, focused)
end sub

sub RenderProgress(theme as object, edge as integer, inner as integer, artBottom as integer)
    percent = m.top.itemContent.progressPercent
    if m.top.itemContent.plain then percent = 0
    height = ProgressBarHeight()

    m.progressTrack.visible = percent > 0
    m.progressFill.visible = percent > 0
    if percent <= 0 then return

    top = artBottom - height

    m.progressTrack.width = inner
    m.progressTrack.height = height
    m.progressTrack.color = ProgressTrackColor()
    m.progressTrack.translation = [edge, top]

    m.progressFill.width = inner * ClampInt(percent, 0, 100) / 100.0
    m.progressFill.height = height
    m.progressFill.color = theme.accent
    m.progressFill.translation = [edge, top]
end sub

sub RenderMarker(theme as object, space as object, edge as integer, inner as integer, focused as boolean, watched as boolean)
    disc = space.s5
    ring = disc + space.s1
    inset = edge + space.s2
    left = edge + inner - ring - space.s2

    showDismiss = m.top.itemContent.dismissible and focused

    m.dismiss.visible = showDismiss
    if showDismiss
        m.dismiss.uri = ControlIconUri("close")
        m.dismiss.width = disc
        m.dismiss.height = disc
        m.dismiss.blendColor = theme.text
        m.dismiss.translation = [left + (ring - disc) / 2, inset + (ring - disc) / 2]
    end if

    m.watchedRing.visible = watched or showDismiss
    m.watchedDisc.visible = watched and not showDismiss

    if not m.watchedRing.visible then return

    m.watchedRing.uri = "pkg:/images/watched-ring.png"
    m.watchedRing.width = ring
    m.watchedRing.height = ring
    m.watchedRing.blendColor = theme.surface
    m.watchedRing.translation = [left, inset]

    if not m.watchedDisc.visible then return

    m.watchedDisc.uri = "pkg:/images/watched-disc.png"
    m.watchedDisc.width = disc
    m.watchedDisc.height = disc
    m.watchedDisc.blendColor = theme.ok
    m.watchedDisc.translation = [left + (ring - disc) / 2, inset + (ring - disc) / 2]
end sub

sub RenderBadge(theme as object, sizes as object, space as object, edge as integer, inner as integer, content as object)
    text = TextOrBlank(content.badge)

    m.badgePlate.visible = not IsBlank(text)
    m.badge.visible = m.badgePlate.visible
    if not m.badgePlate.visible then return

    size = sizes.textSm
    width = Int(TextWidth(text, size, true)) + space.s3 * 2
    height = Int(size * 1.7)
    left = edge + inner - width - space.s2
    top = edge + space.s2

    m.badgePlate.boxWidth = width
    m.badgePlate.boxHeight = height
    m.badgePlate.fillColor = theme.surface
    m.badgePlate.lineColor = theme.border
    m.badgePlate.translation = [left, top]

    m.badge.text = text
    m.badge.color = theme.text
    m.badge.font = SizedBoldFont(size)
    m.badge.width = width
    m.badge.height = height
    m.badge.horizAlign = "center"
    m.badge.vertAlign = "center"
    m.badge.translation = [left, top]
end sub

sub RenderText(theme as object, sizes as object, space as object, metrics as object, edge as integer, inner as integer, content as object, focused as boolean)
    shown = ClampInt(content.captionRows, 1, 3)

    left = edge + space.s3
    width = inner - space.s3 * 2
    offset = metrics.artHeight + space.s2

    offset = PlaceRow(m.title, m.titleTicker, content.cardTitle, sizes.textBase, theme.text, left, width, offset, focused, shown > 0)

    offset = PlaceRow(m.subtitle, m.subtitleTicker, content.cardSubtitle, sizes.textSm, theme.muted, left, width, offset, focused, shown > 1)

    PlaceRow(m.caption, m.captionTicker, content.cardCaption, sizes.textSm, theme.muted, left, width, offset, focused, shown > 2)
end sub

function PlaceRow(label as object, ticker as object, text as string, size as integer, color as string, left as integer, width as integer, offset as integer, focused as boolean, wanted as boolean) as integer
    if not wanted
        label.visible = false
        ticker.visible = false
        return offset
    end if

    height = Int(size * 1.3)

    label.visible = not focused
    label.text = text
    label.color = color
    label.font = SizedFont(size)
    label.width = width
    label.height = height
    label.maxLines = 1
    label.ellipsisText = "…"
    label.translation = [left, offset]

    ticker.visible = focused
    ticker.text = text
    ticker.color = color
    ticker.font = SizedFont(size)
    ticker.maxWidth = width
    ticker.repeatCount = -1
    ticker.scrollSpeed = TickerSpeed()
    ticker.translation = [left, offset]

    return offset + height
end function
