function StringTable() as object
    if m.DoesExist("smStrings") then return m.smStrings

    m.smStrings = BuildStringTable()
    return m.smStrings
end function

function BuildStringTable() as object
    return {
        "app.name": "Shadowmask",

        "nav.home": "Home",
        "nav.movies": "Movies",
        "nav.series": "Series",
        "nav.collections": "Collections",
        "nav.search": "Search",
        "nav.account": "Account",
        "nav.people": "People",
        "action.highContrast": "High contrast",
        "action.changeServer": "Change server",
        "action.seeAll": "See all",

        "theme.dark": "Dark",
        "theme.light": "Light",
        "theme.retro": "Retro",

        "nav.watchlist": "Watchlist",
        "nav.favorites": "Favorites",
        "nav.history": "History",

        "action.ok": "OK",
        "action.cancel": "Cancel",
        "action.edit": "Edit",
        "action.play": "Play",
        "action.resume": "Resume",
        "action.back": "Back",
        "action.retry": "Try again",
        "action.signOut": "Sign out",
        "action.markWatched": "Mark watched",
        "action.markUnwatched": "Mark unwatched",
        "action.addWatchlist": "Add to watchlist",
        "action.removeWatchlist": "Remove from watchlist",
        "action.addFavorite": "Add to favorites",
        "action.removeFavorite": "Remove from favorites",
        "action.watchedLabel": "Watched",
        "action.watchlistLabel": "Watchlist",
        "action.favoriteLabel": "Favorite",
        "action.theme": "Theme",
        "action.link": "Link",
        "action.dismissResume": "Remove from Continue watching",
        "action.removeHistory": "Remove from history",
        "action.clearHistory": "Clear history",
        "action.nextEpisode": "Next",
        "action.previousEpisode": "Previous episode",
        "action.nextSeason": "Next season",
        "action.previousSeason": "Previous season",
        "action.randomInSeries": "Random",
        "action.randomInSeason": "Random",
        "action.random": "Random",
        "action.randomInCollection": "Random from this collection",
        "action.loadMore": "Load more",
        "action.more": "… more.",

        "state.loading": "Loading…",
        "state.loadingMore": "Loading more…",
        "state.yes": "Yes",
        "state.no": "No",
        "state.on": "On",
        "state.off": "Off",
        "state.watched": "Watched",
        "state.onWatchlist": "On your watchlist",
        "state.favorited": "A favorite",
        "speech.partWatched": "{percent} percent watched",
        "speech.slotOf": "{index} of {count}",
        "speech.unavailable": "Unavailable",
        "speech.selected": "Selected",
        "speech.at": "At {position} of {duration}",

        "confirm.clearHistory": "This removes every entry from your watch history. What you have marked as watched is not changed.",
        "confirm.signOut": "Sign out of this device? You will need a link code to sign back in.",
        "confirm.changeServer": "Connect this device to a different server? You will need a link code for it.",
        "help.profileReadOnly": "Preferences are changed from a phone or computer.",
        "help.autoplayNext": "Start the next episode when one finishes, after a countdown you can cancel.",
        "help.timeDisplay": "Count down how much is left instead of counting up how far in you are.",
        "help.diagnostics": "Show technical detail about what is playing, on top of the video. Useful when reporting a problem with playback.",
        "confirm.watchSeries": "Mark every episode in this series as watched?",
        "confirm.unwatchSeries": "Mark every episode in this series as unwatched?",
        "confirm.watchSeason": "Mark every episode in this season as watched?",
        "confirm.unwatchSeason": "Mark every episode in this season as unwatched?",
        "empty.noResults": "No results.",
        "empty.nothingToPlay": "There is nothing here to play.",
        "empty.nothingToShowYet": "Nothing to show yet.",
        "empty.noMovies": "No movies found.",
        "empty.noSeries": "No series found.",
        "empty.noCollections": "No collections found.",
        "empty.noWatchlist": "Your watchlist is empty.",
        "empty.noFavorites": "You have no favorites yet.",
        "empty.noHistory": "No history found.",
        "empty.noLibraryItems": "Nothing saved yet. Add a title to your watchlist or favorites, or play something.",
        "empty.noLibrariesShared": "No libraries have been shared with your account yet. Ask an administrator to grant you access.",
        "empty.noSeasons": "No seasons.",
        "empty.noEpisodes": "No episodes.",
        "empty.noOverview": "No overview.",
        "empty.noBiography": "No biography.",
        "empty.noFilmography": "Nothing in this library for this person.",
        "empty.noSubtitles": "No subtitles.",
        "empty.noTracks": "No tracks reported.",

        "error.couldNotLoad": "Could not load.",
        "error.couldNotLoadHome": "Could not load your home screen.",
        "error.couldNotLoadMovies": "Could not load movies.",
        "error.couldNotLoadSeries": "Could not load series.",
        "error.couldNotLoadCollections": "Could not load collections.",
        "error.couldNotLoadWatchlist": "Could not load your watchlist.",
        "error.couldNotLoadFavorites": "Could not load your favorites.",
        "error.couldNotLoadHistory": "Could not load your history.",
        "error.couldNotLoadSearch": "Could not run the search.",
        "error.couldNotLoadTitle": "Could not load this title.",
        "error.couldNotLoadCollection": "Could not load this collection.",
        "error.couldNotLoadSeason": "Could not load this season.",
        "error.couldNotLoadEpisode": "Could not load this episode.",
        "error.couldNotLoadPerson": "Could not load this person.",
        "error.couldNotLoadVersion": "Could not load this version.",
        "error.couldNotLoadVersions": "Could not load the versions.",
        "error.couldNotLoadMore": "Could not load more.",
        "error.couldNotPickRandom": "Nothing here could be picked to play.",
        "error.screenMissing": "{screen} is not available in this build.",
        "error.serverAddressInvalid": "That does not look like a server address.",
        "error.serverAddressUnanswered": "Nothing answered at that address.",
        "error.linkCodeRequired": "Enter the link code.",
        "error.linkFailed": "Linking failed ({detail}).",
        "error.notAuthorised": "Not Authorized.",
        "error.profileVersion": "This server speaks profile version {reported}; this app expects {expected}. Update whichever is older.",
        "error.signInRequired": "Sign-in required.",
        "error.couldNotPlay": "Could not start playback ({detail}).",
        "error.serverUnreachable": "Cannot reach the server",
        "error.serverUnreachableBody": "The server did not answer. It may be restarting, or this device may be offline.",

        "toast.ok.watchlistAdded": "Added {title} to your watchlist.",
        "toast.ok.watchlistRemoved": "Removed {title} from your watchlist.",
        "toast.ok.favoriteAdded": "Added {title} to your favorites.",
        "toast.ok.favoriteRemoved": "Removed {title} from your favorites.",
        "toast.ok.markedWatched": "Marked {title} watched.",
        "toast.ok.markedUnwatched": "Marked {title} unwatched.",
        "toast.ok.resumeDismissed": "Removed from Continue watching.",
        "toast.ok.historyRemoved": "Removed from history.",
        "toast.ok.historyCleared": "History cleared.",
        "toast.ok.updated": "Updated {title}.",
        "toast.err.actionFailed": "That did not work. Try again.",

        "form.serverAddress": "Server address",
        "form.connecting": "Connecting…",
        "form.linkCode": "Link code",
        "form.deviceName": "Device name",
        "form.linkDeviceTitle": "Link this device",
        "form.linking": "Linking…",
        "form.signingOut": "Signing out…",

        "player.loading": "Loading…",
        "player.loadingTitle": "Loading {title}…",
        "player.play": "Play",
        "player.pause": "Pause",
        "player.replay": "Watch again",
        "player.autoplayNext": "Autoplay next",
        "player.autoplayDelay": "Autoplay delay",
        "player.delaySeconds": "{seconds} seconds",
        "player.timeDisplay": "Time display",
        "player.timeElapsed": "Elapsed",
        "player.timeRemaining": "Remaining",
        "player.stalled": "The stream has stopped",
        "player.stalledBody": "Nothing has arrived for a while. Keep waiting, or go back.",
        "player.keepWaiting": "Keep waiting",
        "player.upNextIn": "Up next in {seconds}s",
        "player.settings": "Settings",
        "player.quality": "Quality",
        "player.original": "Original",
        "player.converting": "Converting",
        "player.deliveryAuto": "Auto",
        "player.deliveryNever": "Never",
        "player.deliveryAlways": "Always",
        "player.burnSubtitles": "Burn subtitles in",
        "player.downmix": "Stereo downmix",
        "player.subtitleOffset": "Subtitle offset",
        "player.offsetMs": "{ms} ms",
        "player.offsetEarlier": "Earlier",
        "player.offsetLater": "Later",
        "player.offsetReset": "Reset",
        "player.previousEpisode": "Previous episode",
        "player.nextEpisode": "Next episode",
        "player.diagnostics": "Diagnostics",
        "action.playNow": "Play now",

        "status.resumeAt": "Resume at {percent}%",
        "status.season": "Season {number}",
        "status.episodeCount": "{count} episodes",
        "status.episodeCountOne": "1 episode",
        "status.count": "{noun} ({total})",
        "status.watchedCount": "{watched} of {total} watched",
        "status.unavailable": "Unavailable",
        "status.watchedAt": "Watched {when}",
        "status.timesWatched": "{count}x",
        "status.notSet": "Not set",

        "home.continueWatching": "Continue watching",
        "home.upNext": "Up next",
        "home.watchlist": "On your watchlist",
        "home.recentMovies": "Recently Added Movies",
        "home.recentShows": "Recently Added Series",

        "list.sort": "Sort by",
        "list.order": "Order",
        "list.genres": "Genres",
        "list.library": "Library",
        "list.all": "All",
        "list.sortAdded": "Added",
        "list.sortTitle": "Title",
        "list.sortYear": "Year",
        "list.orderAsc": "Ascending",
        "list.orderDesc": "Descending",

        "fact.year": "Year",
        "fact.runtime": "Runtime",
        "fact.rating": "Rating",
        "fact.quality": "Quality",
        "fact.season": "Season",
        "fact.episode": "Episode",
        "fact.airDate": "Aired",
        "fact.watched": "Watched",
        "fact.born": "Born",
        "fact.died": "Died",
        "fact.birthplace": "From",
        "fact.username": "Username",
        "fact.role": "Role",
        "fact.preferredAudio": "Preferred audio",
        "fact.preferredSubtitle": "Preferred subtitles",
        "fact.maximumRating": "Maximum rating",
        "fact.concurrentStreams": "Concurrent streams",
        "fact.bitrateCap": "Bitrate cap",
        "fact.container": "Container",

        "heading.overview": "Overview",
        "heading.cast": "Cast",
        "heading.seasons": "Seasons",
        "heading.episodes": "Episodes",
        "heading.filmography": "In this library",
        "heading.biography": "Biography",
        "heading.video": "Video",
        "heading.audio": "Audio",
        "heading.subtitles": "Subtitles",
        "heading.inCollection": "In {name}",
        "heading.moreWith": "More with {name}",
        "heading.directedBy": "Directed by",
        "heading.library": "Library",
        "heading.profile": "Profile",
        "heading.playback": "Playback",
        "heading.playbackSupport": "Playback support",
        "heading.appearance": "Appearance",
        "heading.server": "Server",
        "heading.session": "Session",
        "heading.about": "About",

        "about.legal": "Copyright 2026 https://github.com/sndnv · Apache License 2.0",
        "about.bundled": "This channel bundles the font below, under its own license.",
        "about.tmdb": "This product uses TMDB and the TMDB APIs but is not endorsed, certified, or otherwise approved by TMDB.",
        "about.omdb": "Ratings and certifications are supplemented by OMDb, whose data is licensed under CC BY-NC 4.0. OMDb is not endorsed by or affiliated with IMDb.com.",
        "about.subtitles": "Subtitles are searched and downloaded through OpenSubtitles.",
        "action.viewLicenses": "View license",

        "caps.detected": "Detected",
        "caps.hardware": "Hardware",
        "caps.software": "Software",
        "caps.unsupported": "Do not use",
        "caps.allow": "Allow",
        "caps.deny": "Never",
        "caps.reset": "Use detected values",
        "caps.changed": "Overridden",
        "caps.picture": "Largest picture",
        "caps.frameRate": "Frame rate",
        "caps.upTo": "Up to {height}p",
        "caps.upToRate": "Up to {rate} fps",
        "caps.change": "Change",
        "heading.writtenBy": "Written by",

        "diagnostics.heading": "Playback support",
        "diagnostics.mode": "Mode",
        "diagnostics.bitrate": "Bitrate",
        "diagnostics.buffer": "Buffer",
        "diagnostics.underrun": "Underruns",
        "diagnostics.startTime": "Start time",
        "diagnostics.state": "State",
        "diagnostics.timeline": "Timeline",
        "diagnostics.fromStart": "From start",
        "diagnostics.picture": "Largest picture",
        "diagnostics.hdr": "HDR",
        "diagnostics.containers": "Containers",
        "diagnostics.profileVersion": "Profile version",
        "diagnostics.lastPlayback": "Last playback",
        "diagnostics.none": "None",
        "diagnostics.notMeasured": "Not measured",
        "diagnostics.noPlaybackYet": "Nothing played yet",

        "track.forced": "Forced",
        "track.default": "Default",
        "track.channels": "{count} channels",
        "track.bitDepth": "{bits}-bit",
        "track.sourceOpenSubtitles": "OpenSubtitles",
        "track.sourceExternal": "External",
        "track.sourceGenerated": "Generated",
        "track.sourceTranslated": "Translated",
        "track.sourceCombined": "Combined",

        "detail.versionNumber": "Version {number}",
        "detail.chooseVersion": "Choose a version",

        "watch.notYet": "Playback arrives in a later update.",
        "watch.versionReference": "Version {id}",

        "search.field": "Search the library",
        "search.opening": "Search across movies, series, episodes and people.",
        "search.keepTyping": "Keep typing to search.",

        "type.movie": "Movies",
        "type.series": "Series",
        "type.episode": "Episodes",
        "type.person": "People"
    }
end function

function EpisodeCountText(count as dynamic) as string
    if Int(count) = 1 then return Phrase("status.episodeCountOne")
    return PhraseWith("status.episodeCount", { count: count })
end function

function CountLabel(noun as string, total as dynamic) as string
    return PhraseWith("status.count", { noun: noun, total: total })
end function

function Phrase(key as string) as string
    table = StringTable()
    if table.DoesExist(key) then return table[key]
    return key
end function

function ThemeLabel(name as dynamic) as string
    key = "theme." + LCase(TextOrBlank(name))

    label = Phrase(key)
    if label = key then return TextOrBlank(name)
    return label
end function

function ThemeOptions() as object
    options = []
    for each name in ThemeNames()
        options.Push({ value: name, label: ThemeLabel(name) })
    end for
    return options
end function

function PhraseWith(key as string, values as object) as string
    text = Phrase(key)
    if values = invalid then return text

    for each name in values
        text = text.Replace("{" + name + "}", AsText(values[name]))
    end for
    return text
end function

function AsText(value as dynamic) as string
    if value = invalid then return ""
    if type(value) = "String" or type(value) = "roString" then return value
    return value.ToStr()
end function

function StringKeys() as object
    keys = []
    for each key in StringTable()
        keys.Push(key)
    end for
    return keys
end function
