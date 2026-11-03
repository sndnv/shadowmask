sub init()
    m.heading = m.top.FindNode("heading")
    m.facts = m.top.FindNode("facts")
    m.crew = m.top.FindNode("crew")
    m.chips = m.top.FindNode("chips")
    m.overview = m.top.FindNode("overview")
end sub

sub render()
    theme = m.top.theme
    content = m.top.summary
    width = Int(m.top.summaryWidth)
    if theme = invalid or theme.Count() = 0 then return
    if content = invalid or width <= 0 then return

    sizes = TypeScale()

    offset = HeroLine(m.heading, ValueAt(content, "heading", ""), sizes.text2xl, theme.text, true, 0, width, 0, 1)
    offset = HeroLine(m.facts, ValueAt(content, "facts", ""), sizes.textBase, theme.muted, false, 0, width, offset, 1)
    offset = HeroLine(m.crew, ValueAt(content, "crew", ""), sizes.textSm, theme.muted, false, 0, width, offset, 1)
    offset = HeroChips(m.chips, theme, ValueAt(content, "chips", invalid), 0, width, offset)

    text = TextOrBlank(ValueAt(content, "overview", ""))
    lines = SummaryOverviewLines(Int(m.top.summaryHeight) - offset, sizes.textSm, SummaryOverviewMaxLines())
    m.overview.visible = not IsBlank(text) and lines > 0
    if m.overview.visible then DrawOverviewText(m.overview, theme, text, sizes.textSm, 0, width, offset, lines)
end sub
