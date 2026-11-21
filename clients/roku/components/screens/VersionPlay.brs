sub PlayChosenVersion()
    if m.available.Count() < 2
        PlayResolvedVersion()
        return
    end if
    if m.versionPlayTask <> invalid then return

    session = SessionFor(m.global)
    m.versionPlayTask = Ask(ContinueRequest(session.serverUrl, session.token, session.userId), "onVersionPlayResume")
end sub

sub onVersionPlayResume(event as object)
    if m.versionPlayTask = invalid or not m.versionPlayTask.IsSameNode(event.GetRoSGNode()) then return
    m.versionPlayTask = invalid

    parsed = Answered(event)
    if m.released or IsUnauthorised(parsed.status) then return

    if parsed.ok
        m.resumable = ResumeProgressMap(ContinueCards(parsed.json))
        if m.dismissTask <> invalid then m.resumable.Delete(m.dismissedVersion)
        Refresh()
    end if
    PlayResolvedVersion()
end sub

sub PlayResolvedVersion()
    target = ResolvePlayTarget(m.available, m.resumable)
    if target <> invalid then OpenWatch(ValueAt(target, "id", ""))
end sub

sub CheckQueueEntry()
    if IsBlank(m.queueEntryId) then return

    session = SessionFor(m.global)
    Ask(QueueRequest(session.serverUrl, session.token, session.userId), "onQueueChecked")
end sub

sub onQueueChecked(event as object)
    parsed = Answered(event)
    if m.released or not parsed.ok then return

    if not QueueHolds(ListItems(parsed.json), m.queueEntryId) then m.queueEntryId = ""
end sub
