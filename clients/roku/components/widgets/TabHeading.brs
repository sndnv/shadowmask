sub init()
    m.plain = m.top.FindNode("plain")
    m.tab = m.top.FindNode("tab")
    m.rule = m.top.FindNode("rule")
    m.body = m.top.FindNode("body")
    m.slant = m.top.FindNode("slant")
    m.label = m.top.FindNode("label")
    m.tally = m.top.FindNode("count")
end sub

sub render()
    theme = m.top.theme
    if theme = invalid or theme.Count() = 0 then return

    cut = IsCut(theme)
    m.plain.visible = not cut
    m.tab.visible = cut and not IsBlank(m.top.text)
    if m.tab.visible
        RenderTab(theme)
    else if not cut
        RenderPlain(theme)
    end if
end sub

sub RenderPlain(theme as object)
    text = m.top.text
    if m.top.count >= 0 then text = CountLabel(text, m.top.count)

    m.plain.text = text
    m.plain.color = theme.text
    m.plain.font = SizedBoldFont(m.top.size)
    m.plain.width = m.top.headingWidth
    m.plain.maxLines = 1
    m.plain.ellipsisText = "…"
    m.plain.translation = [0, 0]
end sub

sub RenderTab(theme as object)
    size = TypeScale().textXs
    height = TabHeight()
    slant = TabSlantWidth()
    width = Int(m.top.headingWidth)
    text = CapsFor(theme, m.top.text)
    tally = ""
    if m.top.count >= 0 then tally = m.top.count.ToStr()

    room = width - TabPadLeft() - TabPadRight()
    m.tally.visible = not IsBlank(tally)
    if m.tally.visible then room = room - SpacingScale().s3 - TextWidth(tally, size)
    if room < 1 then room = 1

    shown = TextWidth(text, size, true)
    if shown > room then shown = room
    tabWidth = TabPadLeft() + shown + TabPadRight()

    m.body.width = tabWidth - slant
    m.body.height = height
    m.body.color = theme.accent
    m.body.translation = [0, 0]

    m.slant.uri = GlyphUri("tab-slant")
    m.slant.width = slant
    m.slant.height = height
    m.slant.blendColor = theme.accent
    m.slant.translation = [tabWidth - slant, 0]

    m.label.text = text
    m.label.color = theme.accentContrast
    m.label.font = SizedBoldFont(size)
    m.label.width = room
    m.label.height = height
    m.label.maxLines = 1
    m.label.ellipsisText = "…"
    m.label.vertAlign = "center"
    m.label.translation = [TabPadLeft(), 0]

    if m.tally.visible
        m.tally.text = tally
        m.tally.color = theme.accent
        m.tally.font = SizedFont(size)
        m.tally.height = height
        m.tally.vertAlign = "center"
        m.tally.translation = [tabWidth + SpacingScale().s3, 0]
    end if

    m.rule.width = width
    m.rule.height = BorderThickness()
    m.rule.color = theme.line
    m.rule.translation = [0, height - BorderThickness()]
end sub
