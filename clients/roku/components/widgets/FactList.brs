sub init()
    m.entries = m.top.FindNode("entries")
end sub

sub render()
    theme = m.top.theme
    rows = m.top.rows
    if theme = invalid or theme.Count() = 0 then return
    if type(rows) <> "roArray" then return

    while m.entries.GetChildCount() > 0
        m.entries.RemoveChildIndex(0)
    end while

    if rows.Count() = 0
        m.top.listHeight = 0
        return
    end if

    space = SpacingScale()
    columns = ClampInt(Int(m.top.columns), 1, rows.Count())
    width = FittedColumnWidth(Int(m.top.listWidth), columns, space.s5)

    tallest = 0
    at = 0
    for each column in SplitEvenly(rows, columns)
        offset = 0
        for each row in column
            offset = DrawEntry(row, theme, width, at * (width + space.s5), offset)
        end for
        if offset > tallest then tallest = offset
        at = at + 1
    end for

    m.top.listHeight = tallest
end sub

function DrawEntry(row as object, theme as object, width as integer, left as integer, offset as integer) as integer
    sizes = TypeScale()

    label = m.entries.CreateChild("Label")
    label.text = TextOrBlank(ValueAt(row, "label", ""))
    label.color = theme.muted
    label.font = SizedFont(sizes.textXs)
    label.width = width
    label.maxLines = 1
    label.ellipsisText = "…"
    label.translation = [left, offset]

    at = offset + TextLinePitch(sizes.textXs)

    values = ValueAt(row, "values", invalid)
    if type(values) = "roArray"
        for each value in values
            line = m.entries.CreateChild("Label")
            line.text = TextOrBlank(value)
            line.color = theme.text
            line.font = SizedBoldFont(sizes.textSm)
            line.width = width
            line.maxLines = 1
            line.ellipsisText = "…"
            line.translation = [left, at]
            at = at + TextLinePitch(sizes.textSm)
        end for
    end if

    return at + SpacingScale().s3
end function
