sub init()
    m.grid = m.top.FindNode("grid")
    m.grid.ObserveField("itemSelected", "onSelected")
    m.grid.ObserveField("itemFocused", "onFocused")

    m.cards = []
end sub

sub render()
    theme = m.top.theme
    if theme = invalid or theme.Count() = 0 then return

    space = SpacingScale()
    metrics = CardMetrics(m.top.cardWidth, m.top.aspect, m.top.captionRows)

    m.grid.itemComponentName = "PosterCard"
    m.grid.itemSize = [metrics.width, metrics.height]
    m.grid.itemSpacing = [space.s4, space.s5]
    m.grid.numColumns = m.top.columns
    m.grid.numRows = VisibleSlots(m.top.gridHeight, metrics.height, space.s5)
    m.grid.vertFocusAnimationStyle = "fixedFocus"
    m.grid.drawFocusFeedback = false
end sub

sub replaceCards()
    cards = m.top.cards
    if type(cards) <> "roArray" then return

    UnifyAspect(cards)
    m.top.aspect = AspectForSet(cards)
    render()

    m.cards = []
    for each card in cards
        m.cards.Push(card)
    end for

    m.grid.content = CardContentList(m.cards, m.top.serverUrl, CardImageWidthFor(m.top.cardWidth), m.top.captionRows)
    m.top.loaded = m.cards.Count()
end sub

sub onDormant()
    if m.top.dormant
        m.grid.content = invalid
        return
    end if

    if m.cards.Count() = 0 then return

    render()
    m.grid.content = CardContentList(m.cards, m.top.serverUrl, CardImageWidthFor(m.top.cardWidth), m.top.captionRows)
end sub

sub appendMore()
    cards = m.top.appendCards
    root = m.grid.content
    if type(cards) <> "roArray" or cards.Count() = 0 or root = invalid then return

    for each card in cards
        card.aspect = m.top.aspect
        m.cards.Push(card)
        root.AppendChild(CardContentNode(card, m.top.serverUrl, CardImageWidthFor(m.top.cardWidth), m.top.captionRows))
    end for
    m.top.loaded = m.cards.Count()
end sub

sub jumpToCard()
    index = m.top.jumpTo
    if index < 0 or index >= m.cards.Count() then return

    m.grid.jumpToItem = index
end sub

sub refreshStates()
    states = m.top.cardStates
    root = m.grid.content
    if root = invalid or type(states) <> "roArray" then return

    start = ClampInt(m.top.stateOffset, 0, m.cards.Count())
    count = StateWindow(states.Count(), start, root.GetChildCount(), m.cards.Count())

    for index = 0 to count - 1
        at = start + index
        watched = ValueAt(states[index], "watched", false) = true
        percent = Int(ValueAt(states[index], "progressPercent", 0))

        node = root.GetChild(at)
        node.watched = watched
        node.progressPercent = percent

        m.cards[at].watched = watched
        m.cards[at].progressPercent = percent
    end for
end sub

sub onSelected()
    root = m.grid.content
    if root = invalid or root.GetChildCount() = 0 then return

    index = ClampInt(m.grid.itemSelected, 0, root.GetChildCount() - 1)
    m.top.selected = CardTarget(root.GetChild(index))
end sub

sub onFocused()
    m.top.focused = m.grid.itemFocused

    root = m.grid.content
    if root = invalid or root.GetChildCount() = 0 then return

    index = ClampInt(m.grid.itemFocused, 0, root.GetChildCount() - 1)
    m.top.focusedCard = CardTarget(root.GetChild(index))
end sub

sub onFocus()
    if m.top.hasFocus() then m.grid.SetFocus(true)
end sub
