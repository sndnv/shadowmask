function ArmTimer(seconds as float, callback as string, data as object) as object
    timer = CreateObject("roSGNode", "Timer")
    timer.AddFields({ data: data })
    timer.repeat = false
    timer.duration = seconds
    timer.ObserveField("fire", callback)
    m.top.AppendChild(timer)
    timer.control = "start"
    return timer
end function

sub StopTimer(timer as dynamic)
    if timer = invalid then return

    timer.control = "stop"
    timer.UnobserveField("fire")
    m.top.RemoveChild(timer)
end sub

function TimerData(event as object) as object
    return event.GetRoSGNode().data
end function
