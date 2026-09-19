sub init()
    m.viewport = m.top.FindNode("viewport")
    m.stack = m.top.FindNode("stack")

    m.rails = []
    m.layout = []
    m.tops = []
    m.heights = []
    m.row = 0
    m.col = 0
end sub

sub render()
    theme = m.top.theme
    rows = m.top.rows
    if theme = invalid or theme.Count() = 0 then return
    if type(rows) <> "roArray" then return

    space = SpacingScale()
    gap = space.s5

    Rebuild(rows.Count())
    widths = RailCardWidths(rows)
    m.layout = PackedLayout(rows, widths, space)

    m.tops = []
    m.heights = []
    offset = 0

    for each line in m.layout
        left = 0
        tallest = 0

        for each slot in line
            rail = m.rails[slot.index]
            rail.theme = theme
            rail.serverUrl = m.top.serverUrl
            rail.railWidth = slot.width
            rail.cardWidth = widths[slot.index]
            rail.captionRows = m.top.captionRows
            rail.heading = TextOrBlank(ValueAt(rows[slot.index], "heading", ""))
            rail.cards = ValueAt(rows[slot.index], "cards", [])
            rail.translation = [left, offset]

            left = left + slot.width + space.s5
            if rail.railHeight > tallest then tallest = rail.railHeight
        end for

        m.tops.Push(offset)
        m.heights.Push(tallest)
        offset = offset + tallest + gap
    end for

    m.viewport.clippingRect = [0, 0, m.top.rowsWidth, m.top.rowsHeight]
    m.top.contentHeight = offset - gap + ContentBottomPad()

    m.row = ClampInt(m.row, 0, m.layout.Count() - 1)
    if m.layout.Count() > 0 then m.col = ClampInt(m.col, 0, m.layout[m.row].Count() - 1)
    ScrollToFocused()
end sub

function RailCardWidths(rows as object) as object
    widths = []
    for each row in rows
        if m.top.packed
            widths.Push(CardWidthFor(AspectForSet(ValueAt(row, "cards", []))))
        else
            widths.Push(Int(m.top.cardWidth))
        end if
    end for
    return widths
end function

function PackedLayout(rows as object, widths as object, space as object) as object
    if not m.top.packed
        lines = []
        for index = 0 to rows.Count() - 1
            lines.Push([{ index: index, width: Int(m.top.rowsWidth) }])
        end for
        return lines
    end if

    natural = []
    for index = 0 to rows.Count() - 1
        cards = ValueAt(rows[index], "cards", [])
        count = 0
        if type(cards) = "roArray" then count = cards.Count()
        natural.Push(GroupNaturalWidth(count, widths[index], space.s4))
    end for

    return PackedRows(natural, Int(m.top.rowsWidth), space.s5)
end function

sub Rebuild(count as integer)
    if m.rails.Count() = count then return

    while m.stack.GetChildCount() > 0
        m.stack.RemoveChildIndex(0)
    end while
    m.rails = []

    for index = 0 to count - 1
        rail = m.stack.CreateChild("CardRail")
        rail.ObserveField("selected", "onRailSelected")
        rail.ObserveField("focusedCard", "onRailCardFocused")
        m.rails.Push(rail)
    end for
end sub

sub ScrollToFocused()
    if m.layout.Count() = 0 then return

    offset = ScrollOffsetFor(m.row, m.tops, m.heights, m.top.rowsHeight)
    m.stack.translation = [0, - offset]
    m.top.scrollOffset = offset
end sub

function FocusedRail() as dynamic
    if m.layout.Count() = 0 then return invalid

    line = m.layout[m.row]
    if line.Count() = 0 then return invalid
    return m.rails[line[ClampInt(m.col, 0, line.Count() - 1)].index]
end function

sub FocusRail(row as integer, col as integer)
    if m.layout.Count() = 0 then return

    m.row = ClampInt(row, 0, m.layout.Count() - 1)
    m.col = ClampInt(col, 0, m.layout[m.row].Count() - 1)

    ScrollToFocused()

    rail = FocusedRail()
    if rail = invalid then return

    rail.SetFocus(true)
    m.top.focusedRail = m.layout[m.row][m.col].index
    m.top.focusedCard = rail.focusedCard
end sub

sub onRailSelected(event as object)
    target = event.GetData()
    if target = invalid or target.Count() = 0 then return

    m.top.selected = target
end sub

sub onRailCardFocused(event as object)
    rail = FocusedRail()
    if rail = invalid then return
    if not rail.isSameNode(event.GetRoSGNode()) then return

    m.top.focusedCard = event.GetData()
end sub

sub onFocus()
    if not m.top.hasFocus() then return
    if m.layout.Count() = 0 then return

    FocusRail(m.row, m.col)
end sub

function onKeyEvent(key as string, press as boolean) as boolean
    if not KeysAreOurs() then return false
    if not press then return false
    if m.layout.Count() = 0 then return false

    if key = "down"
        if m.row >= m.layout.Count() - 1 then return false
        FocusRail(m.row + 1, m.col)
        return true
    end if

    if key = "up"
        if m.row <= 0 then return false
        FocusRail(m.row - 1, m.col)
        return true
    end if

    if key = "right"
        if m.col >= m.layout[m.row].Count() - 1 then return false
        FocusRail(m.row, m.col + 1)
        return true
    end if

    if key = "left"
        if m.col <= 0 then return false
        FocusRail(m.row, m.col - 1)
        return true
    end if

    return false
end function
