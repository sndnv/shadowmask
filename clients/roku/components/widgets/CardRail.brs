sub init()
    m.heading = m.top.FindNode("heading")
    m.row = m.top.FindNode("row")

    m.arrows = [
        {
            group: m.top.FindNode("leftArrow"),
            disc: m.top.FindNode("leftDisc"),
            chevron: m.top.FindNode("leftChevron"),
            uri: ChevronUri("left")
        },
        {
            group: m.top.FindNode("rightArrow"),
            disc: m.top.FindNode("rightDisc"),
            chevron: m.top.FindNode("rightChevron"),
            uri: ChevronUri("right")
        }
    ]

    m.row.ObserveField("rowItemSelected", "onSelected")
    m.row.ObserveField("rowItemFocused", "onItemFocused")
    m.builtWith = ""
    m.builtShape = ""
    m.builtViewport = 0

    m.top.ObserveField("focusedChild", "onFocus")
end sub

sub render()
    theme = m.top.theme
    cards = m.top.cards
    if theme = invalid or theme.Count() = 0 then return
    if type(cards) <> "roArray" then return

    sizes = TypeScale()
    space = SpacingScale()

    UnifyAspect(cards)
    metrics = CardMetrics(m.top.cardWidth, AspectForSet(cards), m.top.captionRows)
    labelHeight = Int(sizes.textXl * 1.4) + space.s2
    room = FocusRoom(metrics.width, metrics.height, space.s4)
    rowTop = labelHeight + room.y
    m.room = room

    m.heading.text = m.top.heading
    m.heading.color = theme.text
    m.heading.font = SizedBoldFont(sizes.textXl)
    m.heading.width = m.top.railWidth
    m.heading.height = labelHeight
    m.heading.maxLines = 1
    m.heading.ellipsisText = "…"
    m.heading.translation = [0, 0]

    viewport = RailViewportWidth(Int(m.top.railWidth), metrics.width, space.s4)
    m.railViewport = viewport

    m.row.itemComponentName = "PosterCard"
    m.row.numRows = 1
    m.row.itemSize = [viewport + room.x * 2, metrics.height + room.y * 2]
    m.row.rowItemSize = [[metrics.width + room.x * 2, metrics.height + room.y * 2]]
    m.row.rowItemSpacing = [[space.s4 - room.x * 2, 0]]
    m.row.rowHeights = [metrics.height + room.y * 2]
    m.row.showRowLabel = [false]
    m.row.showRowCounter = [false]
    m.row.focusXOffset = RailFocusRange(viewport, metrics.width)
    m.row.vertFocusAnimationStyle = "fixedFocus"
    m.row.rowFocusAnimationStyle = "floatingFocus"
    m.row.drawFocusFeedback = false
    m.row.translation = [- room.x, labelHeight]

    RefreshContent(cards, viewport)

    m.top.railHeight = rowTop + metrics.height
    m.artHeight = metrics.artHeight
    m.rowTop = rowTop
    RenderArrows(theme)
end sub

sub onDormant()
    if m.top.dormant
        m.row.content = invalid
        m.builtWith = ""
        m.builtShape = ""
        m.builtViewport = 0
        return
    end if

    render()
end sub

sub RefreshContent(cards as object, viewport as integer)
    if m.top.dormant then return
    if cards.Count() = 0 and m.row.content = invalid then return

    imageWidth = CardImageWidthFor(m.top.cardWidth)
    layout = "@" + viewport.ToStr() + "@" + m.room.x.ToStr() + "x" + m.room.y.ToStr()
    shape = CardSetShape(cards, m.top.captionRows) + layout
    signature = CardSetSignature(cards, m.top.serverUrl, imageWidth, m.top.captionRows) + layout
    if m.builtWith <> invalid and m.builtWith = signature then return

    if m.builtShape <> invalid and m.builtShape = shape and PaintedArtwork(cards, imageWidth)
        m.builtWith = signature
        return
    end if

    held = m.row.isInFocusChain()
    at = Int(m.top.itemFocused)

    if m.builtViewport <> invalid and m.builtViewport > 0 and m.builtViewport <> viewport then m.row.content = invalid

    root = CreateObject("roSGNode", "ContentNode")
    row = root.CreateChild("ContentNode")
    for each card in cards
        row.AppendChild(CardContentNode(card, m.top.serverUrl, imageWidth, m.top.captionRows, m.room))
    end for

    m.row.content = root
    m.builtShape = shape
    m.builtWith = signature
    m.builtViewport = viewport

    if cards.Count() = 0 then return

    m.row.jumpToRowItem = [0, ClampInt(at, 0, cards.Count() - 1)]
    if held then m.row.SetFocus(true)
end sub

function PaintedArtwork(cards as object, imageWidth as integer) as boolean
    root = m.row.content
    if root = invalid or root.GetChildCount() = 0 then return false

    items = root.GetChild(0)
    if items.GetChildCount() <> cards.Count() then return false

    for index = 0 to cards.Count() - 1
        uri = CardImageUrl(m.top.serverUrl, cards[index], imageWidth)
        item = items.GetChild(index)
        if item.imageUri <> uri then item.imageUri = uri
    end for

    return true
end function

sub RenderArrows(theme as object)
    cards = m.top.cards
    if type(cards) <> "roArray" then return

    space = SpacingScale()
    disc = ChevronSize() + space.s3
    chevron = ChevronSize()
    top = m.rowTop + Int((m.artHeight - disc) / 2)

    viewport = Int(m.railViewport)
    if viewport <= 0 then viewport = Int(m.top.railWidth)

    visible = ColumnsThatFit(viewport, Int(m.top.cardWidth), space.s4)
    overflows = cards.Count() > visible

    focused = m.top.itemFocused
    shown = [overflows and focused > 0, overflows and focused < cards.Count() - 1]
    lefts = [space.s2, viewport - disc - space.s2]

    for index = 0 to m.arrows.Count() - 1
        arrow = m.arrows[index]
        arrow.group.visible = shown[index]
        if shown[index]
            arrow.group.translation = [lefts[index], top]

            arrow.disc.uri = GlyphUri("disc")
            arrow.disc.width = disc
            arrow.disc.height = disc
            arrow.disc.blendColor = theme.surface
            arrow.disc.translation = [0, 0]

            arrow.chevron.uri = arrow.uri
            arrow.chevron.width = chevron
            arrow.chevron.height = chevron
            arrow.chevron.blendColor = theme.text
            arrow.chevron.translation = [(disc - chevron) / 2, (disc - chevron) / 2]
        end if
    end for
end sub

sub onSelected()
    root = m.row.content
    if root = invalid or root.GetChildCount() = 0 then return

    items = root.GetChild(0)
    if items.GetChildCount() = 0 then return

    index = ClampInt(m.row.rowItemSelected[1], 0, items.GetChildCount() - 1)
    m.top.selected = CardTarget(items.GetChild(index))
end sub

sub onItemFocused()
    m.top.itemFocused = m.row.rowItemFocused[1]
    m.top.focusedCard = CardAt(m.top.itemFocused)

    theme = m.top.theme
    if theme <> invalid and theme.Count() > 0 then RenderArrows(theme)
end sub

function CardAt(index as integer) as object
    root = m.row.content
    if root = invalid or root.GetChildCount() = 0 then return {}

    items = root.GetChild(0)
    if items.GetChildCount() = 0 then return {}

    return CardTarget(items.GetChild(ClampInt(index, 0, items.GetChildCount() - 1)))
end function

sub onFocus()
    if m.top.hasFocus() then m.row.SetFocus(true)
end sub
