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

    if TextOrBlank(ValueAt(result, "field", "")) = "cardVersion"
        if result.cancelled then return true
        if type(m.menuVersions) <> "roArray" or m.menuVersions.Count() = 0 then return true

        chosen = m.menuVersions[ClampInt(result.index, 0, m.menuVersions.Count() - 1)]
        PlayCardVersion(TextOrBlank(ValueAt(chosen, "id", "")))
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

        PlayCardVersion(versionId)
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

sub PlayCardVersion(versionId as string)
    card = m.menuCard

    m.top.advanceTarget = {
        versionId: versionId,
        title: TextOrBlank(ValueAt(card, "title", "")),
        percent: Int(ValueAt(card, "progressPercent", 0)),
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
    titleId = TextOrBlank(ValueAt(card, "id", ""))
    seriesId = TextOrBlank(ValueAt(card, "seriesId", ""))
    seasonId = TextOrBlank(ValueAt(card, "seasonId", ""))

    if IsBlank(seriesId) or IsBlank(seasonId)
        m.menuTask = SendRequest(MovieVersionsRequest(session.serverUrl, session.token, titleId), "onCardVersions")
        return
    end if

    m.menuTask = SendRequest(EpisodeVersionsRequest(session.serverUrl, session.token, seriesId, seasonId, titleId), "onCardVersions")
end sub

sub onCardVersions(event as object)
    m.menuTask = invalid
    parsed = ParseResponse(event.GetData().status, event.GetData().body)

    m.menuVersions = []
    if parsed.ok then m.menuVersions = AvailableVersions(OrderedVersions(ListItems(parsed.json)))

    if m.menuVersions.Count() = 0
        RaiseToast("err", Phrase("empty.nothingToPlay"))
        return
    end if

    if m.menuVersions.Count() = 1
        PlayCardVersion(TextOrBlank(ValueAt(m.menuVersions[0], "id", "")))
        return
    end if

    rows = VersionRows(m.menuVersions)
    options = []
    for each row in rows
        options.Push({ label: row.label, detail: row.detail })
    end for

    m.top.choiceRequest = { field: "cardVersion", title: Phrase("detail.chooseVersion"), kind: "choice", options: options, selected: 0 }
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
    else
        RaiseToast("ok", PhraseWith("toast.ok.updated", { title: TextOrBlank(ValueAt(m.menuCard, "title", "")) }))
    end if
    ReloadCardStates()
end sub
