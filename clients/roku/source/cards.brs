function PosterAspect() as string
    return "poster"
end function

function LandscapeAspect() as string
    return "landscape"
end function

function LandscapeFallbackUri(target as dynamic) as string
    if TextOrBlank(ValueAt(target, "aspect", "")) <> LandscapeAspect() then return ""

    return TextOrBlank(ValueAt(target, "imageUri", ""))
end function

function CaptionHeight(captionRows as integer) as integer
    sizes = TypeScale()
    space = SpacingScale()

    rows = ClampInt(captionRows, 1, 3)
    height = sizes.textBase * 1.3 + sizes.textSm * 1.3 * (rows - 1)
    return Int(height + space.s2 * 2)
end function

function CardFlagDigits(card as dynamic) as string
    digits = ""
    for each name in ["watched", "watchlisted", "favorite", "dismissible", "plain"]
        if ValueAt(card, name, false) = true
            digits = digits + "1"
        else
            digits = digits + "0"
        end if
    end for
    return digits
end function

function CardSetShape(cards as dynamic, captionRows as integer) as string
    if type(cards) <> "roArray" then return ""

    parts = [captionRows.ToStr()]
    for each card in cards
        parts.Push(TextOrBlank(ValueAt(card, "id", "")) + ":" + TextOrBlank(ValueAt(card, "aspect", "")) + ":" + TextOrBlank(ValueAt(card, "badge", "")) + ":" + Int(ValueAt(card, "progressPercent", 0)).ToStr() + ":" + CardFlagDigits(card))
    end for
    return parts.Join("|")
end function

function CardSetArt(cards as dynamic, serverUrl as string, imageWidth as integer) as string
    if type(cards) <> "roArray" then return ""

    parts = [serverUrl, imageWidth.ToStr()]
    for each card in cards
        parts.Push(CardImageUrl(serverUrl, card, imageWidth))
    end for
    return parts.Join("|")
end function

function CardSetSignature(cards as dynamic, serverUrl as string, imageWidth as integer, captionRows as integer) as string
    if type(cards) <> "roArray" then return ""

    return CardSetShape(cards, captionRows) + "||" + CardSetArt(cards, serverUrl, imageWidth)
end function

function LongestWordWidth(text as dynamic, size as integer) as integer
    widest = 0
    for each word in TextOrBlank(text).Split(" ")
        span = TextWidth(word, size, true)
        if span > widest then widest = span
    end for

    return widest
end function

function FittedNameSize(text as dynamic, width as integer) as integer
    sizes = TypeScale()
    ladder = [sizes.textBase, sizes.textSm, sizes.textXs]

    for each size in ladder
        if LongestWordWidth(text, size) <= width then return size
    end for

    return ladder[ladder.Count() - 1]
end function

function CastCardWidthFor(cardWidth as float, expanded as boolean) as integer
    if expanded then return Int(cardWidth) * 2 + SpacingScale().s4
    return Int(cardWidth)
end function

function CardMetrics(cardWidth as float, aspect as string, captionRows = 3 as integer) as object
    artHeight = cardWidth * 3 / 2
    if aspect = LandscapeAspect() then artHeight = cardWidth * 9 / 16

    textHeight = CaptionHeight(captionRows)

    return {
        width: Int(cardWidth),
        artHeight: Int(artHeight),
        textHeight: textHeight,
        height: Int(artHeight) + textHeight
    }
end function

function VisibleSlots(available as float, slotHeight as integer, spacing as integer) as integer
    if slotHeight <= 0 then return 1

    stride = slotHeight + spacing
    whole = Int((available + spacing) / stride)
    if whole * stride < available then whole = whole + 1

    return ClampInt(whole, 1, 64)
end function

function WholeSlots(available as float, slotHeight as integer, spacing as integer) as integer
    if slotHeight <= 0 then return 1

    stride = slotHeight + spacing
    whole = Int((available + spacing) / stride)
    return ClampInt(whole, 1, 64)
end function

function VisibleRailCount(rowCount as integer, available as float, rowHeight as integer, spacing as integer) as integer
    if rowCount <= 0 then return 1
    return ClampInt(VisibleSlots(available, rowHeight, spacing), 1, rowCount)
end function

function ScrollOffsetFor(index as integer, tops as object, heights as object, viewport as float) as integer
    if type(tops) <> "roArray" or index <= 0 or index >= tops.Count() then return 0

    centred = (viewport - heights[index]) / 2
    offset = tops[index] - centred
    if offset < 0 then return 0
    return Int(offset)
end function

function NewCard() as object
    return {
        kind: "",
        id: "",
        refType: "",
        refId: "",
        rollupType: "",
        rollupId: "",
        title: "",
        subtitle: "",
        caption: "",
        aspect: PosterAspect(),
        artwork: invalid,
        versionId: "",
        dismissible: false,
        plain: false,
        badge: "",
        watched: false,
        watchlisted: false,
        favorite: false,
        progressPercent: 0,
        seriesId: "",
        seasonId: ""
    }
end function

function HasArtwork(artwork as dynamic) as boolean
    if type(artwork) <> "roAssociativeArray" then return false

    for each name in ["posters", "backdrops"]
        sets = ValueAt(artwork, name, invalid)
        if type(sets) = "roArray" and sets.Count() > 0 then return true
    end for
    return false
end function

function FirstArtwork(candidates as object) as dynamic
    for each candidate in candidates
        if HasArtwork(candidate) then return candidate
    end for
    return invalid
end function

function MovieCard(json as dynamic) as object
    card = NewCard()
    if json = invalid then return card

    card.kind = "movie"
    card.id = TextOrBlank(ValueAt(json, "id", ""))
    card.refType = "movie"
    card.refId = card.id
    card.title = TextOrBlank(ValueAt(json, "title", ""))
    card.subtitle = NumberText(ValueAt(json, "year", invalid))
    card.artwork = ValueAt(json, "artwork", invalid)
    return card
end function

function SeriesCard(json as dynamic) as object
    card = NewCard()
    if json = invalid then return card

    card.kind = "series"
    card.id = TextOrBlank(ValueAt(json, "id", ""))
    card.rollupType = "series"
    card.rollupId = card.id
    card.title = TextOrBlank(ValueAt(json, "title", ""))
    card.subtitle = NumberText(ValueAt(json, "year", invalid))
    card.artwork = ValueAt(json, "artwork", invalid)
    return card
end function

function CardSpeech(card as dynamic) as string
    parts = [TextOrBlank(ValueAt(card, "title", "")), TextOrBlank(ValueAt(card, "subtitle", ""))]

    percent = Int(ValueAt(card, "progressPercent", 0))
    if ValueAt(card, "watched", false) = true
        parts.Push(Phrase("state.watched"))
    else if percent > 0
        parts.Push(PhraseWith("speech.partWatched", { percent: percent }))
    end if

    if ValueAt(card, "watchlisted", false) = true then parts.Push(Phrase("state.onWatchlist"))
    if ValueAt(card, "favorite", false) = true then parts.Push(Phrase("state.favorited"))
    parts.Push(TextOrBlank(ValueAt(card, "badge", "")))

    return SpokenLine(parts)
end function

function HubSeriesCard(json as dynamic) as object
    card = SeriesCard(json)

    count = ValueAt(json, "episode_count", invalid)
    if count <> invalid then card.subtitle = EpisodeCountText(count)
    return card
end function

function EpisodeCard(json as dynamic, asSeriesPoster = false as boolean) as object
    card = NewCard()
    if json = invalid then return card

    card.kind = "episode"
    card.id = TextOrBlank(ValueAt(json, "id", ""))
    card.refType = "episode"
    card.refId = card.id
    card.seriesId = TextOrBlank(ValueAt(json, "series_id", ""))
    card.seasonId = TextOrBlank(ValueAt(json, "season_id", ""))

    episodeTitle = TextOrBlank(ValueAt(json, "title", ""))
    seriesTitle = TextOrBlank(ValueAt(json, "series_title", ""))
    code = EpisodeCode(ValueAt(json, "season_number", invalid), ValueAt(json, "number", invalid))

    if not asSeriesPoster
        card.aspect = LandscapeAspect()
        card.title = episodeTitle
        card.subtitle = code
        if not IsBlank(seriesTitle) then card.subtitle = seriesTitle + " · " + code
        card.artwork = ValueAt(json, "artwork", invalid)
        return card
    end if

    card.subtitle = code
    if IsBlank(seriesTitle)
        card.title = episodeTitle
    else
        card.title = seriesTitle
        card.caption = episodeTitle
    end if
    card.artwork = FirstArtwork([ValueAt(json, "series_artwork", invalid), ValueAt(json, "artwork", invalid)])
    return card
end function

function SeasonCard(json as dynamic) as object
    card = NewCard()
    if json = invalid then return card

    card.kind = "season"
    card.id = TextOrBlank(ValueAt(json, "id", ""))
    card.rollupType = "season"
    card.rollupId = card.id
    card.seriesId = TextOrBlank(ValueAt(json, "series_id", ""))
    card.title = SeasonLabel(json)
    card.artwork = FirstArtwork([ValueAt(json, "artwork", invalid), ValueAt(json, "series_artwork", invalid)])
    return card
end function

function FilmographyCard(json as dynamic) as object
    card = NewCard()
    if json = invalid then return card

    card.kind = TextOrBlank(ValueAt(json, "kind", ""))
    card.id = TextOrBlank(ValueAt(json, "title_id", ""))
    card.title = TextOrBlank(ValueAt(json, "display_title", ""))
    card.subtitle = NumberText(ValueAt(json, "year", invalid))
    card.caption = TextOrBlank(ValueAt(json, "character", ""))
    card.artwork = ValueAt(json, "artwork", invalid)

    if card.kind = "series"
        card.rollupType = "series"
        card.rollupId = card.id
    else
        card.refType = card.kind
        card.refId = card.id
    end if
    return card
end function

function CastCard(actor as dynamic) as object
    card = NewCard()
    if actor = invalid then return card

    person = TextOrBlank(ValueAt(actor, "name", ""))
    character = TextOrBlank(ValueAt(actor, "character", ""))

    card.kind = "person"
    card.id = TextOrBlank(ValueAt(actor, "id", ""))
    card.title = character
    if IsBlank(card.title) then card.title = person
    card.subtitle = person
    return card
end function

function PersonCard(json as dynamic) as object
    card = NewCard()
    if json = invalid then return card

    card.kind = "person"
    card.id = TextOrBlank(ValueAt(json, "id", ""))
    card.title = TextOrBlank(ValueAt(json, "name", ""))
    card.artwork = ValueAt(json, "artwork", invalid)
    return card
end function

function CollectionCard(json as dynamic) as object
    card = NewCard()
    if json = invalid then return card

    card.kind = "collection"
    card.id = TextOrBlank(ValueAt(json, "id", ""))
    card.title = TextOrBlank(ValueAt(json, "name", ""))
    card.artwork = ValueAt(json, "artwork", invalid)
    return card
end function

function ResumeCard(json as dynamic) as object
    card = NewCard()
    if json = invalid then return card

    ref = ValueAt(json, "title", invalid)
    card.kind = TextOrBlank(ValueAt(ref, "type", ""))
    card.id = TextOrBlank(ValueAt(ref, "id", ""))
    card.refType = card.kind
    card.refId = card.id
    card.progressPercent = Int(ValueAt(json, "progress_percent", 0))

    displayTitle = TextOrBlank(ValueAt(json, "display_title", ""))
    seriesTitle = TextOrBlank(ValueAt(json, "series_title", ""))

    if card.kind <> "episode"
        card.title = displayTitle
        card.subtitle = NumberText(ValueAt(json, "year", invalid))
        card.artwork = ValueAt(json, "artwork", invalid)
        return card
    end if

    if IsBlank(seriesTitle)
        card.title = displayTitle
    else
        card.title = seriesTitle
        card.caption = displayTitle
    end if

    episodeNumber = ValueAt(json, "episode_number", invalid)
    if episodeNumber <> invalid
        card.subtitle = EpisodeCode(ValueAt(json, "season_number", invalid), episodeNumber)
    end if
    card.artwork = FirstArtwork([ValueAt(json, "series_artwork", invalid), ValueAt(json, "artwork", invalid)])
    return card
end function

function CardFromJson(json as dynamic, asSeriesPoster = false as boolean) as object
    kind = TextOrBlank(ValueAt(json, "type", ""))
    if kind = "series" then return SeriesCard(json)
    if kind = "episode" then return EpisodeCard(json, asSeriesPoster)
    if kind = "person" then return PersonCard(json)
    return MovieCard(json)
end function

function HubCard(json as dynamic) as object
    kind = TextOrBlank(ValueAt(json, "type", ""))
    if kind = "series" then return HubSeriesCard(json)
    return CardFromJson(json, true)
end function

function WatchedAtText(stamp as dynamic) as string
    iso = TextOrBlank(stamp)
    if IsBlank(iso) then return ""

    text = FormatDate(iso)
    if IsBlank(text) then return ""

    return PhraseWith("status.watchedAt", { when: text })
end function

function PlayCountBadge(count as dynamic) as string
    text = NumberText(count)
    if IsBlank(text) then return ""

    times = Int(Val(text))
    if times <= 1 then return ""

    return PhraseWith("status.timesWatched", { count: times })
end function

function LibraryCard(card as object, entry as dynamic) as object
    episodeTitle = TextOrBlank(ValueAt(card, "caption", ""))
    if not IsBlank(episodeTitle) then card.title = episodeTitle

    card.caption = WatchedAtText(ValueAt(entry, "last_watched_at", ""))
    card.badge = PlayCountBadge(ValueAt(entry, "play_count", invalid))
    card.plain = true
    return card
end function

function PosterCardFromJson(json as dynamic) as object
    card = CardFromJson(json, true)
    if card.kind <> "episode" then return card

    card.title = TextOrBlank(ValueAt(json, "title", ""))
    card.subtitle = SeriesEpisodeLabel(ValueAt(json, "series_title", invalid), card.subtitle)
    card.caption = ""
    return card
end function

function SeriesEpisodeLabel(seriesTitle as dynamic, code as string) as string
    named = TextOrBlank(seriesTitle)
    if IsBlank(named) then return code
    if IsBlank(code) then return named
    return named + ": " + code
end function

function KindOrder(kind as dynamic) as integer
    ranks = { "movie": 0, "series": 1, "episode": 2, "person": 3 }

    key = LCase(TextOrBlank(kind))
    if ranks.DoesExist(key) then return ranks[key]
    return ranks.Count()
end function

function SortedByKind(cards as dynamic) as object
    ordered = []
    if type(cards) <> "roArray" then return ordered

    for rank = 0 to KindOrder("")
        for each card in cards
            if KindOrder(ValueAt(card, "kind", "")) = rank then ordered.Push(card)
        end for
    end for
    return ordered
end function

function IndexOfCard(cards as dynamic, id as string) as integer
    if type(cards) <> "roArray" or IsBlank(id) then return -1

    for index = 0 to cards.Count() - 1
        if TextOrBlank(ValueAt(cards[index], "id", "")) = id then return index
    end for
    return -1
end function

function CardsFrom(items as dynamic, builder as function) as object
    cards = []
    if type(items) <> "roArray" then return cards

    for each item in items
        cards.Push(builder(item))
    end for
    return cards
end function

function HubRails(hubs as dynamic) as object
    rails = []
    if type(hubs) <> "roArray" then return rails

    for each hub in hubs
        id = TextOrBlank(ValueAt(hub, "id", ""))
        if not CoveredByContinueFeed(id)
            cards = CardsFrom(ValueAt(hub, "items", invalid), HubCard)
            if cards.Count() > 0
                rails.Push({ id: id, heading: HubTitle(id, TextOrBlank(ValueAt(hub, "title", ""))), cards: cards })
            end if
        end if
    end for
    return rails
end function

function CoveredByContinueFeed(id as string) as boolean
    return id = "continue_watching" or id = "on_deck"
end function

function HubTitle(id as string, fallback as string) as string
    overrides = {
        "watchlist": "home.watchlist",
        "recently_added_movies": "home.recentMovies",
        "recently_added_shows": "home.recentShows"
    }
    if overrides.DoesExist(id) then return Phrase(overrides[id])
    return fallback
end function

function ContinueCards(feed as dynamic) as object
    cards = []
    for each name in ["now_playing", "in_progress"]
        entries = ValueAt(feed, name, invalid)
        if type(entries) = "roArray"
            for each entry in entries
                card = ValueAt(entry, "card", invalid)
                if card <> invalid
                    built = ResumeCard(card)
                    built.versionId = TextOrBlank(FirstPresent([
                        ValueAt(entry, "progress.version_id", invalid),
                        ValueAt(entry, "version_id", invalid)
                    ], ""))
                    built.dismissible = not IsBlank(built.versionId)
                    cards.Push(built)
                end if
            end for
        end if
    end for
    return cards
end function

function UpNextCards(feed as dynamic) as object
    cards = []

    episodes = ValueAt(feed, "next_episodes", invalid)
    if type(episodes) = "roArray"
        for each episode in episodes
            cards.Push(EpisodeCard(episode, true))
        end for
    end if

    movies = ValueAt(feed, "next_movies", invalid)
    if type(movies) = "roArray"
        for each movie in movies
            cards.Push(MovieCard(movie))
        end for
    end if

    return cards
end function

function AspectForSet(cards as dynamic) as string
    if type(cards) <> "roArray" or cards.Count() = 0 then return PosterAspect()

    for each card in cards
        if TextOrBlank(ValueAt(card, "aspect", PosterAspect())) <> LandscapeAspect() then return PosterAspect()
    end for
    return LandscapeAspect()
end function

sub UnifyAspect(cards as dynamic)
    aspect = AspectForSet(cards)
    if type(cards) <> "roArray" then return

    for each card in cards
        card.aspect = aspect
    end for
end sub
