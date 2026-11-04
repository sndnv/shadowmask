sub render()
    m.top.RemoveChildrenIndex(m.top.GetChildCount(), 0)

    lines = m.top.lines
    size = m.top.size
    if lines = invalid or lines.Count() = 0 or size <= 0 then return

    rows = []
    total = 0
    for each line in lines
        row = SubtitleRow(line, size)
        rows.Push(row)
        total = total + row.height
    end for

    top = CanvasHeight() - SpacingScale().s8 - total
    for each row in rows
        row.node.translation = [Int((CanvasWidth() - row.width) / 2), top]
        m.top.AppendChild(row.node)
        top = top + row.height
    end for
end sub

function SubtitleRow(line as object, size as integer) as object
    padX = Int(size * 0.3)
    padY = Int(size * 0.1)
    widest = Int(CanvasWidth() * 0.8)

    group = CreateObject("roSGNode", "Group")
    plate = group.CreateChild("Rectangle")
    label = group.CreateChild("Label")
    label.font = SubtitleFont(line, size)
    label.color = "0xFFFFFFFF"
    label.text = line.text

    bounds = label.boundingRect()
    width = Int(bounds.width)
    height = Int(bounds.height)
    wrapped = width > widest
    if wrapped
        label.width = widest
        label.wrap = true
        label.horizAlign = "center"
        width = widest
        height = Int(label.boundingRect().height)
    end if

    label.translation = [padX, padY]
    plate.color = "0x000000B3"
    plate.width = width + padX * 2
    plate.height = height + padY * 2

    if line.underline = true and not wrapped
        thickness = Int(size / 16)
        if thickness < 2 then thickness = 2
        rule = group.CreateChild("Rectangle")
        rule.color = "0xFFFFFFFF"
        rule.width = width
        rule.height = thickness
        rule.translation = [padX, padY + height - thickness]
    end if

    return { node: group, width: plate.width, height: plate.height }
end function

function SubtitleFont(line as object, size as integer) as object
    role = SubtitleFontRole(line, ItalicFontReady())
    if role = "italic" then return SizedItalicFont(size)
    if role = "medium" then return SizedBoldFont(size)
    return SizedFont(size)
end function
