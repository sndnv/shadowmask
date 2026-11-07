function OpenedCardMenu(card as dynamic, listKind = "" as dynamic) as boolean
    m.menuActions = CardMenuActions(card, listKind)
    if m.menuActions.Count() = 0 then return false

    m.menuCard = card
    m.menuAction = ""

    m.top.choiceRequest = {
        field: "cardMenu",
        title: CardMenuTitle(card),
        kind: "menu",
        options: MenuOptions(m.menuActions),
        selected: 0
    }
    return true
end function

function HandledCardMenuChoice(result as dynamic) as boolean
    if result = invalid then return false

    if TextOrBlank(ValueAt(result, "field", "")) = "cardWatched"
        if WatchedConfirmAccepted(result) then CommitCardWatched()
        return true
    end if

    if TextOrBlank(ValueAt(result, "field", "")) = "clearHistory"
        if ConfirmAccepted(result) then CommitClearHistory()
        return true
    end if

    if TextOrBlank(ValueAt(result, "field", "")) = "pickPlaylist"
        if ValueAt(result, "cancelled", false) <> true then PickedPlaylistAt(Int(ValueAt(result, "index", 0)))
        return true
    end if

    if result.cancelled then return false
    if result.field <> "cardMenu" then return false
    if type(m.menuActions) <> "roArray" or m.menuActions.Count() = 0 then return false

    action = m.menuActions[ClampInt(result.index, 0, m.menuActions.Count() - 1)]
    RunCardMenuAction(action.id)
    return true
end function

sub CommitCardWatched()
    card = m.menuCard
    if card = invalid then return

    session = SessionFor(m.global)
    kind = TextOrBlank(ValueAt(card, "kind", ""))
    titleId = TextOrBlank(ValueAt(card, "id", ""))
    m.menuTask = SendRequest(SetWatchedRequest(session.serverUrl, session.token, session.userId, kind, titleId, m.pendingCardWatched), "onCardMenuDone")
end sub

sub RunCardMenuAction(id as string)
    session = SessionFor(m.global)
    card = m.menuCard
    m.menuAction = id
    kind = TextOrBlank(ValueAt(card, "kind", ""))
    titleId = TextOrBlank(ValueAt(card, "id", ""))
    versionId = TextOrBlank(ValueAt(card, "versionId", ""))

    if id = "play"
        if IsBlank(versionId)
            AskCardVersions(card, session)
            return
        end if

        PlayCardVersion(card, versionId)
        return
    end if

    if id = "dismissResume"
        m.menuTask = SendRequest(ClearProgressRequest(session.serverUrl, session.token, session.userId, versionId), "onCardMenuDone")
        return
    end if

    if id = "randomInCollection"
        m.menuTask = SendRequest(RandomInCollectionRequest(session.serverUrl, session.token, titleId), "onCardRandom")
        return
    end if

    if id = "watched"
        wanted = not (ValueAt(card, "watched", false) = true)
        if NeedsWatchedConfirm(kind)
            m.pendingCardWatched = wanted
            m.top.choiceRequest = {
                field: "cardWatched",
                title: Phrase("action.watchedLabel"),
                message: WatchedConfirmTitle(kind, wanted),
                options: WatchedConfirmOptions(wanted),
                selected: 0
            }
            return
        end if

        m.menuTask = SendRequest(SetWatchedRequest(session.serverUrl, session.token, session.userId, kind, titleId, wanted), "onCardMenuDone")
        return
    end if

    if id = "watchlist"
        if ValueAt(card, "watchlisted", false) = true
            m.menuTask = SendRequest(WatchlistRemoveRequest(session.serverUrl, session.token, session.userId, titleId), "onCardMenuDone")
        else
            m.menuTask = SendRequest(WatchlistAddRequest(session.serverUrl, session.token, session.userId, kind, titleId), "onCardMenuDone")
        end if
        return
    end if

    if id = "favorite"
        if ValueAt(card, "favorite", false) = true
            m.menuTask = SendRequest(FavoriteRemoveRequest(session.serverUrl, session.token, session.userId, titleId), "onCardMenuDone")
        else
            m.menuTask = SendRequest(FavoriteAddRequest(session.serverUrl, session.token, session.userId, kind, titleId), "onCardMenuDone")
        end if
        return
    end if

    if id = "removeHistory"
        m.menuTask = SendRequest(HistoryRemoveRequest(session.serverUrl, session.token, session.userId, titleId), "onCardMenuDone")
        return
    end if

    if id = "dequeue"
        entryId = TextOrBlank(ValueAt(card, "queueEntryId", ""))
        m.menuTask = SendRequest(QueueRemoveRequest(session.serverUrl, session.token, session.userId, entryId), "onCardMenuDone")
        return
    end if

    if id = "playlist"
        AskPlaylistFor(card)
        return
    end if

    if id = "clearHistory"
        m.top.choiceRequest = {
            field: "clearHistory",
            title: Phrase("action.clearHistory"),
            message: Phrase("confirm.clearHistory"),
            options: ConfirmOptions(Phrase("action.clearHistory"), true),
            selected: 1
        }
        return
    end if
end sub

sub PlayCardVersion(card as dynamic, versionId as string)
    m.top.advanceTarget = {
        versionId: versionId,
        title: TextOrBlank(ValueAt(card, "title", "")),
        percent: Int(ValueAt(card, "progressPercent", 0)),
        queueEntryId: TextOrBlank(ValueAt(card, "queueEntryId", "")),
        episode: {
            id: TextOrBlank(ValueAt(card, "id", "")),
            seriesId: TextOrBlank(ValueAt(card, "seriesId", "")),
            seasonId: TextOrBlank(ValueAt(card, "seasonId", ""))
        }
    }
    m.top.advance = "WatchScreen"
end sub

sub onCardRandom(event as object)
    m.menuTask = invalid
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return

    versionId = TextOrBlank(ValueAt(parsed.json, "version_id", ""))
    if not parsed.ok or IsBlank(versionId)
        RaiseToast("err", Phrase("error.couldNotPickRandom"))
        return
    end if

    m.top.advanceTarget = { versionId: versionId, resolve: true, percent: 0 }
    m.top.advance = "WatchScreen"
end sub

sub AskCardVersions(card as object, session as object)
    m.menuPlayCard = card
    m.menuVersions = invalid
    m.menuResumable = invalid
    m.menuResumeTask = SendRequest(ContinueRequest(session.serverUrl, session.token, session.userId), "onCardResume")
    m.menuVersionsTask = SendRequest(CardVersionsRequest(session.serverUrl, session.token, card), "onCardVersions")
end sub

sub onCardVersions(event as object)
    if m.menuVersionsTask = invalid or not m.menuVersionsTask.IsSameNode(event.GetRoSGNode()) then return
    m.menuVersionsTask = invalid
    parsed = ParseResponse(event.GetData().status, event.GetData().body)

    m.menuVersions = []
    if parsed.ok then m.menuVersions = AvailableVersions(OrderedVersions(ListItems(parsed.json)))
    FinishCardPlay()
end sub

sub onCardResume(event as object)
    if m.menuResumeTask = invalid or not m.menuResumeTask.IsSameNode(event.GetRoSGNode()) then return
    m.menuResumeTask = invalid
    parsed = ParseResponse(event.GetData().status, event.GetData().body)

    m.menuResumable = {}
    m.menuFeedOk = parsed.ok
    if parsed.ok then m.menuResumable = ResumeProgressMap(ContinueCards(parsed.json))
    FinishCardPlay()
end sub

sub FinishCardPlay()
    if m.menuVersions = invalid or m.menuResumable = invalid then return

    card = m.menuPlayCard
    versions = m.menuVersions
    chosen = ResolvePlayTarget(versions, m.menuResumable)
    m.menuPlayCard = invalid
    m.menuVersions = invalid
    m.menuResumable = invalid
    if chosen = invalid
        RaiseToast("err", Phrase("empty.nothingToPlay"))
        return
    end if
    if not m.menuFeedOk and versions.Count() > 1
        RaiseActionFailed()
        return
    end if
    PlayCardVersion(card, TextOrBlank(ValueAt(chosen, "id", "")))
end sub

sub AskPlaylistFor(card as dynamic)
    if m.pickTask <> invalid then return

    m.pickCard = card
    session = SessionFor(m.global)
    m.pickTask = SendRequest(PlaylistsRequest(session.serverUrl, session.token, session.userId), "onPickPlaylists")
end sub

sub onPickPlaylists(event as object)
    m.pickTask = invalid
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return

    if not parsed.ok
        RaiseActionFailed()
        return
    end if

    m.pickPlaylists = ListItems(parsed.json)
    m.top.choiceRequest = {
        field: "pickPlaylist",
        title: Phrase("action.addToPlaylist"),
        kind: "menu",
        options: MenuOptions(PlaylistPickOptions(m.pickPlaylists)),
        selected: 0
    }
end sub

sub PickedPlaylistAt(index as integer)
    picked = PickedPlaylist(m.pickPlaylists, index)
    if picked <> invalid
        AddToPickedPlaylist(picked)
        return
    end if

    m.top.keyboardRequest = { field: "pickNewPlaylist", title: Phrase("action.newPlaylist"), text: "" }
end sub

function HandledCardMenuKeyboard(result as dynamic) as boolean
    if TextOrBlank(ValueAt(result, "field", "")) <> "pickNewPlaylist" then return false
    if ValueAt(result, "cancelled", true) = true or m.pickTask <> invalid then return true

    name = TextOrBlank(ValueAt(result, "text", "")).Trim()
    if IsBlank(name) then return true

    session = SessionFor(m.global)
    m.pickTask = SendRequest(PlaylistCreateRequest(session.serverUrl, session.token, session.userId, name), "onPickCreated")
    return true
end function

sub onCardMenuKeyboard()
    HandledCardMenuKeyboard(m.top.keyboardResult)
end sub

sub onPickCreated(event as object)
    m.pickTask = invalid
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return

    if not parsed.ok
        RaiseActionFailed()
        return
    end if

    AddToPickedPlaylist(parsed.json)
end sub

sub AddToPickedPlaylist(playlist as dynamic)
    if m.pickTask <> invalid then return

    m.pickTarget = playlist
    session = SessionFor(m.global)
    playlistId = TextOrBlank(ValueAt(playlist, "id", ""))
    kind = TextOrBlank(ValueAt(m.pickCard, "kind", ""))
    titleId = TextOrBlank(ValueAt(m.pickCard, "id", ""))
    m.pickTask = SendRequest(PlaylistAddRequest(session.serverUrl, session.token, session.userId, playlistId, kind, titleId), "onPlaylistAdded")
end sub

sub onPlaylistAdded(event as object)
    m.pickTask = invalid
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return

    if m.DoesExist("reloadOnPlaylistAdd") then ReloadCardStates()
    if not parsed.ok
        RaiseActionFailed()
        return
    end if

    RaiseToast("ok", AddedToPlaylistText(ValueAt(parsed.json, "added", 0), ValueAt(m.pickTarget, "name", "")))
end sub

sub CommitClearHistory()
    session = SessionFor(m.global)
    m.menuTask = SendRequest(HistoryClearRequest(session.serverUrl, session.token, session.userId), "onCardMenuDone")
end sub

sub onCardMenuDone(event as object)
    parsed = ParseResponse(event.GetData().status, event.GetData().body)
    if HandledUnauthorised(m.top, parsed.status) then return

    if not parsed.ok
        RaiseActionFailed()
        return
    end if

    if m.menuAction = "removeHistory"
        RaiseToast("ok", Phrase("toast.ok.historyRemoved"))
    else if m.menuAction = "clearHistory"
        RaiseToast("ok", Phrase("toast.ok.historyCleared"))
    else if m.menuAction = "dequeue"
        RaiseToast("ok", PhraseWith("toast.ok.removedFromQueue", { title: TextOrBlank(ValueAt(m.menuCard, "title", "")) }))
    else
        RaiseToast("ok", PhraseWith("toast.ok.updated", { title: TextOrBlank(ValueAt(m.menuCard, "title", "")) }))
    end if
    ReloadCardStates()
end sub
