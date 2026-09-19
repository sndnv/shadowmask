function SendRequest(request as object, callback as string) as object
    task = CreateObject("roSGNode", "HttpTask")
    task.ObserveField("response", "onRequestFinished")
    task.ObserveField("response", callback)
    task.request = request

    m.httpQueue = QueuedRequests(m.httpQueue, task)
    DrainRequests()
    return task
end function

sub onRequestFinished()
    DrainRequests()
end sub

sub DrainRequests()
    m.httpActive = LiveRequests(m.httpActive)

    plan = NextRequests(m.httpActive.Count(), m.httpQueue, MaxConcurrentRequests())
    m.httpQueue = plan.queue

    for each task in plan.start
        m.httpActive.Push(task)
        task.control = "RUN"
    end for
end sub
