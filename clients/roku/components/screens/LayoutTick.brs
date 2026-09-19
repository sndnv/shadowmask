sub InitLayoutTick()
    m.layoutTick = m.top.FindNode("layoutTick")
    if m.layoutTick = invalid then return

    m.layoutTick.duration = LayoutSettleSeconds()
    m.layoutTick.repeat = false
    m.layoutTick.ObserveField("fire", "onLayoutTick")
end sub

sub CoalesceLayout()
    if m.layoutTick = invalid or not m.published
        PaintScreen()
        return
    end if

    m.layoutTick.control = "stop"
    m.layoutTick.control = "start"
end sub

sub onLayoutTick()
    PaintScreen()
end sub
