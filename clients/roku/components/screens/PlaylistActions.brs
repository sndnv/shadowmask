function PlaylistBusy() as boolean
    return m.playlistTask <> invalid
end function

sub RunPlaylistAction(id as string, playlist as object)
    if PlaylistBusy() then return

    session = SessionFor(m.global)
    playlistId = TextOrBlank(ValueAt(playlist, "id", ""))
    name = TextOrBlank(ValueAt(playlist, "title", ""))
    m.actionPlaylist = playlist

    if id = "queue" or id = "shuffleQueue"
        m.playlistTask = SendRequest(QueueAddRequest(session.serverUrl, session.token, session.userId, playlistId, id = "shuffleQueue"), "onPlaylistQueued")
        return
    end if

    if id = "shuffle"
        m.playlistTask = SendRequest(PlaylistRequest(session.serverUrl, session.token, session.userId, playlistId), "onPlaylistShuffle")
        return
    end if

    if id = "rename"
        m.top.keyboardRequest = { field: "renamePlaylist", title: Phrase("action.renamePlaylist"), text: name }
        return
    end if

    if id = "delete"
        m.top.choiceRequest = {
            field: "deletePlaylist",
            title: Phrase("action.deletePlaylist"),
            message: PhraseWith("confirm.deletePlaylist", { name: name }),
            options: ConfirmOptions(Phrase("action.deletePlaylist"), true),
            selected: 1
        }
    end if
end sub

sub AskNewPlaylist()
    if PlaylistBusy() then return
    m.top.keyboardRequest = { field: "newPlaylist", title: Phrase("action.newPlaylist"), text: "" }
end sub

function HandledPlaylistMenu(result as dynamic) as boolean
    if result = invalid or result.cancelled then return false
    if TextOrBlank(ValueAt(result, "field", "")) <> "cardMenu" then return false
    if TextOrBlank(ValueAt(m.menuCard, "kind", "")) <> "playlist" then return false
    if type(m.menuActions) <> "roArray" or m.menuActions.Count() = 0 then return false

    action = m.menuActions[ClampInt(result.index, 0, m.menuActions.Count() - 1)]
    RunPlaylistAction(action.id, m.menuCard)
    return true
end function

function HandledPlaylistChoice(result as dynamic) as boolean
    if TextOrBlank(ValueAt(result, "field", "")) <> "deletePlaylist" then return false
    if not ConfirmAccepted(result) or PlaylistBusy() then return true

    session = SessionFor(m.global)
    playlistId = TextOrBlank(ValueAt(m.actionPlaylist, "id", ""))
    m.playlistTask = SendRequest(PlaylistDeleteRequest(session.serverUrl, session.token, session.userId, playlistId), "onPlaylistDeleted")
    return true
end function

function HandledPlaylistKeyboard(result as dynamic) as boolean
    field = TextOrBlank(ValueAt(result, "field", ""))
    if field <> "renamePlaylist" and field <> "newPlaylist" then return false
    if ValueAt(result, "cancelled", true) = true or PlaylistBusy() then return true

    name = TextOrBlank(ValueAt(result, "text", "")).Trim()
    if IsBlank(name) then return true

    session = SessionFor(m.global)
    m.playlistName = name
    if field = "newPlaylist"
        m.playlistTask = SendRequest(PlaylistCreateRequest(session.serverUrl, session.token, session.userId, name), "onPlaylistCreated")
        return true
    end if

    playlistId = TextOrBlank(ValueAt(m.actionPlaylist, "id", ""))
    m.playlistTask = SendRequest(PlaylistRenameRequest(session.serverUrl, session.token, session.userId, playlistId, name), "onPlaylistRenamed")
    return true
end function

function PlaylistAnswer(event as object) as dynamic
    m.playlistTask = invalid
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return invalid
    if not parsed.ok
        RaiseActionFailed()
        return invalid
    end if
    return parsed
end function

sub onPlaylistQueued(event as object)
    parsed = PlaylistAnswer(event)
    if parsed = invalid then return

    RaiseToast("ok", AddedToQueueText(ValueAt(parsed.json, "added", 0)))
    PlaylistQueued()
end sub

sub onPlaylistShuffle(event as object)
    parsed = PlaylistAnswer(event)
    if parsed = invalid then return

    cards = PlaylistItemCards(parsed.json)
    if cards.Count() = 0
        RaiseToast("err", Phrase("empty.nothingToPlay"))
        return
    end if

    AskCardVersions(cards[Rnd(cards.Count()) - 1], SessionFor(m.global))
end sub

sub onPlaylistCreated(event as object)
    parsed = PlaylistAnswer(event)
    if parsed = invalid then return

    PlaylistCreated()
end sub

sub onPlaylistRenamed(event as object)
    parsed = PlaylistAnswer(event)
    if parsed = invalid then return

    PlaylistRenamed()
end sub

sub onPlaylistDeleted(event as object)
    parsed = PlaylistAnswer(event)
    if parsed = invalid then return

    RaiseToast("ok", PhraseWith("toast.ok.playlistDeleted", { name: TextOrBlank(ValueAt(m.actionPlaylist, "title", "")) }))
    PlaylistDeleted()
end sub
