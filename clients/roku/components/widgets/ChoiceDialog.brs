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
    m.spinner = m.top.FindNode("spinner")
    m.spinnerTurn = m.top.FindNode("spinnerTurn")
    m.spin = m.top.FindNode("spin")

    m.busy = false
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
        StopSpinner()
        m.top.visible = false
        return
    end if

    kind = TextOrBlank(ValueAt(request, "kind", "confirm"))
    selected = DialogStartIndex(request, m.top.visible, m.field, m.index, options.Count())

    m.field = TextOrBlank(ValueAt(request, "field", ""))
    m.count = options.Count()
    m.model = DialogRows(options, kind, selected)
    m.index = FocusableRowIndex(m.model, selected)
    m.offset = 0
    m.textOffset = 0
    StopSpinner()

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

    rows = m.model
    if m.busy then rows = DialogBusyRows(m.model, m.index, TextOrBlank(ValueAt(m.top.request, "busy", "")))

    RebuildRows(rows.Count())
    for index = 0 to rows.Count() - 1
        PaintRow(m.built[index], rows[index], theme, inner, plan.tops[index], index = m.index)
    end for

    m.offset = RevealOffset(m.index, plan.tops, plan.heights, shown, m.offset, 0)

    m.viewport.clippingRect = [pad, cursor, inner, shown]
    m.rowGroup.translation = [pad, cursor - m.offset]
    PlaceSpinner(theme, inner, pad, cursor - m.offset + plan.tops[m.index])

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

    Speak(DialogSpeech(ValueAt(m.top.request, "title", ""), ValueAt(m.top.request, "message", ""), rows, m.index))
end sub

sub PlaceSpinner(theme as object, width as integer, left as integer, top as integer)
    if not m.busy
        StopSpinner()
        return
    end if

    size = ChevronSize()
    m.spinner.uri = GlyphUri("icon-replay")
    m.spinner.width = size
    m.spinner.height = size
    m.spinner.scaleRotateCenter = [size / 2, size / 2]
    m.spinner.scale = [-1, 1]
    m.spinner.blendColor = theme.accentContrast
    m.spinnerTurn.scaleRotateCenter = [size / 2, size / 2]
    m.spinnerTurn.translation = [left + width - SpacingScale().s4 - size, top + Int((DialogRowHeight() - size) / 2)]
    m.spinnerTurn.visible = true
    if m.spin.state <> "running" then m.spin.control = "start"
end sub

sub StopSpinner()
    m.busy = false
    m.spin.control = "stop"
    m.spinnerTurn.visible = false
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
            if ValueAt(rows[index], "cancel", false) = true or ValueAt(rows[index], "heading", false) = true
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

    slot.rule.visible = ValueAt(row, "cancel", false) = true or ValueAt(row, "heading", false) = true
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
    if ValueAt(row, "spinning", false) = true then edge = edge - chevron - space.s2

    size = TypeScale().textBase
    detail = TextOrBlank(ValueAt(row, "detail", ""))
    label = TextOrBlank(ValueAt(row, "label", ""))
    span = edge - left
    column = ValueAt(row, "column", false) = true
    room = DialogDetailWidth(span - space.s4, TextWidth(detail, size) + space.s1, TextWidth(label, size))
    at = left + span - room
    align = "right"
    if column
        spans = DialogColumnSpans(span, space.s4)
        room = spans.detail
        at = left + spans.at
        align = "left"
    end if

    slot.detail.visible = not IsBlank(detail) and room > 0
    if slot.detail.visible
        slot.detail.text = detail
        slot.detail.font = SizedFont(size)
        slot.detail.color = paint.detail
        slot.detail.width = room
        slot.detail.height = height
        slot.detail.maxLines = 1
        slot.detail.ellipsisText = "…"
        slot.detail.vertAlign = "center"
        slot.detail.horizAlign = align
        slot.detail.translation = [at, 0]
        span = at - left - space.s4
    end if
    PlaceDetailTicker(slot, detail, paint, size, height, room, at, focused and column and slot.detail.visible)

    span = PlaceMeta(slot, row, paint, size, height, left, span)

    slot.label.text = label
    slot.label.font = SizedFont(size)
    if ValueAt(row, "heading", false) = true then slot.label.font = SizedBoldFont(size)
    slot.label.color = paint.label
    slot.label.width = span
    slot.label.height = height
    slot.label.maxLines = 1
    slot.label.ellipsisText = "…"
    slot.label.vertAlign = "center"
    slot.label.horizAlign = "left"
    if ValueAt(row, "alignRight", false) = true then slot.label.horizAlign = "right"
    slot.label.translation = [left, 0]

    ticking = focused and ValueAt(row, "alignRight", false) <> true
    slot.label.visible = not ticking
    slot.ticker.visible = ticking
    if not ticking then return

    if slot.ticker.text <> slot.label.text then slot.ticker.text = slot.label.text
    slot.ticker.font = slot.label.font
    slot.ticker.color = paint.label
    slot.ticker.maxWidth = span
    slot.ticker.height = height
    slot.ticker.vertAlign = "center"
    slot.ticker.repeatCount = -1
    slot.ticker.scrollSpeed = TickerSpeed()
    slot.ticker.translation = [left, 0]
end sub

sub PlaceDetailTicker(slot as object, detail as string, paint as object, size as integer, height as integer, room as integer, at as integer, scrolling as boolean)
    slot.detailTicker.visible = scrolling
    if not scrolling then return

    slot.detail.visible = false
    if slot.detailTicker.text <> detail then slot.detailTicker.text = detail
    slot.detailTicker.font = SizedFont(size)
    slot.detailTicker.color = paint.detail
    slot.detailTicker.maxWidth = room
    slot.detailTicker.height = height
    slot.detailTicker.vertAlign = "center"
    slot.detailTicker.repeatCount = -1
    slot.detailTicker.scrollSpeed = TickerSpeed()
    slot.detailTicker.translation = [at, 0]
end sub

function PlaceMeta(slot as object, row as object, paint as object, size as integer, height as integer, left as integer, span as integer) as integer
    space = SpacingScale()
    meta = TextOrBlank(ValueAt(row, "meta", ""))
    glyph = GlyphUri(TextOrBlank(ValueAt(row, "metaIcon", "")))

    slot.meta.visible = not IsBlank(meta)
    slot.metaIcon.visible = slot.meta.visible and not IsBlank(glyph)
    if not slot.meta.visible then return span

    right = left + span
    width = TextWidth(meta, size) + space.s1
    slot.meta.text = meta
    slot.meta.font = SizedFont(size)
    slot.meta.color = paint.detail
    slot.meta.width = width
    slot.meta.height = height
    slot.meta.maxLines = 1
    slot.meta.vertAlign = "center"
    slot.meta.horizAlign = "right"
    slot.meta.translation = [right - width, 0]
    used = width

    if slot.metaIcon.visible
        icon = InlineIconSize()
        slot.metaIcon.uri = glyph
        slot.metaIcon.width = icon
        slot.metaIcon.height = icon
        slot.metaIcon.blendColor = paint.detail
        slot.metaIcon.translation = [right - used - space.s1 - icon, Int((height - icon) / 2)]
        used = used + space.s1 + icon
    end if

    return span - used - space.s4
end function

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
            metaIcon: holder.CreateChild("Poster"),
            meta: holder.CreateChild("Label"),
            label: holder.CreateChild("Label"),
            ticker: holder.CreateChild("ScrollingLabel"),
            detailTicker: holder.CreateChild("ScrollingLabel")
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
    StopSpinner()
    m.top.visible = false
    m.model = []
    m.top.result = DialogChoiceResult(m.field, index, m.count)
end sub

sub PickRow(index as integer)
    busy = TextOrBlank(ValueAt(m.top.request, "busy", ""))
    if IsBlank(busy) or index >= m.count
        DeliverIndex(index)
        return
    end if

    m.busy = true
    render()
    m.top.result = DialogPickResult(m.field, index, m.count, true)
end sub

function onKeyEvent(key as string, press as boolean) as boolean
    if not m.top.visible then return false
    if not KeysAreOurs() then return false
    if not press then return false
    if type(m.model) <> "roArray" or m.model.Count() = 0 then return true

    if m.busy
        if key = "back" then DeliverIndex(m.count)
        return true
    end if

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
        if DialogRowSelectable(m.model[m.index]) then PickRow(m.index)
        return true
    end if

    if key = "back"
        DeliverIndex(m.count)
        return true
    end if

    return true
end function
