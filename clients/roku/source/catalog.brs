function MaxBatchSize() as integer
    return 200
end function

function GridPageLimit() as integer
    return 100
end function

function RailPageLimit() as integer
    return 100
end function

function LoadMoreThreshold() as integer
    return 12
end function

function DefaultSort() as string
    return "added_at"
end function

function DefaultOrder() as string
    return "asc"
end function

function DefaultListQuery() as object
    return {
        sort: DefaultSort(),
        order: DefaultOrder(),
        genres: [],
        library: ""
    }
end function

function SortOptions() as object
    return [
        { value: "added_at", label: Phrase("list.sortAdded") },
        { value: "title", label: Phrase("list.sortTitle") },
        { value: "year", label: Phrase("list.sortYear") }
    ]
end function

function OrderOptions() as object
    return [
        { value: "asc", label: Phrase("list.orderAsc") },
        { value: "desc", label: Phrase("list.orderDesc") }
    ]
end function

function OrderGlyphName(order as dynamic) as string
    if AsText(order) = "desc" then return "chevron-down"
    return "chevron-up"
end function

function ValidOption(options as object, value as dynamic, fallback as string) as string
    text = AsText(value)
    for each option in options
        if option.value = text then return text
    end for
    return fallback
end function

function OptionLabel(options as object, value as dynamic) as string
    for each option in options
        if option.value = AsText(value) then return option.label
    end for
    return ""
end function

function EscapeValue(value as dynamic) as string
    text = AsText(value)
    if Len(text) = 0 then return ""
    return CreateObject("roUrlTransfer").Escape(text)
end function

function QueryString(pairs as object) as string
    if pairs = invalid then return ""

    parts = []
    for each pair in pairs
        value = AsText(pair[1])
        if Len(value.Trim()) > 0
            parts.Push(AsText(pair[0]) + "=" + EscapeValue(value))
        end if
    end for

    if parts.Count() = 0 then return ""
    return "?" + parts.Join("&")
end function

function PathWithQuery(path as string, pairs as object) as string
    return path + QueryString(pairs)
end function

function UserPath(userId as dynamic, suffix as string) as string
    return "/users/" + EscapeValue(userId) + suffix
end function

function ListQueryPairs(query as dynamic, offset as integer, limit as integer) as object
    resolved = query
    if resolved = invalid then resolved = DefaultListQuery()

    genres = ValueAt(resolved, "genres", [])
    joined = ""
    if type(genres) = "roArray" then joined = genres.Join(",")

    return [
        ["genres", joined],
        ["library", ValueAt(resolved, "library", "")],
        ["sort", ValueAt(resolved, "sort", DefaultSort())],
        ["order", ValueAt(resolved, "order", DefaultOrder())],
        ["offset", offset],
        ["limit", limit]
    ]
end function

function MoviesRequest(base as string, token as string, query as dynamic, offset as integer, limit as integer) as object
    return BuildRequest(base, PathWithQuery("/movies", ListQueryPairs(query, offset, limit)), "GET", invalid, token)
end function

function SeriesListRequest(base as string, token as string, query as dynamic, offset as integer, limit as integer) as object
    return BuildRequest(base, PathWithQuery("/series", ListQueryPairs(query, offset, limit)), "GET", invalid, token)
end function

function GenresRequest(base as string, token as string, kind as string) as object
    return BuildRequest(base, PathWithQuery("/genres", [["kind", kind]]), "GET", invalid, token)
end function

function LibrariesRequest(base as string, token as string) as object
    return BuildRequest(base, "/libraries", "GET", invalid, token)
end function

function CollectionsRequest(base as string, token as string, offset as integer, limit as integer) as object
    pairs = [["offset", offset], ["limit", limit]]
    return BuildRequest(base, PathWithQuery("/movies/collections", pairs), "GET", invalid, token)
end function

function CollectionRequest(base as string, token as string, id as dynamic) as object
    return BuildRequest(base, "/movies/collections/" + EscapeValue(id), "GET", invalid, token)
end function

function SearchRequest(base as string, token as string, term as dynamic, offset as integer, limit as integer, kind = "" as dynamic) as object
    pairs = [["q", term], ["type", TextOrBlank(kind)], ["offset", offset], ["limit", limit]]
    return BuildRequest(base, PathWithQuery("/search", pairs), "GET", invalid, token)
end function

function SearchPageLimit() as integer
    return 20
end function

function MinSearchLength() as integer
    return 2
end function

function HubRequest(base as string, token as string, userId as dynamic) as object
    return BuildRequest(base, UserPath(userId, "/hub"), "GET", invalid, token)
end function

function ContinueRequest(base as string, token as string, userId as dynamic) as object
    return BuildRequest(base, UserPath(userId, "/continue"), "GET", invalid, token)
end function

function StateBatchRequest(base as string, token as string, userId as dynamic, refs as object) as object
    return BuildRequest(base, UserPath(userId, "/state/batch"), "POST", { titles: refs }, token)
end function

function StateRollupRequest(base as string, token as string, userId as dynamic, targets as object) as object
    return BuildRequest(base, UserPath(userId, "/state/rollup"), "POST", { targets: targets }, token)
end function

function MoviePath(movieId as dynamic) as string
    return "/movies/" + EscapeValue(movieId)
end function

function SeriesPath(seriesId as dynamic) as string
    return "/series/" + EscapeValue(seriesId)
end function

function SeasonPath(seriesId as dynamic, seasonId as dynamic) as string
    return SeriesPath(seriesId) + "/seasons/" + EscapeValue(seasonId)
end function

function EpisodePath(seriesId as dynamic, seasonId as dynamic, episodeId as dynamic) as string
    return SeasonPath(seriesId, seasonId) + "/episodes/" + EscapeValue(episodeId)
end function

function VersionPageLimit() as integer
    return MaxBatchSize()
end function

function VersionPagePairs() as object
    return [["offset", 0], ["limit", VersionPageLimit()]]
end function

function MovieDetailRequest(base as string, token as string, movieId as dynamic) as object
    return BuildRequest(base, MoviePath(movieId), "GET", invalid, token)
end function

function MovieVersionsRequest(base as string, token as string, movieId as dynamic) as object
    return BuildRequest(base, PathWithQuery(MoviePath(movieId) + "/versions", VersionPagePairs()), "GET", invalid, token)
end function

function MovieCollectionsRequest(base as string, token as string, movieId as dynamic) as object
    return BuildRequest(base, MoviePath(movieId) + "/collections", "GET", invalid, token)
end function

function SeriesDetailRequest(base as string, token as string, seriesId as dynamic) as object
    return BuildRequest(base, SeriesPath(seriesId), "GET", invalid, token)
end function

function SeasonsRequest(base as string, token as string, seriesId as dynamic) as object
    return BuildRequest(base, SeriesPath(seriesId) + "/seasons", "GET", invalid, token)
end function

function SeasonRequest(base as string, token as string, seriesId as dynamic, seasonId as dynamic) as object
    return BuildRequest(base, SeasonPath(seriesId, seasonId), "GET", invalid, token)
end function

function EpisodesRequest(base as string, token as string, seriesId as dynamic, seasonId as dynamic) as object
    return BuildRequest(base, SeasonPath(seriesId, seasonId) + "/episodes", "GET", invalid, token)
end function

function EpisodeRequest(base as string, token as string, seriesId as dynamic, seasonId as dynamic, episodeId as dynamic) as object
    return BuildRequest(base, EpisodePath(seriesId, seasonId, episodeId), "GET", invalid, token)
end function

function AnyIdMark() as string
    return "-"
end function

function EpisodeByIdRequest(base as string, token as string, episodeId as dynamic) as object
    return EpisodeRequest(base, token, AnyIdMark(), AnyIdMark(), episodeId)
end function

function EpisodeVersionsRequest(base as string, token as string, seriesId as dynamic, seasonId as dynamic, episodeId as dynamic) as object
    path = PathWithQuery(EpisodePath(seriesId, seasonId, episodeId) + "/versions", VersionPagePairs())
    return BuildRequest(base, path, "GET", invalid, token)
end function

function VersionRequest(base as string, token as string, versionId as dynamic) as object
    return BuildRequest(base, "/versions/" + EscapeValue(versionId), "GET", invalid, token)
end function

function PersonRequest(base as string, token as string, personId as dynamic) as object
    return BuildRequest(base, "/people/" + EscapeValue(personId), "GET", invalid, token)
end function

function PeopleBatchRequest(base as string, token as string, ids as object) as object
    return BuildRequest(base, "/people/batch", "POST", { people: ids }, token)
end function

function RandomFilterPairs(query as dynamic) as object
    resolved = query
    if resolved = invalid then resolved = DefaultListQuery()

    genres = ValueAt(resolved, "genres", [])
    joined = ""
    if type(genres) = "roArray" then joined = genres.Join(",")

    return [
        ["genres", joined],
        ["library", ValueAt(resolved, "library", "")]
    ]
end function

function RandomMovieRequest(base as string, token as string, query as dynamic) as object
    return BuildRequest(base, PathWithQuery("/movies/random", RandomFilterPairs(query)), "GET", invalid, token)
end function

function RandomEpisodeRequest(base as string, token as string, query as dynamic) as object
    return BuildRequest(base, PathWithQuery("/series/random", RandomFilterPairs(query)), "GET", invalid, token)
end function

function RandomInCollectionRequest(base as string, token as string, collectionId as dynamic) as object
    path = "/movies/collections/" + EscapeValue(collectionId) + "/random"
    return BuildRequest(base, path, "GET", invalid, token)
end function

function RandomInSeriesRequest(base as string, token as string, seriesId as dynamic) as object
    return BuildRequest(base, SeriesPath(seriesId) + "/random", "GET", invalid, token)
end function

function RandomInSeasonRequest(base as string, token as string, seriesId as dynamic, seasonId as dynamic) as object
    return BuildRequest(base, SeasonPath(seriesId, seasonId) + "/random", "GET", invalid, token)
end function

function SetWatchedRequest(base as string, token as string, userId as dynamic, kind as dynamic, id as dynamic, watched as boolean) as object
    path = UserPath(userId, "/watched/" + EscapeValue(id))
    return BuildRequest(base, path, "PUT", { "type": TextOrBlank(kind), watched: watched }, token)
end function

function WatchlistAddRequest(base as string, token as string, userId as dynamic, kind as dynamic, id as dynamic) as object
    path = UserPath(userId, "/watchlist/" + EscapeValue(id))
    return BuildRequest(base, path, "PUT", { "type": TextOrBlank(kind) }, token)
end function

function WatchlistRemoveRequest(base as string, token as string, userId as dynamic, id as dynamic) as object
    return BuildRequest(base, UserPath(userId, "/watchlist/" + EscapeValue(id)), "DELETE", invalid, token)
end function

function FavoriteAddRequest(base as string, token as string, userId as dynamic, kind as dynamic, id as dynamic) as object
    path = UserPath(userId, "/favorites/" + EscapeValue(id))
    return BuildRequest(base, path, "PUT", { "type": TextOrBlank(kind) }, token)
end function

function FavoriteRemoveRequest(base as string, token as string, userId as dynamic, id as dynamic) as object
    return BuildRequest(base, UserPath(userId, "/favorites/" + EscapeValue(id)), "DELETE", invalid, token)
end function

function WatchlistRequest(base as string, token as string, userId as dynamic) as object
    return BuildRequest(base, UserPath(userId, "/watchlist"), "GET", invalid, token)
end function

function FavoritesRequest(base as string, token as string, userId as dynamic) as object
    return BuildRequest(base, UserPath(userId, "/favorites"), "GET", invalid, token)
end function

function HistoryRequest(base as string, token as string, userId as dynamic, offset as integer, limit as integer) as object
    path = PathWithQuery(UserPath(userId, "/history"), [["offset", offset], ["limit", limit]])
    return BuildRequest(base, path, "GET", invalid, token)
end function

function HistoryRemoveRequest(base as string, token as string, userId as dynamic, id as dynamic) as object
    return BuildRequest(base, UserPath(userId, "/history/" + EscapeValue(id)), "DELETE", invalid, token)
end function

function HistoryClearRequest(base as string, token as string, userId as dynamic) as object
    return BuildRequest(base, UserPath(userId, "/history"), "DELETE", invalid, token)
end function

function TitleCardsRequest(base as string, token as string, refs as object) as object
    return BuildRequest(base, "/titles/batch", "POST", { titles: refs }, token)
end function

function ResumeRequest(base as string, token as string, userId as dynamic, versionId as dynamic) as object
    return BuildRequest(base, UserPath(userId, "/progress/" + EscapeValue(versionId)), "GET", invalid, token)
end function

function ClearProgressRequest(base as string, token as string, userId as dynamic, versionId as dynamic) as object
    return BuildRequest(base, UserPath(userId, "/progress/" + EscapeValue(versionId)), "DELETE", invalid, token)
end function

function SelfRequest(base as string, token as string) as object
    return BuildRequest(base, "/users/self", "GET", invalid, token)
end function

function RevokeDeviceRequest(base as string, token as string, userId as dynamic, deviceId as dynamic) as object
    path = UserPath(userId, "/devices/" + EscapeValue(deviceId))
    request = BuildRequest(base, path, "DELETE", invalid, token)
    request.timeout = SignOutTimeoutSeconds()
    return request
end function

function ServerInfoRequest(base as string, token as string) as object
    return BuildRequest(base, "/server/info", "GET", invalid, token)
end function

function PersonalListDef(kind as dynamic) as object
    name = LCase(TextOrBlank(kind))

    if name = "favorites"
        return {
            kind: "favorites",
            heading: Phrase("nav.favorites"),
            emptyText: Phrase("empty.noFavorites"),
            errorText: Phrase("error.couldNotLoadFavorites"),
            paged: false,
            clearable: false
        }
    end if

    if name = "history"
        return {
            kind: "history",
            heading: Phrase("nav.history"),
            emptyText: Phrase("empty.noHistory"),
            errorText: Phrase("error.couldNotLoadHistory"),
            paged: true,
            clearable: true
        }
    end if

    return {
        kind: "watchlist",
        heading: Phrase("nav.watchlist"),
        emptyText: Phrase("empty.noWatchlist"),
        errorText: Phrase("error.couldNotLoadWatchlist"),
        paged: false,
        clearable: false
    }
end function

function PersonalListRequest(base as string, token as string, userId as dynamic, kind as dynamic, offset as integer, limit as integer) as object
    name = PersonalListDef(kind).kind

    if name = "favorites" then return FavoritesRequest(base, token, userId)
    if name = "history" then return HistoryRequest(base, token, userId, offset, limit)

    return WatchlistRequest(base, token, userId)
end function

function GenreKindFor(kind as string) as string
    if kind = "series" then return "series"
    return "movie"
end function

function NextOffset(offset as integer, count as integer) as integer
    return offset + count
end function

function HasMore(loaded as integer, total as integer) as boolean
    return loaded < total
end function

function ShouldLoadMore(focusedIndex as integer, loaded as integer, total as integer) as boolean
    if not HasMore(loaded, total) then return false
    return focusedIndex >= loaded - LoadMoreThreshold()
end function
