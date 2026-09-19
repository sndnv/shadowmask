function SortedBy(items as dynamic, precedes as function) as object
    remaining = []
    if type(items) = "roArray"
        for each item in items
            remaining.Push(item)
        end for
    end if

    sorted = []
    while remaining.Count() > 0
        pick = 0
        index = 1
        while index < remaining.Count()
            if precedes(remaining[index], remaining[pick]) then pick = index
            index = index + 1
        end while
        sorted.Push(remaining[pick])
        remaining.Delete(pick)
    end while
    return sorted
end function

function SpokenLine(parts as dynamic) as string
    kept = []
    if type(parts) <> "roArray" then return ""

    for each part in parts
        text = TextOrBlank(part)
        if not IsBlank(text) then kept.Push(text)
    end for

    if kept.Count() = 0 then return ""
    return kept.Join(", ")
end function

function SlotSpeech(label as dynamic, index as integer, count as integer, disabled = false as boolean) as string
    parts = [TextOrBlank(label), PhraseWith("speech.slotOf", { index: index + 1, count: count })]
    if disabled then parts.Push(Phrase("speech.unavailable"))

    return SpokenLine(parts)
end function

function JoinParts(parts as dynamic) as string
    kept = []
    if type(parts) <> "roArray" then return ""

    for each part in parts
        text = TextOrBlank(part)
        if not IsBlank(text) then kept.Push(text)
    end for

    if kept.Count() = 0 then return ""
    return kept.Join(" · ")
end function

function QualityRank(quality as dynamic) as integer
    ranks = { "sd": 0, "hd": 1, "fhd": 2, "uhd": 3 }
    key = LCase(TextOrBlank(quality))
    if ranks.DoesExist(key) then return ranks[key]
    return -1
end function

function QualityLabel(quality as dynamic) as string
    return UCase(TextOrBlank(quality))
end function

function VersionPrecedes(left as object, right as object) as boolean
    leftRank = QualityRank(ValueAt(left, "quality", invalid))
    rightRank = QualityRank(ValueAt(right, "quality", invalid))
    if leftRank <> rightRank then return leftRank > rightRank

    leftSize = ValueAt(left, "size_bytes", 0)
    rightSize = ValueAt(right, "size_bytes", 0)
    if leftSize <> rightSize then return leftSize > rightSize

    return TextOrBlank(ValueAt(left, "id", "")) < TextOrBlank(ValueAt(right, "id", ""))
end function

function OrderedVersions(versions as dynamic) as object
    return SortedBy(versions, VersionPrecedes)
end function

function AvailableVersions(versions as dynamic) as object
    available = []
    if type(versions) <> "roArray" then return available

    for each version in versions
        if ValueAt(version, "available", false) = true then available.Push(version)
    end for
    return available
end function

function BestVersion(ordered as dynamic) as dynamic
    if type(ordered) <> "roArray" or ordered.Count() = 0 then return invalid

    available = AvailableVersions(ordered)
    if available.Count() > 0 then return available[0]
    return ordered[0]
end function

function ResolvePlayTarget(available as dynamic, resumable as dynamic) as dynamic
    if type(available) <> "roArray" or available.Count() = 0 then return invalid
    if available.Count() = 1 then return available[0]
    if type(resumable) <> "roAssociativeArray" then return invalid

    found = invalid
    matches = 0
    for each version in available
        id = TextOrBlank(ValueAt(version, "id", ""))
        if not IsBlank(id) and resumable.DoesExist(id)
            matches = matches + 1
            found = version
        end if
    end for

    if matches = 1 then return found
    return invalid
end function

function ResumeProgressMap(cards as dynamic) as object
    resumable = {}
    if type(cards) <> "roArray" then return resumable

    for each card in cards
        versionId = TextOrBlank(ValueAt(card, "versionId", ""))
        if not IsBlank(versionId) then resumable[versionId] = Int(ValueAt(card, "progressPercent", 0))
    end for
    return resumable
end function

function ResumePercentFor(resumable as dynamic, versionId as dynamic) as integer
    id = TextOrBlank(versionId)
    if type(resumable) <> "roAssociativeArray" or IsBlank(id) then return 0
    if not resumable.DoesExist(id) then return 0
    return Int(resumable[id])
end function

function VersionRowContent(version as dynamic, index as integer) as object
    quality = QualityLabel(ValueAt(version, "quality", invalid))
    container = UCase(TextOrBlank(ValueAt(version, "container", "")))
    size = FormatMegabytes(ValueAt(version, "size_bytes", invalid))
    duration = FormatDurationText(ValueAt(version, "duration_ms", invalid))

    label = quality
    if IsBlank(label) then label = PhraseWith("detail.versionNumber", { number: index + 1 })

    return {
        id: TextOrBlank(ValueAt(version, "id", "")),
        label: label,
        detail: JoinParts([container, size, duration]),
        available: ValueAt(version, "available", false) = true
    }
end function

function VersionRows(ordered as dynamic) as object
    rows = []
    if type(ordered) <> "roArray" then return rows

    index = 0
    for each version in ordered
        rows.Push(VersionRowContent(version, index))
        index = index + 1
    end for
    return rows
end function

function CreditPrecedes(left as object, right as object) as boolean
    leftOrder = Int(ValueAt(left, "order", 0))
    rightOrder = Int(ValueAt(right, "order", 0))
    if leftOrder <> rightOrder then return leftOrder < rightOrder

    return TextOrBlank(ValueAt(left, "person.id", "")) < TextOrBlank(ValueAt(right, "person.id", ""))
end function

function CreditsWithRole(credits as dynamic, role as string) as object
    matching = []
    if type(credits) <> "roArray" then return matching

    for each credit in credits
        if TextOrBlank(ValueAt(credit, "role", "")) = role then matching.Push(credit)
    end for
    return SortedBy(matching, CreditPrecedes)
end function

function MinActorTitles() as integer
    return 3
end function

function MaxActorProbes() as integer
    return 5
end function

function EpisodeBackdropCandidates(episode as dynamic) as object
    return [ValueAt(episode, "artwork", invalid), ValueAt(episode, "series_artwork", invalid)]
end function

function SeasonBackdropCandidates(season as dynamic) as object
    return [ValueAt(season, "series_artwork", invalid)]
end function

function CastArtworkWindow() as integer
    return 10
end function

function CastArtworkLookahead() as integer
    return 4
end function

function CastArtworkWanted(loaded as integer, focused as integer, total as integer) as boolean
    if loaded >= total then return false
    return focused + CastArtworkLookahead() >= loaded
end function

function ShownTitleKeys(sets as dynamic) as object
    shown = {}
    if type(sets) <> "roArray" then return shown

    for each cards in sets
        if type(cards) = "roArray"
            for each card in cards
                shown[TitleKey(ValueAt(card, "kind", ""), ValueAt(card, "id", ""))] = true
            end for
        end if
    end for
    return shown
end function

function FreshActorTitles(cards as dynamic, shown as dynamic) as object
    fresh = []
    if type(cards) <> "roArray" then return fresh

    taken = {}
    for each card in cards
        key = TitleKey(ValueAt(card, "kind", ""), ValueAt(card, "id", ""))
        if not taken.DoesExist(key)
            taken[key] = true
            if type(shown) <> "roAssociativeArray" or not shown.DoesExist(key) then fresh.Push(card)
        end if
    end for
    return fresh
end function

function BilledActors(credits as dynamic) as object
    people = []
    seen = {}

    for each credit in CreditsWithRole(credits, "actor")
        id = TextOrBlank(ValueAt(credit, "person.id", ""))
        if not IsBlank(id) and not seen.DoesExist(id)
            seen[id] = true
            name = TextOrBlank(ValueAt(credit, "person.name", ""))
            people.Push({ id: id, name: name, character: TextOrBlank(ValueAt(credit, "character", "")) })
        end if
    end for
    return people
end function

function CreditPeople(credits as dynamic, role as string, limit as integer) as object
    people = []
    seen = {}

    for each credit in CreditsWithRole(credits, role)
        id = TextOrBlank(ValueAt(credit, "person.id", ""))
        name = TextOrBlank(ValueAt(credit, "person.name", ""))
        if not IsBlank(name) and not seen.DoesExist(id) and people.Count() < limit
            seen[id] = true
            people.Push({ id: id, name: name })
        end if
    end for
    return people
end function

function CrewNameLimit() as integer
    return 2
end function

function CrewGroups(credits as dynamic, limit as integer) as object
    groups = []
    for each spec in [{ role: "director", label: "heading.directedBy" }, { role: "writer", label: "heading.writtenBy" }]
        people = CreditPeople(credits, spec.role, limit)
        if people.Count() > 0 then groups.Push({ label: Phrase(spec.label), people: people })
    end for
    return groups
end function

function CrewPeople(groups as dynamic) as object
    people = []
    if type(groups) <> "roArray" then return people

    for each group in groups
        for each person in group.people
            people.Push(person)
        end for
    end for
    return people
end function

function CrewButtons(groups as dynamic) as object
    buttons = []
    if type(groups) <> "roArray" then return buttons

    at = 0
    for each group in groups
        if buttons.Count() > 0
            buttons.Push({ id: "crewGap:" + at.ToStr(), label: PartSeparator(), style: "link", disabled: true })
        end if
        buttons.Push({ id: "crewRole:" + at.ToStr(), label: group.label, style: "link", bold: false, disabled: true })

        last = group.people.Count() - 1
        for index = 0 to last
            person = group.people[index]
            label = person.name
            if index < last then label = label + ","
            buttons.Push({ id: "crew:" + at.ToStr(), label: label, style: "link", disabled: IsBlank(person.id) })
            at = at + 1
        end for
    end for
    return buttons
end function

function PersonIds(people as dynamic) as object
    ids = []
    if type(people) <> "roArray" then return ids

    for each person in people
        id = TextOrBlank(ValueAt(person, "id", ""))
        if not IsBlank(id) then ids.Push(id)
    end for
    return ids
end function

function ArtworkById(people as dynamic) as object
    lookup = {}
    if type(people) <> "roArray" then return lookup

    for each person in people
        id = TextOrBlank(ValueAt(person, "id", ""))
        artwork = ValueAt(person, "artwork", invalid)
        if not IsBlank(id) and artwork <> invalid then lookup[id] = artwork
    end for
    return lookup
end function

sub MergeArtwork(cards as dynamic, lookup as dynamic)
    if type(cards) <> "roArray" or type(lookup) <> "roAssociativeArray" then return

    for each card in cards
        id = TextOrBlank(ValueAt(card, "id", ""))
        if not IsBlank(id) and lookup.DoesExist(id) then card.artwork = lookup[id]
    end for
end sub

function SeasonPrecedes(left as object, right as object) as boolean
    return Int(ValueAt(left, "number", 0)) < Int(ValueAt(right, "number", 0))
end function

function EpisodePrecedes(left as object, right as object) as boolean
    return Int(ValueAt(left, "number", 0)) < Int(ValueAt(right, "number", 0))
end function

function OrderedSeasons(seasons as dynamic) as object
    return SortedBy(seasons, SeasonPrecedes)
end function

function OrderedEpisodes(episodes as dynamic) as object
    return SortedBy(episodes, EpisodePrecedes)
end function

function IndexOfId(items as dynamic, id as dynamic) as integer
    wanted = TextOrBlank(id)
    if type(items) <> "roArray" or IsBlank(wanted) then return -1

    index = 0
    for each item in items
        if TextOrBlank(ValueAt(item, "id", "")) = wanted then return index
        index = index + 1
    end for
    return -1
end function

function NeedsWatchedConfirm(kind as dynamic) as boolean
    name = LCase(TextOrBlank(kind))

    return name = "series" or name = "season"
end function

function WatchedConfirmTitle(kind as dynamic, wanted as boolean) as string
    if not NeedsWatchedConfirm(kind) then return ""

    if LCase(TextOrBlank(kind)) = "series"
        if wanted then return Phrase("confirm.watchSeries")
        return Phrase("confirm.unwatchSeries")
    end if

    if wanted then return Phrase("confirm.watchSeason")
    return Phrase("confirm.unwatchSeason")
end function

function ConfirmOptions(label as dynamic, danger = false as boolean) as object
    return [
        { label: TextOrBlank(label), danger: danger },
        { label: Phrase("action.cancel"), cancel: true }
    ]
end function

function ConfirmAccepted(result as dynamic) as boolean
    if result = invalid or ValueAt(result, "cancelled", false) = true then return false

    return Int(ValueAt(result, "index", 1)) = 0
end function

function WatchedConfirmOptions(wanted as boolean) as object
    label = Phrase("action.markUnwatched")
    if wanted then label = Phrase("action.markWatched")

    return ConfirmOptions(label)
end function

function WatchedConfirmAccepted(result as dynamic) as boolean
    return ConfirmAccepted(result)
end function

function WatchedIdSet(entries as dynamic, refField as string) as object
    ids = {}
    if type(entries) <> "roArray" then return ids

    for each entry in entries
        if ValueAt(entry, "watched", false) = true
            id = TextOrBlank(ValueAt(entry, refField + ".id", ""))
            if not IsBlank(id) then ids[id] = true
        end if
    end for
    return ids
end function

function FirstUnseen(items as dynamic, seen as dynamic) as dynamic
    if type(items) <> "roArray" or items.Count() = 0 then return invalid

    for each item in items
        id = TextOrBlank(ValueAt(item, "id", ""))
        if type(seen) <> "roAssociativeArray" or not seen.DoesExist(id) then return item
    end for
    return items[0]
end function

function NextSeason(seasons as dynamic, watchedIds as dynamic) as dynamic
    return FirstUnseen(OrderedSeasons(seasons), watchedIds)
end function

function FirstUnwatched(episodes as dynamic, watchedIds as dynamic) as dynamic
    return FirstUnseen(OrderedEpisodes(episodes), watchedIds)
end function

function SeasonLabel(season as dynamic) as string
    title = TextOrBlank(ValueAt(season, "title", ""))
    if not IsBlank(title) then return title
    return PhraseWith("status.season", { number: Int(ValueAt(season, "number", 0)) })
end function

function EpisodeLabel(seasonNumber as dynamic, episode as dynamic) as string
    code = EpisodeCode(seasonNumber, ValueAt(episode, "number", invalid))
    title = TextOrBlank(ValueAt(episode, "title", ""))
    return JoinParts([code, title])
end function

function EpisodePlayerTitle(episode as dynamic) as string
    series = TextOrBlank(ValueAt(episode, "series_title", ""))
    code = EpisodeCode(ValueAt(episode, "season_number", invalid), ValueAt(episode, "number", invalid))
    episodeName = TextOrBlank(ValueAt(episode, "title", ""))

    tail = episodeName
    if not IsBlank(code)
        tail = code
        if not IsBlank(episodeName) then tail = code + ": " + episodeName
    end if

    if IsBlank(series) then return tail
    if IsBlank(tail) then return series

    return series + " - " + tail
end function

function EpisodePlayTarget(episode as dynamic, seriesId as dynamic, seasonId as dynamic, versions as dynamic, seriesTitle = "" as dynamic, seasonNumber = invalid as dynamic) as dynamic
    if episode = invalid then return invalid

    available = AvailableVersions(OrderedVersions(ListItems(versions)))
    if available.Count() = 0 then return invalid

    chosen = ResolvePlayTarget(available, {})
    if chosen = invalid then chosen = available[0]

    full = EpisodeWithContext(episode, seriesTitle, seasonNumber)

    return {
        versionId: TextOrBlank(ValueAt(chosen, "id", "")),
        title: EpisodePlayerTitle(full),
        label: EpisodeLabel(ValueAt(full, "season_number", invalid), full),
        seriesTitle: TextOrBlank(ValueAt(full, "series_title", "")),
        seasonNumber: ValueAt(full, "season_number", invalid),
        percent: 0,
        episode: {
            id: TextOrBlank(ValueAt(episode, "id", "")),
            seriesId: TextOrBlank(seriesId),
            seasonId: TextOrBlank(seasonId)
        }
    }
end function

function SeasonNeighbours(seasonId as dynamic, seasons as dynamic) as object
    neighbours = { previous: invalid, following: invalid }

    ordered = OrderedSeasons(seasons)
    at = IndexOfId(ordered, seasonId)
    if at < 0 then return neighbours

    if at > 0 then neighbours.previous = SeasonLink(ordered[at - 1])
    if at < ordered.Count() - 1 then neighbours.following = SeasonLink(ordered[at + 1])
    return neighbours
end function

function SeasonLink(season as dynamic) as object
    return {
        id: TextOrBlank(ValueAt(season, "id", "")),
        seriesId: TextOrBlank(ValueAt(season, "series_id", "")),
        number: ValueAt(season, "number", invalid),
        label: SeasonLabel(season)
    }
end function

function PickedPlayTarget(target as dynamic, kind as dynamic, json as dynamic) as object
    picked = {}
    if type(target) = "roAssociativeArray" then picked.Append(target)
    picked.Delete("resolve")

    if IsBlank(TextOrBlank(ValueAt(json, "id", ""))) then return picked

    if TextOrBlank(kind) <> "episode"
        picked.title = TextOrBlank(ValueAt(json, "title", ""))
        picked.year = ValueAt(json, "year", invalid)
        return picked
    end if

    picked.title = EpisodePlayerTitle(json)
    picked.label = EpisodeLabel(ValueAt(json, "season_number", invalid), json)
    picked.seriesTitle = TextOrBlank(ValueAt(json, "series_title", ""))
    picked.seasonNumber = ValueAt(json, "season_number", invalid)
    picked.episode = {
        id: TextOrBlank(ValueAt(json, "id", "")),
        seriesId: TextOrBlank(ValueAt(json, "series_id", "")),
        seasonId: TextOrBlank(ValueAt(json, "season_id", ""))
    }
    return picked
end function

function EpisodeWithContext(episode as dynamic, seriesTitle as dynamic, seasonNumber as dynamic) as object
    merged = {}
    if type(episode) = "roAssociativeArray" then merged.Append(episode)

    if IsBlank(TextOrBlank(ValueAt(merged, "series_title", ""))) then merged.series_title = TextOrBlank(seriesTitle)
    if ValueAt(merged, "season_number", invalid) = invalid then merged.season_number = seasonNumber

    return merged
end function

function EpisodeLink(episode as dynamic, season as dynamic) as object
    return {
        id: TextOrBlank(ValueAt(episode, "id", "")),
        seasonId: TextOrBlank(ValueAt(episode, "season_id", "")),
        seriesId: TextOrBlank(ValueAt(season, "series_id", "")),
        label: EpisodeLabel(ValueAt(season, "number", invalid), episode)
    }
end function

function SeasonsToLoad(episode as dynamic, seasons as dynamic, currentSeason as dynamic) as object
    wanted = []

    ordered = OrderedSeasons(seasons)
    at = IndexOfId(ordered, ValueAt(episode, "season_id", ""))
    if at < 0 then return wanted

    episodes = OrderedEpisodes(currentSeason)
    index = IndexOfId(episodes, ValueAt(episode, "id", ""))
    if index < 0 then return wanted

    if index = 0 and at > 0 then wanted.Push(TextOrBlank(ValueAt(ordered[at - 1], "id", "")))
    if index = episodes.Count() - 1 and at < ordered.Count() - 1
        wanted.Push(TextOrBlank(ValueAt(ordered[at + 1], "id", "")))
    end if
    return wanted
end function

function EdgeEpisode(ordered as dynamic, at as integer, span as integer, episodesBySeason as dynamic, takeLast as boolean) as dynamic
    neighbour = at + span
    if neighbour < 0 or neighbour >= ordered.Count() then return invalid

    season = ordered[neighbour]
    theirs = OrderedEpisodes(ValueAt(episodesBySeason, TextOrBlank(ValueAt(season, "id", "")), invalid))
    if theirs.Count() = 0 then return invalid

    if takeLast then return EpisodeLink(theirs[theirs.Count() - 1], season)
    return EpisodeLink(theirs[0], season)
end function

function EpisodeNeighbours(episode as dynamic, seasons as dynamic, episodesBySeason as dynamic) as object
    neighbours = { previous: invalid, following: invalid }

    ordered = OrderedSeasons(seasons)
    seasonId = TextOrBlank(ValueAt(episode, "season_id", ""))
    at = IndexOfId(ordered, seasonId)
    if at < 0 then return neighbours

    episodes = OrderedEpisodes(ValueAt(episodesBySeason, seasonId, invalid))
    index = IndexOfId(episodes, ValueAt(episode, "id", ""))
    if index < 0 then return neighbours

    if index > 0
        neighbours.previous = EpisodeLink(episodes[index - 1], ordered[at])
    else
        neighbours.previous = EdgeEpisode(ordered, at, -1, episodesBySeason, true)
    end if

    if index < episodes.Count() - 1
        neighbours.following = EpisodeLink(episodes[index + 1], ordered[at])
    else
        neighbours.following = EdgeEpisode(ordered, at, 1, episodesBySeason, false)
    end if
    return neighbours
end function

function FactPairs(pairs as dynamic) as object
    facts = []
    if type(pairs) <> "roArray" then return facts

    for each pair in pairs
        value = TextOrBlank(pair[1])
        if not IsBlank(value) then facts.Push({ label: TextOrBlank(pair[0]), value: value })
    end for
    return facts
end function

function FactsText(facts as dynamic) as string
    values = []
    if type(facts) <> "roArray" then return ""

    for each fact in facts
        values.Push(fact.value)
    end for
    return JoinParts(values)
end function

function LabelledFactsText(facts as dynamic) as string
    parts = []
    if type(facts) <> "roArray" then return ""

    for each fact in facts
        if IsBlank(fact.label)
            parts.Push(fact.value)
        else
            parts.Push(fact.label + ": " + fact.value)
        end if
    end for
    return JoinParts(parts)
end function

function WatchedCountText(rollup as dynamic) as string
    total = Int(ValueAt(rollup, "total_episodes", 0))
    if total <= 0 then return ""

    watched = Int(ValueAt(rollup, "watched_episodes", 0))
    return PhraseWith("status.watchedCount", { watched: watched, total: total })
end function

function MovieFacts(detail as dynamic, best as dynamic) as object
    year = NumberText(ValueAt(detail, "year", invalid))
    runtime = FormatRuntimeMinutes(ValueAt(detail, "runtime_minutes", invalid))
    rating = ContentRatingLabel(ValueAt(detail, "content_rating", invalid))
    quality = QualityLabel(ValueAt(best, "quality", invalid))

    pairs = [
        [Phrase("fact.year"), year],
        [Phrase("fact.runtime"), runtime],
        [Phrase("fact.rating"), rating],
        [Phrase("fact.quality"), quality]
    ]
    return FactPairs(pairs)
end function

function SeriesFacts(detail as dynamic, rollup as dynamic) as object
    year = NumberText(ValueAt(detail, "year", invalid))
    rating = ContentRatingLabel(ValueAt(detail, "content_rating", invalid))

    pairs = [
        [Phrase("fact.year"), year],
        [Phrase("fact.rating"), rating],
        [Phrase("fact.watched"), WatchedCountText(rollup)]
    ]
    return FactPairs(pairs)
end function

function SeasonFacts(season as dynamic, rollup as dynamic) as object
    pairs = [
        [Phrase("fact.season"), NumberText(ValueAt(season, "number", invalid))],
        [Phrase("fact.watched"), WatchedCountText(rollup)]
    ]
    return FactPairs(pairs)
end function

function EpisodeFacts(episode as dynamic, best as dynamic) as object
    code = EpisodeCode(ValueAt(episode, "season_number", invalid), ValueAt(episode, "number", invalid))
    airDate = FormatDate(TextOrBlank(ValueAt(episode, "air_date", "")))
    runtime = FormatRuntimeMinutes(ValueAt(episode, "runtime_minutes", invalid))
    quality = QualityLabel(ValueAt(best, "quality", invalid))

    pairs = [
        [Phrase("fact.episode"), code],
        [Phrase("fact.airDate"), airDate],
        [Phrase("fact.runtime"), runtime],
        [Phrase("fact.quality"), quality]
    ]
    return FactPairs(pairs)
end function

function PersonFacts(person as dynamic) as object
    pairs = [
        [Phrase("fact.born"), FormatDate(TextOrBlank(ValueAt(person, "birthday", "")))],
        [Phrase("fact.died"), FormatDate(TextOrBlank(ValueAt(person, "deathday", "")))],
        [Phrase("fact.birthplace"), TextOrBlank(ValueAt(person, "place_of_birth", ""))]
    ]
    return FactPairs(pairs)
end function

function GenreChipList(genres as dynamic) as object
    chips = []
    if type(genres) <> "roArray" then return chips

    for each genre in genres
        name = TextOrBlank(ValueAt(genre, "name", ""))
        if not IsBlank(name) then chips.Push({ label: name, value: "", tone: "", border: "accent" })
    end for
    return chips
end function

function RatingTone(source as dynamic) as string
    tones = {
        "internet movie database": "warn",
        "rotten tomatoes": "danger",
        "metacritic": "ok"
    }
    key = LCase(TextOrBlank(source))
    if tones.DoesExist(key) then return tones[key]
    return ""
end function

function RatingChipList(ratings as dynamic) as object
    chips = []
    if type(ratings) <> "roArray" then return chips

    for each rating in ratings
        source = ValueAt(rating, "source", "")
        score = ScoreText(source, ValueAt(rating, "value", invalid))
        if not IsBlank(score)
            chips.Push({ label: ScoreSource(source) + ":", value: score, tone: RatingTone(source), border: "" })
        end if
    end for
    return chips
end function

function HdrLabel(hdr as dynamic) as string
    labels = {
        "hdr10": "HDR10",
        "hdr10_plus": "HDR10+",
        "dolby_vision": "Dolby Vision",
        "hlg": "HLG"
    }
    key = LCase(TextOrBlank(hdr))
    if labels.DoesExist(key) then return labels[key]
    return ""
end function

function ChannelLabel(channels as integer) as string
    if channels <= 0 then return ""

    named = { "1": "1.0", "2": "2.0", "6": "5.1", "8": "7.1" }
    key = channels.ToStr()
    if named.DoesExist(key) then return named[key]
    return PhraseWith("track.channels", { count: channels })
end function

function VideoTrackLine(track as dynamic) as string
    parts = [UCase(TextOrBlank(ValueAt(track, "codec", "")))]

    height = Int(ValueAt(track, "height", 0))
    width = Int(ValueAt(track, "width", 0))
    if height > 0 and width > 0 then parts.Push(width.ToStr() + "×" + height.ToStr())

    parts.Push(HdrLabel(ValueAt(track, "hdr", invalid)))

    depth = Int(ValueAt(track, "bit_depth", 0))
    if depth > 0 then parts.Push(PhraseWith("track.bitDepth", { bits: depth }))

    parts.Push(FormatFrameRate(ValueAt(track, "frame_rate", invalid)))
    return JoinParts(parts)
end function

function AudioTrackLine(track as dynamic) as string
    parts = [UCase(TextOrBlank(ValueAt(track, "codec", "")))]
    parts.Push(ChannelLabel(Int(ValueAt(track, "channels", 0))))
    parts.Push(UCase(TextOrBlank(ValueAt(track, "language", ""))))
    return JoinParts(parts)
end function

function SubtitleTrackLine(track as dynamic) as string
    parts = [UCase(TextOrBlank(ValueAt(track, "language", "")))]
    parts.Push(UCase(TextOrBlank(ValueAt(track, "format", ""))))

    if ValueAt(track, "forced", false) = true then parts.Push(Phrase("track.forced"))
    if ValueAt(track, "default", false) = true then parts.Push(Phrase("track.default"))
    return JoinParts(parts)
end function

function SubtitleFileLine(file as dynamic) as string
    label = TextOrBlank(ValueAt(file, "label", ""))
    if IsBlank(label) then label = UCase(TextOrBlank(ValueAt(file, "language", "")))

    parts = [label, UCase(TextOrBlank(ValueAt(file, "format", "")))]
    parts.Push(SubtitleSourceLabel(ValueAt(file, "source", invalid)))
    return JoinParts(parts)
end function

function SubtitleSourceLabel(source as dynamic) as string
    labels = {
        "open_subtitles": "track.sourceOpenSubtitles",
        "external": "track.sourceExternal",
        "generated": "track.sourceGenerated",
        "machine_translated": "track.sourceTranslated",
        "combined": "track.sourceCombined"
    }
    key = LCase(TextOrBlank(source))
    if labels.DoesExist(key) then return Phrase(labels[key])
    return ""
end function

function TrackLines(detail as dynamic, field as string, builder as function) as object
    lines = []
    tracks = ValueAt(detail, field, invalid)
    if type(tracks) <> "roArray" then return lines

    for each track in tracks
        line = builder(track)
        if not IsBlank(line) then lines.Push(line)
    end for
    return lines
end function

function LibraryMenuActions(listKind as string) as object
    if listKind = "watchlist" then return [{ id: "watchlist", label: Phrase("action.removeWatchlist"), icon: "icon-close", danger: true }]
    if listKind = "favorites" then return [{ id: "favorite", label: Phrase("action.removeFavorite"), icon: "icon-close", danger: true }]

    return [
        { id: "removeHistory", label: Phrase("action.removeHistory"), icon: "icon-close", danger: true },
        { id: "clearHistory", label: Phrase("action.clearHistory"), icon: "icon-trash", danger: true }
    ]
end function

function IsLibraryList(listKind as dynamic) as boolean
    name = LCase(TextOrBlank(listKind))

    return name = "watchlist" or name = "favorites" or name = "history"
end function

function CardMenuActions(card as dynamic, listKind = "" as dynamic) as object
    actions = []
    if card = invalid then return actions
    if IsLibraryList(listKind) then return LibraryMenuActions(LCase(TextOrBlank(listKind)))

    kind = LCase(TextOrBlank(ValueAt(card, "kind", "")))

    if kind = "collection"
        actions.Push({ id: "randomInCollection", label: Phrase("action.randomInCollection"), icon: "icon-shuffle" })
        return actions
    end if

    leaf = kind = "movie" or kind = "episode"
    rollup = kind = "series" or kind = "season"
    if not leaf and not rollup then return actions

    versionId = TextOrBlank(ValueAt(card, "versionId", ""))
    percent = Int(ValueAt(card, "progressPercent", 0))

    if leaf
        if percent > 0
            actions.Push({ id: "play", label: PhraseWith("status.resumeAt", { percent: percent }), icon: "icon-play" })
            if not IsBlank(versionId)
                actions.Push({ id: "dismissResume", label: Phrase("action.dismissResume"), icon: "icon-close", danger: true })
            end if
        else
            actions.Push({ id: "play", label: Phrase("action.play"), icon: "icon-play" })
        end if
    end if

    if ValueAt(card, "watched", false) = true
        actions.Push({ id: "watched", label: Phrase("action.markUnwatched"), icon: "icon-check" })
    else
        actions.Push({ id: "watched", label: Phrase("action.markWatched"), icon: "icon-check" })
    end if

    if leaf
        if ValueAt(card, "watchlisted", false) = true
            actions.Push({ id: "watchlist", label: Phrase("action.removeWatchlist"), icon: "icon-close", danger: true })
        else
            actions.Push({ id: "watchlist", label: Phrase("action.addWatchlist"), icon: "icon-bookmark" })
        end if

        if ValueAt(card, "favorite", false) = true
            actions.Push({ id: "favorite", label: Phrase("action.removeFavorite"), icon: "icon-close", danger: true })
        else
            actions.Push({ id: "favorite", label: Phrase("action.addFavorite"), icon: "icon-heart" })
        end if
    end if

    return actions
end function

function IsYearText(text as dynamic) as boolean
    value = TextOrBlank(text)
    if Len(value) <> 4 then return false

    for index = 1 to 4
        digit = Mid(value, index, 1)
        if digit < "0" or digit > "9" then return false
    end for
    return true
end function

function CardMenuTitle(card as dynamic) as string
    title = TextOrBlank(ValueAt(card, "title", ""))
    year = TextOrBlank(ValueAt(card, "subtitle", ""))
    if IsBlank(title) or not IsYearText(year) then return title

    return title + " (" + year + ")"
end function

function CrumbSeparator() as string
    return "›"
end function

function MenuOptions(actions as dynamic) as object
    options = []
    if type(actions) <> "roArray" then return options

    for each action in actions
        options.Push({
            label: TextOrBlank(ValueAt(action, "label", "")),
            detail: TextOrBlank(ValueAt(action, "detail", "")),
            icon: TextOrBlank(ValueAt(action, "icon", "")),
            danger: ValueAt(action, "danger", false) = true
        })
    end for
    return options
end function

function DialogRow(option as dynamic) as object
    if type(option) = "roAssociativeArray"
        return {
            label: TextOrBlank(ValueAt(option, "label", "")),
            detail: TextOrBlank(ValueAt(option, "detail", "")),
            icon: TextOrBlank(ValueAt(option, "icon", "")),
            danger: ValueAt(option, "danger", false) = true,
            cancel: ValueAt(option, "cancel", false) = true
        }
    end if

    return { label: TextOrBlank(option), detail: "", icon: "", danger: false, cancel: false }
end function

function DialogTextPlan(natural as integer, rowsNeeded as integer, room as integer, gap as integer) as object
    if room <= 0 then return { text: 0, rows: 0, scrolls: false }

    rows = rowsNeeded
    if rows > room then rows = room

    spare = room - rows
    if natural > 0 then spare = spare - gap
    if spare < 0 then spare = 0

    shown = natural
    if shown > spare then shown = spare

    return { text: shown, rows: rows, scrolls: shown < natural }
end function

function ScrolledTextOffset(offset as integer, direction as integer, shown as integer, natural as integer, pitch as integer) as integer
    span = natural - shown
    if span <= 0 then return 0

    amount = shown - pitch
    if amount < pitch then amount = pitch

    at = offset + amount * direction
    if at < 0 then at = 0
    if at > span then at = span

    return at
end function

function DialogCancelRow() as object
    return DialogRowShape({ label: Phrase("action.cancel"), cancel: true }, "confirm", -1, -1)
end function

function DialogRowShape(option as dynamic, kind as string, index as integer, selected as integer) as object
    row = DialogRow(option)
    row.chosen = kind = "choice" and index = selected and not row.cancel
    row.icon = DialogRowIcon(row, kind)
    row.reserve = not row.cancel and (kind = "choice" or not IsBlank(row.icon))
    row.chevron = not row.cancel and not IsBlank(row.detail)
    row.alignRight = row.cancel
    return row
end function

function DialogSpeech(title as dynamic, message as dynamic, rows as dynamic, index as integer) as string
    parts = [TextOrBlank(title), TextOrBlank(message)]
    if type(rows) <> "roArray" or rows.Count() = 0 then return SpokenLine(parts)

    at = ClampInt(index, 0, rows.Count() - 1)
    row = rows[at]

    parts.Push(TextOrBlank(ValueAt(row, "label", "")))
    parts.Push(TextOrBlank(ValueAt(row, "detail", "")))
    if ValueAt(row, "chosen", false) = true then parts.Push(Phrase("speech.selected"))
    parts.Push(PhraseWith("speech.slotOf", { index: at + 1, count: rows.Count() }))

    return SpokenLine(parts)
end function

function DialogCloseRequest() as object
    return {}
end function

function IsDialogClose(request as dynamic) as boolean
    if type(request) <> "roAssociativeArray" then return false

    return request.Count() = 0
end function

function DialogRows(options as dynamic, kind as dynamic, selected as integer) as object
    rows = []
    if type(options) <> "roArray" then return rows

    name = LCase(TextOrBlank(kind))
    if IsBlank(name) then name = "confirm"

    for index = 0 to options.Count() - 1
        rows.Push(DialogRowShape(options[index], name, index, selected))
    end for

    if name <> "confirm" then rows.Push(DialogCancelRow())
    return rows
end function

function DialogRowIcon(row as dynamic, kind as string) as string
    if ValueAt(row, "cancel", false) = true then return ""

    name = TextOrBlank(ValueAt(row, "icon", ""))
    if not IsBlank(name) then return name
    if kind = "choice" and ValueAt(row, "chosen", false) = true then return "icon-check"

    return ""
end function

function DialogRowPaint(theme as object, row as dynamic, focused as boolean) as object
    if focused
        if ValueAt(row, "danger", false) = true
            return { fill: theme.danger, label: theme.dangerContrast, glyph: theme.dangerContrast, detail: theme.dangerContrast }
        end if
        return { fill: theme.accent, label: theme.accentContrast, glyph: theme.accentContrast, detail: theme.accentContrast }
    end if

    if ValueAt(row, "danger", false) = true
        return { fill: "", label: theme.danger, glyph: theme.danger, detail: theme.muted }
    end if
    if ValueAt(row, "cancel", false) = true
        return { fill: "", label: theme.muted, glyph: theme.muted, detail: theme.muted }
    end if
    if ValueAt(row, "chosen", false) = true
        return { fill: "", label: theme.text, glyph: theme.accent, detail: theme.muted }
    end if

    return { fill: "", label: theme.text, glyph: theme.muted, detail: theme.muted }
end function

function DialogChoiceResult(field as dynamic, index as integer, count as integer) as object
    if index < 0 or index >= count then return { field: TextOrBlank(field), index: 0, cancelled: true }

    return { field: TextOrBlank(field), index: index, cancelled: false }
end function

function SteppedRowIndex(rows as dynamic, from as integer, direction as integer) as integer
    if type(rows) <> "roArray" or rows.Count() = 0 then return 0

    count = rows.Count()
    at = from + direction
    if at < 0 then return count - 1
    if at >= count then return 0

    return at
end function

function PartSeparator() as string
    return "·"
end function

function CrumbEntries(crumbs as dynamic) as object
    entries = [{ label: Phrase("nav.home"), screen: "HomeScreen", target: invalid }]
    if type(crumbs) <> "roArray" then return entries

    for each crumb in crumbs
        if not IsBlank(TextOrBlank(ValueAt(crumb, "label", ""))) then entries.Push(crumb)
    end for
    return entries
end function

function CrumbTrailSpeech(entries as dynamic) as string
    if type(entries) <> "roArray" or entries.Count() = 0 then return ""

    labels = []
    for each entry in entries
        labels.Push(TextOrBlank(ValueAt(entry, "label", "")))
    end for
    return SpokenLine(labels)
end function

function CrumbButtons(entries as dynamic) as object
    buttons = []
    if type(entries) <> "roArray" or entries.Count() = 0 then return buttons

    last = entries.Count() - 1
    for index = 0 to last
        if index > 0
            buttons.Push({ id: "crumbGap:" + index.ToStr(), label: CrumbSeparator(), style: "link", disabled: true })
        end if

        linked = index < last and not IsBlank(TextOrBlank(ValueAt(entries[index], "screen", "")))
        buttons.Push({
            id: "crumb:" + index.ToStr(),
            label: TextOrBlank(ValueAt(entries[index], "label", "")),
            style: "link",
            disabled: not linked
        })
    end for
    return buttons
end function

function ScrollThumb(content as integer, viewport as integer, offset as integer, minimum as integer, track = 0 as integer) as object
    if track <= 0 then track = viewport
    if viewport <= 0 or content <= viewport then return { top: 0, height: track }

    height = Int(track * viewport / content)
    if height < minimum then height = minimum
    if height > track then height = track

    travel = track - height
    scrolled = content - viewport

    at = Int(travel * ClampInt(offset, 0, scrolled) / scrolled)
    return { top: ClampInt(at, 0, travel), height: height }
end function

function LineBudgets(available as integer, count as integer, lastInset as integer) as object
    budgets = []
    if count <= 0 then return budgets

    for index = 1 to count
        if index = count
            budgets.Push(available - lastInset)
        else
            budgets.Push(available)
        end if
    end for
    return budgets
end function

function FittedLines(widths as dynamic, gapWidth as integer, budgets as dynamic) as object
    lines = []
    counts = []
    if type(widths) <> "roArray" or type(budgets) <> "roArray" or budgets.Count() = 0
        return { lines: lines, counts: counts, taken: 0, width: 0 }
    end if

    taken = 0
    used = 0
    count = 0
    at = 0
    budget = Int(budgets[0])

    for each entry in widths
        want = Int(entry)
        needed = want
        if used > 0 then needed = needed + gapWidth

        if used > 0 and used + needed > budget
            lines.Push(used)
            counts.Push(count)

            at = at + 1
            if at >= budgets.Count() then return { lines: lines, counts: counts, taken: taken, width: used }

            budget = Int(budgets[at])
            used = want
            count = 1
        else
            used = used + needed
            count = count + 1
        end if
        taken = taken + 1
    end for

    lines.Push(used)
    counts.Push(count)
    return { lines: lines, counts: counts, taken: taken, width: used }
end function

function LineWords(words as dynamic, counts as dynamic, index as integer) as object
    slice = []
    if type(words) <> "roArray" or type(counts) <> "roArray" then return slice
    if index < 0 or index >= counts.Count() then return slice

    start = 0
    for at = 0 to index - 1
        start = start + Int(counts[at])
    end for

    for at = start to start + Int(counts[index]) - 1
        if at >= 0 and at < words.Count() then slice.Push(words[at])
    end for
    return slice
end function

function WrappedLineCount(widths as dynamic, gapWidth as integer, available as integer) as integer
    if type(widths) <> "roArray" or widths.Count() = 0 or available <= 0 then return 1

    fitted = FittedLines(widths, gapWidth, LineBudgets(available, widths.Count(), 0))
    if fitted.lines.Count() < 1 then return 1
    return fitted.lines.Count()
end function

function FlatWords(text as dynamic) as object
    words = []
    value = TextOrBlank(text)
    if IsBlank(value) then return words

    for each chunk in value.Split(Chr(10))
        for each word in chunk.Split(" ")
            if not IsBlank(word) then words.Push(word)
        end for
    end for
    return words
end function

function TrailingMarks() as string
    return ".,;:!?"
end function

function WithoutTrailingMark(text as dynamic) as string
    value = TextOrBlank(text)
    marks = TrailingMarks()

    trimmed = value
    while Len(trimmed) > 0
        if Instr(1, marks, Right(trimmed, 1)) = 0 then exit while
        trimmed = Left(trimmed, Len(trimmed) - 1)
    end while

    if IsBlank(trimmed) then return value
    return trimmed
end function

function JoinWords(words as dynamic, count as integer) as string
    if type(words) <> "roArray" or count <= 0 then return ""

    kept = []
    for index = 0 to ClampInt(count, 0, words.Count()) - 1
        kept.Push(words[index])
    end for
    return kept.Join(" ")
end function

function WordWidths(paragraph as string, size as integer) as object
    widths = []
    for each word in paragraph.Split(" ")
        if not IsBlank(word) then widths.Push(TextWidth(word, size))
    end for
    return widths
end function

function MeasuredWords(text as dynamic, size as integer) as object
    value = TextOrBlank(text)
    key = size.ToStr() + ":" + value

    if m.DoesExist("smWordMeasure") and m.smWordMeasure.key = key then return m.smWordMeasure

    words = FlatWords(value)
    widths = []
    for each word in words
        widths.Push(TextWidth(word, size))
    end for

    m.smWordMeasure = { key: key, words: words, widths: widths }
    return m.smWordMeasure
end function

function TextLineCount(text as dynamic, width as integer, size as integer) as integer
    value = TextOrBlank(text)
    if IsBlank(value) or width <= 0 then return 1

    key = width.ToStr() + ":" + size.ToStr() + ":" + value
    if m.DoesExist("smLineCount") and m.smLineCount.key = key then return m.smLineCount.lines

    gap = SpaceWidth(size)
    total = 0
    for each paragraph in value.Split(Chr(10))
        total = total + WrappedLineCount(WordWidths(paragraph, size), gap, width)
    end for

    if total < 1 then total = 1

    m.smLineCount = { key: key, lines: total }
    return total
end function

function GroupNaturalWidth(count as integer, cardWidth as integer, gap as integer) as integer
    if count <= 0 or cardWidth <= 0 then return 0
    return count * cardWidth + (count - 1) * gap
end function

function PackedRows(widths as dynamic, available as integer, gap as integer) as object
    rows = []
    if type(widths) <> "roArray" or widths.Count() = 0 or available <= 0 then return rows

    current = []
    used = 0

    for index = 0 to widths.Count() - 1
        want = Int(widths[index])
        if want > available then want = available

        needed = want
        if current.Count() > 0 then needed = needed + gap

        if used + needed > available and current.Count() > 0
            rows.Push(current)
            current = []
            used = 0
            needed = want
        end if

        current.Push({ index: index, width: want })
        used = used + needed
    end for

    if current.Count() > 0 then rows.Push(current)
    return rows
end function

function RevealOffset(index as integer, tops as dynamic, heights as dynamic, viewport as float, current as integer, pad = -1 as integer) as integer
    if type(tops) <> "roArray" or index < 0 or index >= tops.Count() then return current

    if pad < 0 then pad = ContentBottomPad()

    total = tops[tops.Count() - 1] + heights[heights.Count() - 1] + pad
    limit = total - viewport
    if limit < 0 then limit = 0

    offset = current
    top = tops[index]
    bottom = top + heights[index] + pad

    if top < offset then offset = top
    if bottom - offset > viewport then offset = bottom - viewport
    if offset > limit then offset = limit
    if offset < 0 then offset = 0

    return Int(offset)
end function
