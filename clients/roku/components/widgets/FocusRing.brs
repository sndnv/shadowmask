sub init()
    m.plate = m.top.FindNode("plate")
    m.edges = [
        m.top.FindNode("top"),
        m.top.FindNode("bottom"),
        m.top.FindNode("left"),
        m.top.FindNode("right")
    ]
end sub

sub render()
    theme = m.top.theme
    if theme = invalid or theme.Count() = 0 then return

    width = m.top.ringWidth
    height = m.top.ringHeight
    thickness = m.top.thickness

    m.plate.visible = not IsBlank(m.top.fill)
    m.plate.width = width
    m.plate.height = height
    m.plate.color = PlateColor(m.top.fill)

    layout = [
        { width: width, height: thickness, translation: [0, 0] },
        { width: width, height: thickness, translation: [0, height - thickness] },
        { width: thickness, height: height, translation: [0, 0] },
        { width: thickness, height: height, translation: [width - thickness, 0] }
    ]

    color = LineColor(theme)
    for index = 0 to m.edges.Count() - 1
        edge = m.edges[index]
        edge.width = layout[index].width
        edge.height = layout[index].height
        edge.translation = layout[index].translation
        edge.color = color
    end for
end sub

function LineColor(theme as object) as string
    if IsBlank(m.top.lineColor) then return theme.accent
    return m.top.lineColor
end function

function PlateColor(fill as string) as string
    if IsBlank(fill) then return "0x00000000"
    return fill
end function
