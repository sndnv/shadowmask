sub init()
    m.heading = m.top.FindNode("heading")
    m.viewport = m.top.FindNode("viewport")
    m.strip = m.top.FindNode("strip")

    m.built = []
    m.index = 0
    m.offset = 0
end sub

sub onCards()
    render()
end sub

sub onDormant()
    if m.top.dormant
        Clear()
        return
    end if

    render()
end sub

sub Clear()
    while m.strip.GetChildCount() > 0
        m.strip.RemoveChildIndex(0)
    end while
    m.built = []
end sub

sub render()
    theme = m.top.theme
    cards = m.top.cards
    if theme = invalid or theme.Count() = 0 then return
    if type(cards) <> "roArray" then return

    sizes = TypeScale()
    space = SpacingScale()
    metrics = CardMetrics(m.top.cardWidth, PosterAspect(), 1)
    labelHeight = Int(sizes.textXl * 1.4) + space.s2

    m.heading.text = m.top.heading
    m.heading.color = theme.text
    m.heading.font = SizedBoldFont(sizes.textXl)
    m.heading.width = m.top.railWidth
    m.heading.height = labelHeight
    m.heading.maxLines = 1
    m.heading.ellipsisText = "…"
    m.heading.translation = [0, 0]

    m.viewport.translation = [0, labelHeight]
    m.viewport.clippingRect = [0, 0, Int(m.top.railWidth), metrics.height]

    m.top.railHeight = labelHeight + metrics.height

    if m.top.dormant then return

    Build(cards)
    Place(space)
end sub

sub Build(cards as object)
    if m.built.Count() <> cards.Count() then Clear()

    if m.built.Count() = cards.Count()
        for index = 0 to cards.Count() - 1
            m.built[index].serverUrl = m.top.serverUrl
            m.built[index].card = cards[index]
        end for
        return
    end if

    for each card in cards
        node = m.strip.CreateChild("CastCard")
        node.serverUrl = m.top.serverUrl
        node.card = card
        m.built.Push(node)
    end for
end sub

sub Place(space as object)
    if m.built.Count() = 0 then return

    held = m.top.isInFocusChain()
    m.index = ClampInt(m.index, 0, m.built.Count() - 1)

    cursor = 0
    lefts = []
    widths = []

    for index = 0 to m.built.Count() - 1
        node = m.built[index]
        wide = held and index = m.index

        node.cardWidth = m.top.cardWidth
        node.expanded = wide
        node.lit = held and index = m.index
        node.translation = [cursor, 0]

        width = CastCardWidthFor(m.top.cardWidth, wide)
        lefts.Push(cursor)
        widths.Push(width)
        cursor = cursor + width + space.s4
    end for

    Scroll(lefts, widths, cursor - space.s4)
end sub

sub Scroll(lefts as object, widths as object, total as integer)
    viewport = Int(m.top.railWidth)
    at = ClampInt(m.index, 0, lefts.Count() - 1)

    offset = m.offset
    right = lefts[at] + widths[at]
    if right > offset + viewport then offset = right - viewport
    if lefts[at] < offset then offset = lefts[at]

    ceiling = total - viewport
    if ceiling < 0 then ceiling = 0
    offset = ClampInt(offset, 0, ceiling)

    m.offset = offset
    m.strip.translation = [0 - offset, 0]
end sub

function CardAt(index as integer) as object
    cards = m.top.cards
    if type(cards) <> "roArray" or cards.Count() = 0 then return {}

    card = cards[ClampInt(index, 0, cards.Count() - 1)]

    return {
        kind: TextOrBlank(ValueAt(card, "kind", "")),
        id: TextOrBlank(ValueAt(card, "id", "")),
        title: TextOrBlank(ValueAt(card, "subtitle", "")),
        subtitle: TextOrBlank(ValueAt(card, "title", "")),
        imageUri: CardImageUrl(m.top.serverUrl, card, CardImageWidthFor(m.top.cardWidth)),
        aspect: TextOrBlank(ValueAt(card, "aspect", PosterAspect()))
    }
end function

sub Focus(index as integer)
    wanted = ClampInt(index, 0, m.built.Count() - 1)
    if wanted = m.index then return

    m.index = wanted
    Publish()
    render()
end sub

sub Publish()
    cards = m.top.cards
    if type(cards) <> "roArray" or cards.Count() = 0 then return

    at = ClampInt(m.index, 0, cards.Count() - 1)
    m.top.itemFocused = at
    m.top.focusedCard = CardAt(at)
    Speak(SlotSpeech(CardSpeech(cards[at]), at, cards.Count()))
end sub

sub onFocus()
    render()
    if m.top.hasFocus() then Publish()
end sub

function onKeyEvent(key as string, press as boolean) as boolean
    if not KeysAreOurs() then return false
    if not press then return false
    if m.built.Count() = 0 then return false

    if key = "left"
        if m.index <= 0 then return false

        Focus(m.index - 1)
        return true
    end if

    if key = "right"
        if m.index >= m.built.Count() - 1 then return true

        Focus(m.index + 1)
        return true
    end if

    if key = "OK"
        m.top.selected = CardAt(m.index)
        return true
    end if

    return false
end function
