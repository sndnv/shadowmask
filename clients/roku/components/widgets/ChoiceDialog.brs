sub init()
    m.scrim = m.top.FindNode("scrim")
    m.panel = m.top.FindNode("panel")
    m.scrim.muteAudioGuide = true
    m.plate = m.top.FindNode("plate")
    m.titleIcon = m.top.FindNode("titleIcon")
    m.title = m.top.FindNode("title")
    m.textView = m.top.FindNode("textView")
    m.message = m.top.FindNode("message")
    m.textBar = m.top.FindNode("textBar")
    m.viewport = m.top.FindNode("viewport")
    m.rowGroup = m.top.FindNode("rows")
    m.bar = m.top.FindNode("bar")

    m.built = []
    m.model = []
    m.index = 0
    m.offset = 0
    m.count = 0
    m.field = ""
    m.textOffset = 0
    m.textNatural = 0
    m.textShown = 0
    m.textLines = 1
    m.titleLines = 1
end sub

sub MeasureRequest()
    sizes = TypeScale()
    inner = DialogWidth() - SpacingScale().s5 * 2

    body = TextOrBlank(ValueAt(m.top.request, "message", ""))
    m.textLines = 0
    if not IsBlank(body) then m.textLines = TextLineCount(body, inner, sizes.textBase)

    title = TextOrBlank(ValueAt(m.top.request, "title", ""))
    m.titleLines = ClampInt(TextLineCount(title, inner - TitleGlyphLead(), sizes.textLg), 1, DialogMessageLines())
end sub

function TitleGlyph() as string
    if IsBlank(TextOrBlank(ValueAt(m.top.request, "title", ""))) then return ""
    return GlyphUri(TextOrBlank(ValueAt(m.top.request, "icon", "")))
end function

function TitleGlyphLead() as integer
    if IsBlank(TitleGlyph()) then return 0
    return ControlIconSize() + SpacingScale().s3
end function

sub onRequest()
    request = m.top.request
    options = []
    if request <> invalid then options = ValueAt(request, "options", [])

    if type(options) <> "roArray" or options.Count() = 0
        m.top.visible = false
        return
    end if

    kind = TextOrBlank(ValueAt(request, "kind", "confirm"))
    selected = ClampInt(Int(ValueAt(request, "selected", 0)), 0, options.Count() - 1)

    m.field = TextOrBlank(ValueAt(request, "field", ""))
    m.count = options.Count()
    m.model = DialogRows(options, kind, selected)
    m.index = selected
    m.offset = 0
    m.textOffset = 0

    m.top.visible = true
    MeasureRequest()
    render()
end sub

sub render()
    theme = m.top.theme
    if theme = invalid or theme.Count() = 0 then return
    if not m.top.visible then return
    if type(m.model) <> "roArray" or m.model.Count() = 0 then return

    space = SpacingScale()
    sizes = TypeScale()
    width = DialogWidth()
    pad = space.s5
    inner = width - pad * 2

    m.scrim.width = CanvasWidth()
    m.scrim.height = CanvasHeight()
    m.scrim.color = ScrimColor(theme)

    cursor = pad
    cursor = PlaceDialogTitle(theme, sizes.textLg, inner, pad, cursor)

    size = sizes.textBase
    body = TextOrBlank(ValueAt(m.top.request, "message", ""))
    natural = 0
    if m.textLines > 0 then natural = TextBlockHeight(size, m.textLines)

    plan = RowSpans(m.model, space)
    room = DialogMaxHeight() - cursor - pad
    if room < DialogRowHeight() then room = DialogRowHeight()

    budget = DialogTextPlan(natural, plan.content, room, space.s3)

    m.textNatural = natural
    m.textShown = budget.text
    m.textOffset = ClampInt(m.textOffset, 0, natural - budget.text)

    cursor = PlaceDialogMessage(theme, body, size, inner, pad, cursor, budget.text, natural, m.textLines)

    shown = budget.rows

    RebuildRows(m.model.Count())
    for index = 0 to m.model.Count() - 1
        PaintRow(m.built[index], m.model[index], theme, inner, plan.tops[index], index = m.index)
    end for

    m.offset = RevealOffset(m.index, plan.tops, plan.heights, shown, m.offset, 0)

    m.viewport.clippingRect = [pad, cursor, inner, shown]
    m.rowGroup.translation = [pad, cursor - m.offset]

    m.bar.theme = theme
    m.bar.trackHeight = shown
    m.bar.viewportHeight = shown
    m.bar.contentHeight = plan.content
    m.bar.offset = m.offset
    m.bar.translation = [width - pad + space.s1, cursor]

    height = cursor + shown + pad
    m.plate.boxWidth = width
    m.plate.boxHeight = height
    m.plate.fillColor = theme.surface
    m.plate.lineColor = theme.border

    m.panel.translation = [Int((CanvasWidth() - width) / 2), Int((CanvasHeight() - height) / 2)]

    Speak(DialogSpeech(ValueAt(m.top.request, "title", ""), ValueAt(m.top.request, "message", ""), m.model, m.index))
end sub

function PlaceDialogMessage(theme as object, body as string, size as integer, width as integer, left as integer, top as integer, shown as integer, natural as integer, lines as integer) as integer
    m.textView.visible = shown > 0 and not IsBlank(body)
    if not m.textView.visible
        m.textBar.visible = false
        return top
    end if

    m.message.text = body
    m.message.color = theme.muted
    m.message.font = SizedFont(size)
    m.message.width = width
    m.message.height = natural
    m.message.wrap = lines > 1
    m.message.maxLines = lines
    m.message.lineSpacing = TextLineSpacing()
    m.message.translation = [left, top - m.textOffset]

    m.textView.clippingRect = [left, top, width, shown]

    m.textBar.theme = theme
    m.textBar.trackHeight = shown
    m.textBar.viewportHeight = shown
    m.textBar.contentHeight = natural
    m.textBar.offset = m.textOffset
    m.textBar.translation = [left + width + SpacingScale().s1, top]

    return top + shown + SpacingScale().s3
end function

function PlaceDialogTitle(theme as object, size as integer, width as integer, left as integer, top as integer) as integer
    glyph = TitleGlyph()
    lead = TitleGlyphLead()
    icon = ControlIconSize()

    m.titleIcon.visible = not IsBlank(glyph)
    if m.titleIcon.visible
        m.titleIcon.uri = glyph
        m.titleIcon.width = icon
        m.titleIcon.height = icon
        m.titleIcon.blendColor = theme.text
        m.titleIcon.translation = [left, top + Int((FontLineHeight(size) - icon) / 2)]
    end if

    return PlaceHeading(m.title, ValueAt(m.top.request, "title", ""), size, true, theme.text, width - lead, left + lead, top, m.titleLines)
end function

function PlaceHeading(node as object, text as dynamic, size as integer, bold as boolean, color as string, width as integer, left as integer, top as integer, lines as integer) as integer
    value = TextOrBlank(text)
    node.visible = not IsBlank(value)
    if not node.visible then return top

    node.text = value
    node.color = color
    if bold
        node.font = SizedBoldFont(size)
    else
        node.font = SizedFont(size)
    end if
    node.width = width
    node.height = TextBlockHeight(size, lines)
    node.wrap = lines > 1
    node.maxLines = lines
    node.ellipsisText = "…"
    node.translation = [left, top]

    return top + node.height + SpacingScale().s3
end function

function RowSpans(rows as object, space as object) as object
    tops = []
    heights = []
    height = DialogRowHeight()
    cursor = 0

    for index = 0 to rows.Count() - 1
        if index > 0
            if ValueAt(rows[index], "cancel", false) = true
                cursor = cursor + space.s5
            else
                cursor = cursor + space.s1
            end if
        end if

        tops.Push(cursor)
        heights.Push(height)
        cursor = cursor + height
    end for

    return { tops: tops, heights: heights, content: cursor }
end function

sub PaintRow(slot as object, row as object, theme as object, width as integer, top as integer, focused as boolean)
    space = SpacingScale()
    height = DialogRowHeight()
    inset = space.s4
    icon = ControlIconSize()
    paint = DialogRowPaint(theme, row, focused)

    slot.group.translation = [0, top]

    slot.rule.visible = ValueAt(row, "cancel", false) = true
    if slot.rule.visible
        slot.rule.width = width
        slot.rule.height = BorderThickness()
        slot.rule.color = theme.border
        slot.rule.translation = [0, - space.s3]
    end if

    slot.box.boxWidth = width
    slot.box.boxHeight = height
    slot.box.fillColor = paint.fill
    slot.box.lineColor = ""

    glyph = GlyphUri(TextOrBlank(ValueAt(row, "icon", "")))
    slot.icon.visible = not IsBlank(glyph)
    if slot.icon.visible
        slot.icon.uri = glyph
        slot.icon.width = icon
        slot.icon.height = icon
        slot.icon.blendColor = paint.glyph
        slot.icon.translation = [inset, Int((height - icon) / 2)]
    end if

    left = inset
    if ValueAt(row, "reserve", false) = true then left = inset + icon + space.s2

    edge = width - inset
    chevron = ChevronSize()

    slot.chevron.visible = ValueAt(row, "chevron", false) = true
    if slot.chevron.visible
        slot.chevron.uri = ChevronUri("right")
        slot.chevron.width = chevron
        slot.chevron.height = chevron
        slot.chevron.blendColor = paint.glyph
        slot.chevron.translation = [edge - chevron, Int((height - chevron) / 2)]
        edge = edge - chevron - space.s2
    end if

    size = TypeScale().textBase
    detail = TextOrBlank(ValueAt(row, "detail", ""))
    span = edge - left

    slot.detail.visible = not IsBlank(detail)
    if slot.detail.visible
        slot.detail.text = detail
        slot.detail.font = SizedFont(size)
        slot.detail.color = paint.detail
        slot.detail.width = span
        slot.detail.height = height
        slot.detail.maxLines = 1
        slot.detail.ellipsisText = "…"
        slot.detail.vertAlign = "center"
        slot.detail.horizAlign = "right"
        slot.detail.translation = [left, 0]
        span = span - TextWidth(detail, size) - space.s4
    end if

    slot.label.text = TextOrBlank(ValueAt(row, "label", ""))
    slot.label.font = SizedFont(size)
    slot.label.color = paint.label
    slot.label.width = span
    slot.label.height = height
    slot.label.maxLines = 1
    slot.label.ellipsisText = "…"
    slot.label.vertAlign = "center"
    slot.label.horizAlign = "left"
    if ValueAt(row, "alignRight", false) = true then slot.label.horizAlign = "right"
    slot.label.translation = [left, 0]
end sub

sub RebuildRows(count as integer)
    if m.built.Count() = count then return

    while m.rowGroup.GetChildCount() > 0
        m.rowGroup.RemoveChildIndex(0)
    end while
    m.built = []

    for index = 0 to count - 1
        holder = m.rowGroup.CreateChild("Group")
        m.built.Push({
            group: holder,
            rule: holder.CreateChild("Rectangle"),
            box: holder.CreateChild("RoundedBox"),
            icon: holder.CreateChild("Poster"),
            chevron: holder.CreateChild("Poster"),
            detail: holder.CreateChild("Label"),
            label: holder.CreateChild("Label")
        })
    end for
end sub

function ScrollsText() as boolean
    if type(m.model) <> "roArray" or m.model.Count() <> 1 then return false

    return m.textNatural > m.textShown
end function

sub ScrollText(direction as integer)
    at = ScrolledTextOffset(m.textOffset, direction, m.textShown, m.textNatural, TextLinePitch(TypeScale().textBase))
    if at = m.textOffset then return

    m.textOffset = at
    render()
end sub

sub MoveSelection(direction as integer)
    wanted = SteppedRowIndex(m.model, m.index, direction)
    if wanted = m.index then return

    m.index = wanted
    render()
end sub

sub DeliverIndex(index as integer)
    m.top.visible = false
    m.model = []
    m.top.result = DialogChoiceResult(m.field, index, m.count)
end sub

function onKeyEvent(key as string, press as boolean) as boolean
    if not m.top.visible then return false
    if not KeysAreOurs() then return false
    if not press then return false
    if type(m.model) <> "roArray" or m.model.Count() = 0 then return true

    if key = "up"
        if ScrollsText()
            ScrollText(-1)
        else
            MoveSelection(-1)
        end if
        return true
    end if

    if key = "down"
        if ScrollsText()
            ScrollText(1)
        else
            MoveSelection(1)
        end if
        return true
    end if

    if key = "OK"
        DeliverIndex(m.index)
        return true
    end if

    if key = "back"
        DeliverIndex(m.count)
        return true
    end if

    return true
end function
