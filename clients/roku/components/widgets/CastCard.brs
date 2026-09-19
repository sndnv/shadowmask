sub init()
    m.global.ObserveField("theme", "render")

    m.plate = m.top.FindNode("plate")
    m.art = m.top.FindNode("art")
    m.poster = m.top.FindNode("poster")
    m.glyph = m.top.FindNode("glyph")
    m.ring = m.top.FindNode("ring")
    m.line = m.top.FindNode("line")
    m.lineTicker = m.top.FindNode("lineTicker")
    m.name = m.top.FindNode("name")

    m.top.muteAudioGuide = true
end sub

sub render()
    theme = m.global.theme
    card = m.top.card
    if theme = invalid or theme.Count() = 0 then return
    if card = invalid or card.Count() = 0 then return

    sizes = TypeScale()
    space = SpacingScale()
    metrics = CardMetrics(m.top.cardWidth, PosterAspect(), 1)

    expanded = m.top.expanded
    width = CastCardWidthFor(m.top.cardWidth, expanded)
    edge = BorderThickness()
    inner = metrics.width - edge * 2
    artHeight = metrics.artHeight - edge

    m.plate.width = width
    m.plate.height = metrics.height
    m.plate.color = theme.surface

    m.art.width = inner
    m.art.height = artHeight
    m.art.color = theme.artBg
    m.art.translation = [edge, edge]

    uri = CardImageUrl(m.top.serverUrl, card, CardImageWidthFor(m.top.cardWidth))
    hasArt = not IsBlank(uri)

    m.poster.visible = hasArt
    m.poster.width = inner
    m.poster.height = artHeight
    m.poster.loadWidth = inner
    m.poster.loadHeight = artHeight
    m.poster.uri = uri
    m.poster.loadDisplayMode = "scaleToZoom"
    m.poster.translation = [edge, edge]

    glyph = PlaceholderGlyphSize()
    m.glyph.visible = not hasArt
    m.glyph.uri = CardPlaceholderUri(card)
    m.glyph.width = glyph
    m.glyph.height = glyph
    m.glyph.blendColor = theme.border
    m.glyph.translation = [edge + (inner - glyph) / 2, edge + (artHeight - glyph) / 2]

    m.ring.theme = theme
    m.ring.fill = ""
    m.ring.lineColor = BorderColorFor(theme, m.top.lit)
    m.ring.ringWidth = width
    m.ring.ringHeight = metrics.height
    m.ring.thickness = edge
    m.ring.translation = [0, 0]

    RenderBelow(theme, sizes, space, metrics, edge, width, expanded)
    RenderBeside(theme, space, metrics, width, expanded)
end sub

sub RenderBelow(theme as object, sizes as object, space as object, metrics as object, edge as integer, width as integer, expanded as boolean)
    character = TextOrBlank(ValueAt(m.top.card, "title", ""))
    span = width - edge * 2 - space.s3 * 2
    lineHeight = FontLineHeight(sizes.textSm)
    top = metrics.artHeight + Int((metrics.textHeight - lineHeight) / 2)

    m.line.visible = not expanded
    m.line.text = character
    m.line.color = theme.text
    m.line.font = SizedFont(sizes.textSm)
    m.line.width = span
    m.line.height = lineHeight
    m.line.wrap = false
    m.line.maxLines = 1
    m.line.ellipsisText = "…"
    m.line.horizAlign = "left"
    m.line.translation = [edge + space.s3, top]

    m.lineTicker.visible = expanded
    m.lineTicker.text = character
    m.lineTicker.color = theme.text
    m.lineTicker.font = SizedFont(sizes.textSm)
    m.lineTicker.maxWidth = span
    m.lineTicker.repeatCount = -1
    m.lineTicker.scrollSpeed = TickerSpeed()
    m.lineTicker.translation = [edge + space.s3, top]
end sub

sub RenderBeside(theme as object, space as object, metrics as object, width as integer, expanded as boolean)
    m.name.visible = expanded
    if not expanded then return

    left = metrics.width + space.s3
    column = width - left - space.s3
    room = metrics.artHeight - space.s4 * 2

    person = TextOrBlank(ValueAt(m.top.card, "subtitle", ""))
    size = FittedNameSize(person, column)
    lines = ClampInt(Int(room / TextLinePitch(size)), 1, 4)

    m.name.text = person
    m.name.color = theme.text
    m.name.font = SizedBoldFont(size)
    m.name.width = column
    m.name.height = metrics.artHeight
    m.name.wrap = true
    m.name.maxLines = lines
    m.name.ellipsisText = "…"
    m.name.horizAlign = "left"
    m.name.vertAlign = "center"
    m.name.translation = [left, 0]
end sub
