sub PlayEpisode(episode as dynamic, seriesId as dynamic, seasonId as dynamic, seasonNumber as dynamic, seriesTitle as dynamic)
    if episode = invalid
        m.episodePlay = invalid
        RaiseToast("err", Phrase("empty.nothingToPlay"))
        return
    end if

    session = SessionFor(m.global)
    play = {
        episode: episode,
        seriesId: TextOrBlank(seriesId),
        seasonId: TextOrBlank(seasonId),
        seasonNumber: seasonNumber,
        seriesTitle: TextOrBlank(seriesTitle),
        versions: invalid,
        resumable: invalid
    }
    play.versionsTask = Ask(EpisodeVersionsRequest(session.serverUrl, session.token, play.seriesId, play.seasonId, TextOrBlank(ValueAt(episode, "id", ""))), "onEpisodePlayVersions")
    play.resumeTask = Ask(ContinueRequest(session.serverUrl, session.token, session.userId), "onEpisodePlayResume")
    m.episodePlay = play
end sub

function EpisodePlayFor(event as object, field as string) as dynamic
    play = m.episodePlay
    if play = invalid then return invalid
    task = play[field]
    if task = invalid or not task.IsSameNode(event.GetRoSGNode()) then return invalid
    return play
end function

sub onEpisodePlayVersions(event as object)
    parsed = Answered(event)
    if m.released then return

    play = EpisodePlayFor(event, "versionsTask")
    if play = invalid then return

    play.versions = []
    if parsed.ok then play.versions = AvailableVersions(OrderedVersions(ListItems(parsed.json)))
    FinishEpisodePlay()
end sub

sub onEpisodePlayResume(event as object)
    parsed = Answered(event)
    if m.released then return

    play = EpisodePlayFor(event, "resumeTask")
    if play = invalid then return

    play.resumable = {}
    if parsed.ok then play.resumable = ResumeProgressMap(ContinueCards(parsed.json))
    FinishEpisodePlay()
end sub

sub FinishEpisodePlay()
    play = m.episodePlay
    if play.versions = invalid or play.resumable = invalid then return

    if play.versions.Count() = 0
        m.episodePlay = invalid
        RaiseToast("err", Phrase("empty.nothingToPlay"))
        return
    end if

    chosen = ResolvePlayTarget(play.versions, play.resumable)
    if chosen <> invalid
        PlayEpisodeVersion(chosen)
        return
    end if

    labels = []
    for each row in VersionRows(play.versions)
        labels.Push(row.label)
    end for
    m.top.choiceRequest = { field: "episodeVersion", title: Phrase("detail.chooseVersion"), kind: "choice", options: labels, selected: 0 }
end sub

function HandledEpisodeVersionChoice(result as dynamic) as boolean
    if result = invalid or TextOrBlank(ValueAt(result, "field", "")) <> "episodeVersion" then return false

    play = m.episodePlay
    if result.cancelled or play = invalid or type(play.versions) <> "roArray" or play.versions.Count() = 0
        m.episodePlay = invalid
        return true
    end if

    PlayEpisodeVersion(play.versions[ClampInt(result.index, 0, play.versions.Count() - 1)])
    return true
end function

sub PlayEpisodeVersion(version as object)
    play = m.episodePlay
    m.episodePlay = invalid

    percent = ResumePercentFor(play.resumable, ValueAt(version, "id", ""))
    m.top.advanceTarget = EpisodeVersionTarget(play.episode, play.seriesId, play.seasonId, version, play.seriesTitle, play.seasonNumber, percent)
    m.top.advance = "WatchScreen"
end sub
