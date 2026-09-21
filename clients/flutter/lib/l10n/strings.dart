abstract final class Strings {
  static const String appName = 'SHADOWMASK';
  static const String skipToContent = 'Skip to content';
  static const String serverUnreachableHeading = 'Cannot reach the server';
  static const String serverUnreachableBody =
      'The server did not answer. It may be restarting, or this device may be '
      'offline.';
  static const String notFoundHeading = 'Page not found';
  static const String notFoundBody =
      'That address does not match anything here. The link may be out of date.';
  static const String appTitle = 'Shadowmask';
  static String documentTitle(String page) => '$page · $appTitle';

  static const String navigationHome = 'Home';
  static const String navigationMovies = 'Movies';
  static const String navigationSeries = 'Series';
  static const String navigationCollections = 'Collections';
  static const String navigationSearch = 'Search';
  static const String navigationAccount = 'Account';
  static const String navigationPeople = 'People';
  static const String navigationAdmin = 'Admin';
  static const String navigationMore = 'More';
  static const String signOut = 'Sign out';
  static const String appearanceHeading = 'Appearance';
  static const String accountPlaybackHeading = 'Playback';
  static const String themeDark = 'Dark';
  static const String themeLight = 'Light';
  static const String themeRetro = 'Retro';
  static const String highContrast = 'High contrast';
  static const String highContrastHelp =
      'Strengthens borders and brings secondary text up to full strength. '
      'Follows your system setting until you change it here.';

  static const String signInTitle = 'Sign in';
  static const String username = 'Username';
  static const String password = 'Password';
  static const String showPassword = 'Show password';
  static const String hidePassword = 'Hide password';
  static const String serverUnreachable = 'cannot reach the server';
  static const String usernameAndPasswordRequired =
      'Username and password are required.';
  static String signInFailed(String detail) => 'Sign in failed ($detail).';

  static const String linkDeviceTitle = 'Link this device';
  static const String linkDeviceHelp =
      'Ask someone with an account to create a link code for you, then enter '
      'it here. This device can browse and play, and no password is kept on '
      'it.';
  static const String linkCodeLabel = 'Link code';
  static const String deviceNameLabel = 'Device name';
  static const String deviceNameFallback = 'This device';
  static const String linkCodeRequired = 'Enter the link code.';
  static const String deviceNameRequired = 'Give this device a name.';
  static const String linkDeviceAction = 'Link';
  static const String linking = 'Linking…';
  static String linkFailed(String detail) => 'Linking failed ($detail).';
  static const String useLinkCode = 'Use a link code instead';
  static const String usePassword = 'Sign in with a password instead';

  static const String serverHeading = 'Server';
  static const String serverAddress = 'Server address';
  static const String serverAddressHelp =
      'The address of the media server this device connects to, for example '
      'http://192.168.1.10:8080.';
  static const String serverAddressInvalid =
      'That does not look like a server address.';
  static const String serverAddressUnanswered =
      'Nothing answered at that address.';
  static const String connect = 'Connect';
  static const String connecting = 'Connecting…';
  static const String changeServer = 'Change server';
  static const String changeServerHelp =
      'Connecting to a different server signs you out of this one.';

  static const String playbackSupportHeading = 'Playback support';
  static const String playbackSupportDeviceType = 'Device type';
  static const String playbackSupportLargestPicture = 'Largest picture';
  static const String playbackSupportVideo = 'Video';
  static const String playbackSupportAudio = 'Audio';
  static const String playbackSupportHdr = 'High dynamic range';
  static const String playbackSupportNone = 'None';
  static String playbackSupportVideoCodec(
    String codec,
    int bits, {
    required bool smooth,
  }) => '$codec · $bits-bit · ${smooth ? 'hardware' : 'software'}';
  static String playbackSupportAudioCodec(String codec, int channels) =>
      '$codec · up to $channels channels';
  static const String playbackSupportFrameRate = 'Frame rate';
  static const String playbackSupportAuto = 'Detected';
  static const String playbackSupportHardware = 'Hardware';
  static const String playbackSupportSoftware = 'Software';
  static const String playbackSupportUnsupported = 'Do not use';
  static const String playbackSupportAllow = 'Allow';
  static const String playbackSupportDeny = 'Never';
  static const String playbackSupportReset = 'Use detected values';
  static const String editPlaybackSupport = 'Edit playback support';
  static const String playbackSupportChanged = 'Overridden';
  static const String playbackSupportPictureHelp =
      'The biggest picture this device asks for. Lower it if large videos '
      'stutter: anything bigger is shrunk before it is sent, which asks more '
      'of the server but less of this device.';
  static const String playbackSupportFrameRateHelp =
      'The fastest frame rate this device asks for. Video recorded faster than '
      'this is rebuilt before it is sent.';
  static const String playbackSupportCodecHelp =
      'How this device handles video saved in this format. Hardware plays it '
      'most smoothly. Software still plays it, using more battery and '
      'sometimes stuttering. Selecting "Do not use" forces a rebuild before video is sent, '
      'which always plays but asks the most of the server.';
  static const String playbackSupportHdrHelp =
      'Whether this device is sent video with a wider range of brightness and '
      'colour. Choose "Never" if colours look washed out or too dark, and it '
      'will be converted before it is sent.';
  static String playbackSupportUpTo(int height) => 'Up to ${height}p';
  static String playbackSupportUpToRate(int rate) => 'Up to $rate fps';
  static String playbackSupportDetectedAs(String value) => 'Detected: $value';

  static const String loading = 'Loading…';
  static const String couldNotLoad = 'Could not load.';
  static const String couldNotLoadMore = 'Could not load more.';
  static const String couldNotLoadHome = 'Could not load your home screen.';
  static const String couldNotLoadMovies = 'Could not load movies.';
  static const String couldNotLoadSeries = 'Could not load series.';
  static const String couldNotLoadCollections = 'Could not load collections.';
  static const String couldNotLoadCollection =
      'Could not load this collection.';
  static const String couldNotLoadSearch = 'Could not run the search.';
  static const String couldNotLoadTitle = 'Could not load this title.';
  static const String couldNotLoadSeason = 'Could not load this season.';
  static const String couldNotLoadEpisode = 'Could not load this episode.';
  static const String back = 'Back';
  static const String couldNotLoadPerson = 'Could not load this person.';
  static const String couldNotLoadRelated = 'Could not load related titles.';
  static const String couldNotLoadVersion = 'Could not load this version.';
  static const String notAuthorized = 'Not Authorized.';
  static const String signInRequired = 'Sign-in required.';
  static const String nothingToShowYet = 'Nothing to show yet.';

  static const String noMoviesFound = 'No movies found.';
  static const String noSeriesFound = 'No series found.';
  static const String noCollectionsFound = 'No collections found.';
  static const String noResultsFound = 'No results.';
  static const String noSeasonsFound = 'No seasons.';
  static const String noEpisodesFound = 'No episodes.';
  static const String noVersionsAvailable = 'No versions available.';
  static const String noLibrariesShared =
      'No libraries have been shared with your account yet. '
      'Ask an administrator to grant you access.';

  static const String genresLabel = 'Genres';
  static const String libraryLabel = 'Library';
  static const String filterAll = 'All';
  static const String sortLabel = 'Sort by';
  static const String orderLabel = 'Order';
  static const String applyAction = 'Apply';
  static const String clearAction = 'Clear';
  static const String showMore = 'Show more';
  static const String showLess = 'Show less';
  static const String sortAdded = 'Added';
  static const String sortTitle = 'Title';
  static const String sortYear = 'Year';
  static const String orderAscending = 'Ascending';
  static const String orderDescending = 'Descending';
  static const String filtersHeading = 'Filters';
  static String orderTooltip(String value) => '$orderLabel: $value';

  static const String previous = 'Previous';
  static const String next = 'Next';
  static String pagerRange(int from, int to, int total) =>
      '$from–$to of $total';
  static String countLabel(String noun, int total) => '$noun ($total)';
  static String watchedCount(int watched, int total) =>
      '$watched of $total watched';

  static const String factYear = 'Year';
  static const String factRuntime = 'Runtime';
  static const String factRating = 'Rating';
  static const String factAdded = 'Added';
  static const String factUpdated = 'Updated';
  static const String factAirDate = 'Air date';
  static const String factBorn = 'Born';
  static const String factDied = 'Died';
  static const String factBirthplace = 'Place of birth';
  static const String factDuration = 'Duration';
  static const String factSize = 'Size';
  static const String factContainer = 'Container';
  static const String factQuality = 'Quality';
  static const String factName = 'Name';
  static const String factKind = 'Kind';
  static const String factId = 'Id';
  static const String kindMovie = 'Movie';
  static const String kindSeries = 'Series';
  static const String kindSeason = 'Season';
  static const String kindEpisode = 'Episode';
  static const String kindPerson = 'Person';
  static const String kindCollection = 'Collection';
  static const String alsoKnownAs = 'Also known as';
  static const String episodeLabel = 'Episode';
  static String episodeCode(int? season, int number) {
    final String code = 'E${number.toString().padLeft(2, '0')}';
    return season == null ? code : 'S${season.toString().padLeft(2, '0')}$code';
  }

  static String episodeTitleWithCode(int? season, int number, String title) {
    final String code = episodeCode(season, number);
    final String name = title.trim();
    return name.isEmpty ? code : '$code: $name';
  }

  static String episodeTitle(int number, String title) {
    final String numbered = '$episodeLabel $number';
    return title.trim().isEmpty || title.trim() == numbered
        ? numbered
        : '$numbered: $title';
  }

  static const String castHeading = 'Cast';
  static const String seasonsHeading = 'Seasons';
  static String seasonLabel(int number) => 'Season $number';
  static const String episodesHeading = 'Episodes';
  static const String versionsHeading = 'Versions';
  static const String filmographyHeading = 'Filmography';
  static const String moreWithPrefix = 'More with ';
  static String moreWithActor(String name) => '$moreWithPrefix$name';
  static const String roleActor = 'Actor';
  static const String roleDirector = 'Director';
  static const String roleWriter = 'Writer';
  static const String directedBy = 'Directed by';
  static const String writtenBy = 'Written by';
  static const String moreInPrefix = 'More in ';
  static String moreInCollection(String name) => '$moreInPrefix$name';
  static const String continueWatching = 'Continue watching';
  static const String upNext = 'Up next';
  static const String onYourWatchlist = 'On your watchlist';
  static const String recentlyAddedMovies = 'Recently Added Movies';
  static const String recentlyAddedShows = 'Recently Added Series';
  static String episodeCountLabel(int n) =>
      n == 1 ? '1 episode' : '$n episodes';

  static const String play = 'Play';
  static const String resumeAction = 'Resume';
  static const String chooseVersion = 'Choose a version';
  static const String nextEpisode = 'Next episode';
  static String resume(int percent) => 'Resume at $percent%';
  static const String ccNone = 'None';
  static const String fullVersionDetails = 'Full version details';
  static const String subtitlesNone = 'No subtitles';
  static const String unavailable = 'unavailable';
  static const String randomMovie = 'Play a random movie';
  static const String randomEpisode = 'Play a random episode';
  static const String randomInCollection =
      'Play a random movie from this collection';
  static const String randomInSeries = 'Play a random episode from this series';
  static const String randomInSeason = 'Play a random episode from this season';
  static const String randomNothingToPlay = 'There is nothing here to play.';
  static const String downloadVersion = 'Download the original file';
  static const String downloadNoSubtitles =
      'Downloads are the original file only. Separate subtitle files are not included.';
  static const String toastDownloadStarted = 'Download started.';
  static const String factPath = 'Path';
  static const String chaptersHeading = 'Chapters';
  static const String markersHeading = 'Markers';
  static String markerRange(String kind, String from, String to) =>
      '$kind $from to $to';
  static const String markerIntro = 'Intro';
  static const String markerCredits = 'Credits';

  static const String searchLabel = 'Search';
  static const String searchFieldLabel = 'Search the library';
  static const String searchOpening =
      'Search across movies, series, episodes and people.';
  static const String searchTypeLabel = 'Result type';
  static const String typeAll = 'All';
  static const String typeMovie = 'Movies';
  static const String typeSeries = 'Series';
  static const String typeEpisode = 'Episodes';
  static const String typePerson = 'People';

  static const String watchHeading = 'Watch';
  static const String playerPlay = 'Play';
  static const String playerPause = 'Pause';
  static const String playerSettings = 'Settings';
  static const String playerOptions = 'Player options';
  static const String playerFullscreen = 'Fullscreen';
  static const String playerWideScreen = 'Wide screen';
  static const String playerNormalScreen = 'Normal width';
  static const String playerVideo = 'Video';
  static const String playerMute = 'Mute';
  static const String playerRemainingTime = 'Count down to the end';
  static String playerSeekSeconds(int seconds) => '${seconds}s';
  static String playerHoldSpeed(double rate) =>
      '${rate == rate.roundToDouble() ? rate.toInt() : rate}x';
  static const String playerNormalSpeed = 'Normal';
  static const String playerPictureInPicture = 'Picture in picture';
  static const String playerQuality = 'Quality';
  static const String playerQualityHelp =
      'The largest picture asked for. Anything bigger is shrunk before it is '
      'sent, so choosing a smaller size can steady a weak connection at the '
      'cost of detail.';
  static const String playerAudio = 'Audio';
  static const String playerSubtitles = 'Subtitles';
  static String subtitleTrackLabel(int index) => 'Sub $index';

  static const String sourceOpenSubtitles = 'OpenSubtitles';
  static const String sourceExternal = 'External';
  static const String sourceGenerated = 'Generated';
  static const String sourceTranslated = 'Translated';
  static const String sourceCombined = 'Combined';
  static const String playerOffset = 'Offset';
  static const String playerOffsetHelp =
      'Shift the subtitles in time, in thousandths of a second. Use a positive '
      'number when they appear too early and a negative one when they lag '
      'behind the speech. A thousand is one second.';
  static const String playerBurnIn = 'Burn in';
  static const String playerBurnInHelp =
      'Draw the subtitles into the picture itself instead of sending them '
      'separately. Turn it on if subtitles do not show up at all, or if the '
      'styling of the original is lost. The cost is that they can no longer be '
      'turned off or restyled without starting over, and the video has to be '
      'rebuilt, which asks more of the server.';
  static const String playerSubtitlesImageTrack =
      'This subtitle track is an image, so it is always drawn into the picture.';
  static const String playerStereoDownmix = 'Stereo downmix';
  static const String playerStereoDownmixHelp =
      'Fold surround sound down to two channels. Turn it on for headphones or '
      'stereo speakers, where surround audio can otherwise sound thin or leave '
      'speech too quiet, because the channel carrying the dialogue is never '
      'played.';
  static const String playerSpeed = 'Speed';
  static const String playerDiagnostics = 'Diagnostics';
  static const String playerDiagnosticsHelp =
      'Show technical detail about what is playing, on top of the video. '
      'Useful when reporting a problem with playback.';
  static const String playerAutoplayNext = 'Autoplay next';
  static const String playerAutoplayNextHelp =
      'Start the next episode when one finishes, after a countdown you can '
      'cancel.';
  static const String playerAutoplayOff = 'Off';
  static String playerAutoplayDelay(int seconds) => '$seconds seconds';
  static String playerUpNextIn(int seconds) => 'Up next in ${seconds}s';
  static const String playerPlayNow = 'Play now';
  static const String shortcutsHeading = 'Shortcuts';
  static const String playerNetworkTimeout = 'Timeout';
  static const String playerNetworkTimeoutHelp =
      'How long to keep waiting for the server before giving up on a piece of '
      'video. Raise it on a slow or unreliable connection, where a short wait '
      'abandons video that would have arrived.';
  static const String playerBufferingSection = 'Buffering';
  static const String playerBufferTarget = 'Buffer';
  static const String playerBufferHelp =
      'How much video to load ahead of what you are watching. A larger amount '
      'rides out an uneven connection, but takes longer to fill and asks more '
      'of the server. Very high quality video may not reach the amount you '
      'pick, because it fills the space set aside for it sooner.';
  static const String playerBufferLimit = 'Limit';
  static const String playerBufferLimitHelp =
      'The most memory to set aside for video loaded ahead. Loading stops at '
      'whichever runs out first, this or the time above, so high quality video '
      'often stops short of the time you picked. Raise it if this device has '
      'memory to spare, lower it if other things slow down while watching.';
  static String playerBufferSize(int bytes) {
    final int mb = bytes ~/ (1024 * 1024);
    return mb >= 1024 ? '${mb ~/ 1024} GB' : '$mb MB';
  }

  static const String playerWaitForBuffer = 'Wait for buffer';
  static const String playerWaitForBufferHelp =
      'Hold playback until the amount above has loaded, instead of starting '
      'straight away. Useful on a connection that keeps pausing. Playback '
      'starts anyway if the amount stops growing, so a target this connection '
      'cannot reach will not leave you waiting for ever.';
  static String playerBufferDuration(int seconds) {
    if (seconds < 60) {
      return '$seconds seconds';
    }
    final int minutes = seconds ~/ 60;
    return minutes == 1 ? '1 minute' : '$minutes minutes';
  }

  static String playerBufferingTo(double ready, int target) =>
      'Buffering ${ready.round()}s of ${target}s';
  static const String shortcutPlayPause = 'Play or pause';
  static const String shortcutSeekBack = 'Back 10 seconds';
  static const String shortcutSeekForward = 'Forward 10 seconds';
  static const String shortcutVolumeUp = 'Volume up';
  static const String shortcutVolumeDown = 'Volume down';
  static const String shortcutMute = 'Mute or unmute';
  static const String shortcutFullscreen = 'Full screen';
  static const String shortcutWide = 'Wide screen';
  static const String shortcutPreviousEpisode = 'Previous episode';
  static const String shortcutNextEpisode = 'Next episode';
  static const String shortcutRemaining = 'Time elapsed or remaining';
  static const String shortcutLegend = 'Show this list';
  static const String shortcutBack = 'Close the panel, or leave full screen';
  static const String timelineLabel = 'Playback position';
  static const String shortcutDigits = 'Jump to 0% through 90%';
  static const String shortcutDigitKeys = '0 - 9';
  static const String playerOriginal = 'Original';
  static const String playerNoSubtitles = 'Off';
  static const String playerNoAudio = 'No audio';
  static String qualityRung(int height) => '${height}p';
  static String playerOriginalAt(int height) =>
      '$playerOriginal - ${sourceHeight(height)}';
  static String sourceHeight(int height) => switch (height) {
    >= 4320 => '8K',
    >= 2160 => '4K',
    >= 1440 => '2K',
    _ => '${height}p',
  };
  static const String playerModeLabel = 'Mode';
  static const String playerModeHelp =
      'How this video is reaching you right now. Reported, not chosen: it '
      'follows from the Converting setting and what this device can play.';
  static const String playerAdvanced = 'Advanced';
  static const String playerDelivery = 'Converting';
  static const String playerDeliveryHelp =
      'Whether video is rebuilt before it is sent. "Auto" rebuilds only when '
      'this device cannot play the original. "Never" sends the original where '
      'it can, which is fastest but may not play at all. "Always" rebuilds '
      'every time, which plays most reliably and asks the most of the server. '
      'This applies to the current video only.';
  static const String playerDeliveryAuto = 'Auto';
  static const String playerDeliveryNever = 'Never';
  static const String playerDeliveryAlways = 'Always';
  static const String playerContainerLabel = 'Segments';
  static const String playerContainerHelp =
      'How the video is packaged while it is sent to you. Reported, not '
      'chosen: it follows from the format of the original and whether it is '
      'being rebuilt.';
  static const String playerContainerNone = 'none';
  static const String playerUnmute = 'Sound is off, turn it on';
  static const String playerLoading = 'Loading';
  static const String playerBuffering = 'Buffering';
  static const String playerTooSlow =
      'This may not play at all, or may be very choppy';
  static const String playerTooSlowHint =
      'Another version of this title may play better.';
  static const String playerKeepWaiting = 'Keep waiting';
  static const String playerGoBack = 'Go back';
  static const String playerReplay = 'Watch again';
  static String playerBufferedSeconds(double seconds) =>
      '${seconds.toStringAsFixed(1)}s ready';
  static String playerBufferedPercent(double percent) =>
      '${percent.round()}% ready';
  static String titleWithYear(String title, int year) => '$title ($year)';
  static const String couldNotStartPlayback = 'Could not start playback.';
  static const String renegotiationFailed =
      'Could not switch playback options.';
  static const String noVersionSpecified = 'No version specified.';

  static const String videoHeading = 'Video';
  static const String audioHeading = 'Audio';
  static const String subtitlesHeading = 'Subtitles';
  static const String subtitleForced = 'forced';
  static const String subtitleDefault = 'default';
  static const String sdr = 'SDR';
  static const String watchedLabel = 'Watched';
  static const String watchlistLabel = 'Watchlist';
  static const String favoriteLabel = 'Favorite';

  static const String markWatched = 'Mark watched';
  static const String markUnwatched = 'Mark unwatched';
  static const String addWatchlist = 'Add to watchlist';
  static const String removeWatchlist = 'Remove from watchlist';
  static const String addFavorite = 'Add to favorites';
  static const String removeFavorite = 'Remove from favorites';
  static const String dismiss = 'Dismiss';
  static const String dismissResume = 'Remove from Continue watching';
  static const String view = 'View';
  static const String save = 'Save';
  static const String close = 'Close';
  static const String delete = 'Delete';
  static const String edit = 'Edit';
  static const String revoke = 'Revoke';
  static const String editProfile = 'Edit profile';
  static const String changePassword = 'Change password';
  static const String createLinkCode = 'Create link code';
  static const String signOutEverywhere = 'Sign out everywhere';

  static String toastAddedWatchlist(String title) =>
      'Added $title to your watchlist.';
  static String toastRemovedWatchlist(String title) =>
      'Removed $title from your watchlist.';
  static String toastAddedFavorite(String title) =>
      'Added $title to your favorites.';
  static String toastRemovedFavorite(String title) =>
      'Removed $title from your favorites.';
  static String confirmWatchedBody(int episodes) =>
      'Are you sure you want to mark $episodes episodes as watched?';
  static String confirmUnwatchedBody(int episodes) =>
      'Are you sure you want to mark $episodes episodes as not watched?';
  static const String confirmWatchedSeries =
      'Mark every episode in this series as watched?';
  static const String confirmUnwatchedSeries =
      'Mark every episode in this series as unwatched?';
  static const String confirmWatchedSeason =
      'Mark every episode in this season as watched?';
  static const String confirmUnwatchedSeason =
      'Mark every episode in this season as unwatched?';
  static String toastMarkedWatched(String title) => 'Marked $title watched.';
  static String toastMarkedUnwatched(String title) =>
      'Marked $title unwatched.';
  static String toastRemovedTitle(String title) => 'Removed $title.';
  static const String toastProfileSaved = 'Profile saved.';
  static const String toastSettingSaved = 'Setting saved.';
  static const String toastPasswordChanged = 'Password changed.';
  static const String toastCodeCreated = 'Code created.';
  static const String toastCodeRevoked = 'Code revoked.';
  static const String toastDeviceRevoked = 'Device revoked.';
  static const String toastTokenRevoked = 'Token revoked.';
  static const String toastAllSessionsRevoked = 'All sessions revoked.';
  static const String toastResumeDismissed = 'Removed from Continue watching.';

  static String errorProfileVersion(int reported, int expected) =>
      'This server speaks profile version $reported; this app expects '
      '$expected. Update whichever is older.';

  static const String errorAdd = 'Add failed.';
  static const String errorRemove = 'Remove failed.';
  static const String errorSave = 'Save failed.';
  static const String errorAction = 'Action failed.';
  static const String errorRandom = 'Could not pick something to play.';
  static const String errorRevoke = 'Revoke failed.';
  static const String errorCreate = 'Create failed.';
  static const String errorPasswordChange =
      'Change failed. Check that your current password is correct.';

  static const String reasonAccessDenied = 'You do not have access to that.';
  static const String reasonNotFound = 'It is no longer there.';
  static const String reasonScanInProgress =
      'A scan is already running for that library.';
  static const String reasonNotCancellable =
      'That job has already finished or been cancelled.';
  static const String reasonConcurrentLimit =
      'You are already watching on as many devices as your account allows.';
  static const String reasonFeatureDisabled =
      'That feature is turned off on the server.';
  static const String reasonUpstream =
      'The service it depends on did not respond.';
  static const String reasonNotEmpty =
      'Something is still filed under it. Remove that first.';
  static const String reasonUnavailable =
      'Its file is missing, so there is nothing to read.';
  static const String reasonNegotiationFailed =
      'No playable stream could be prepared for that version.';
  static const String reasonUsernameTaken = 'That username is already taken.';
  static const String reasonUnknownLinkCode =
      'That code is not valid, or it has already been used.';

  static const String requiredNewPassword = 'A new password is required.';
  static const String passwordsDoNotMatch = 'The passwords do not match.';
  static const String requiredCurrentPassword =
      'Your current password is required.';

  static const String couldNotLoadAccount = 'Could not load your account.';
  static const String emptyHistory = 'No history found.';
  static const String emptyDevices = 'No devices found.';
  static const String emptyTokens = 'No API tokens found.';
  static const String emptyLinkCodes = 'No pending codes.';
  static const String emptyWatchlist = 'Your watchlist is empty.';
  static const String emptyFavorites = 'You have no favorites yet.';

  static const String accountProfileHeading = 'Profile';
  static const String accountLinkCodesHeading = 'Link codes';
  static const String accountWatchlistHeading = 'Watchlist';
  static const String accountFavoritesHeading = 'Favorites';
  static const String accountHistoryHeading = 'History';
  static const String accountDevicesHeading = 'Devices';
  static const String accountTokensHeading = 'API tokens';
  static const String accountSessionHeading = 'Session';
  static const String accountAboutHeading = 'About';
  static const String aboutLegalese =
      'Copyright 2026 https://github.com/sndnv\nLicensed under the Apache License, Version 2.0.';
  static const String aboutBundledHelp =
      'This application bundles third-party libraries, each under its own license.';
  static const String viewThirdPartyLicenses = 'Third-party licenses';
  static const String aboutMetadataHeading = 'Metadata and subtitles';
  static const String aboutTmdbNotice =
      'This product uses TMDB and the TMDB APIs but is not endorsed, certified, or otherwise approved by TMDB.';
  static const String aboutTmdbLogoLabel = 'TMDB';
  static const String aboutSubtitleProvider =
      'Subtitles are searched and downloaded through OpenSubtitles.';
  static const String aboutOmdbNotice =
      'Ratings and certifications are supplemented by OMDb, whose data is licensed under CC BY-NC 4.0. OMDb is not endorsed by or affiliated with IMDb.com.';
  static const String accountTabProfile = 'Profile';
  static const String accountTabLibrary = 'Library';
  static const String accountTabDevices = 'Devices & Access';

  static const String fieldRole = 'Role';
  static const String fieldUsername = 'Username';
  static const String fieldPassword = 'Password';
  static const String fieldCurrentPassword = 'Current password';
  static const String fieldNewPassword = 'New password';
  static const String fieldConfirmPassword = 'Confirm new password';
  static const String fieldRepeatPassword = 'Confirm password';
  static const String fieldPreferredAudio = 'Preferred audio';
  static const String fieldPreferredSubtitle = 'Preferred subtitles';
  static const String fieldMaximumRating = 'Maximum rating';
  static const String fieldConcurrentStreams = 'Concurrent streams';
  static const String fieldBitrateCap = 'Bitrate cap';

  static const String preferredAudioHelp =
      'The audio language chosen for you when a title has more than one. '
      'Titles without it fall back to their first track.';
  static const String preferredSubtitleHelp =
      'Subtitles are turned on automatically when a title has them in this '
      'language. Leave it empty to start with subtitles off.';
  static const String maximumRatingHelp =
      'The highest content rating this account may play. Pick the rating '
      'system first, then the rating within it.';
  static const String concurrentStreamsHelp =
      'How many things this account may play at once, across every device.';
  static const String bitrateCapHelp =
      'A limit on how much bandwidth a stream may use. Useful for someone '
      'watching from a slow connection.';
  static const String fieldRatingSystem = 'Rating system';
  static const String fieldRatingCode = 'Rating';
  static const String optionUnlimited = 'Unlimited';
  static const String optionNoLimit = 'No limit';
  static const String optionNone = 'None';
  static const String optionNoPreference = 'No preference';
  static const String optionAutoDetect = 'Detect automatically';
  static const String optionAnyLanguage = 'Any language';
  static const String optionNoLanguage = 'No language';
  static String streamCount(int n) => n == 1 ? '1 stream' : '$n streams';
  static String megabitsPerSecond(int mbps) => '$mbps Mbps';
  static String expiresAt(String when) => 'Expires $when';
  static String lastSeen(String when) => 'Last seen $when';
  static String watchedAt(String when) => 'Watched $when';
  static String timesWatched(int times) => '${times}x';
  static String timesWatchedTooltip(int times) => 'Watched $times times';
  static const String removeFromHistory = 'Remove from history';
  static const String clearHistory = 'Clear history';
  static const String confirmClearHistory =
      'This removes every entry from your watch history. What you have marked '
      'as watched is not changed.';
  static const String toastHistoryRemoved = 'Removed from history.';
  static const String toastHistoryCleared = 'History cleared.';
  static const String notSet = 'Not set';

  static const String adminHeading = 'Admin';
  static const String adminGroupContent = 'Content';
  static const String adminGroupOperations = 'Operations';
  static const String adminJobs = 'Jobs';
  static const String adminLibraries = 'Libraries';
  static const String adminUsers = 'Users';
  static const String adminVersions = 'Versions';
  static String versionsWithCount(int count) => 'Versions ($count)';
  static const String versionTrickplay = 'Trickplay';
  static const String versionContainer = 'Container';
  static const String adminActivity = 'Activity';
  static const String adminFetch = 'Fetch content';
  static const String adminCollections = 'Collections';
  static const String adminLibrariesAbout = 'Manage libraries and scanning.';
  static const String adminCollectionsAbout = 'Curate movie collections.';
  static const String adminVersionsAbout = 'Manage versions and subtitles.';
  static const String adminFetchAbout = 'Fetch remote content.';
  static const String adminUsersAbout = 'Manage users and access.';
  static const String adminJobsAbout = 'Background job queue.';
  static const String adminActivityAbout = 'Recent server activity.';

  static const String relink = 'Relink';
  static const String rename = 'Rename';
  static const String viewText = 'View text';
  static const String download = 'Download';
  static const String downloaded = 'Downloaded';
  static const String downloading = 'Downloading';
  static const String scan = 'Scan';
  static const String fetch = 'Fetch';
  static const String refreshMetadata = 'Refresh metadata';
  static const String refreshUnavailable =
      'The series this episode belongs to is not known, so there is nothing to '
      'refresh from here.';
  static const String refresh = 'Refresh';
  static const String refreshThisPage = 'Refresh this page';
  static const String hiddenLevels = 'Show the levels in between';
  static const String noEarlierEpisode = 'No earlier episode';
  static const String noLaterEpisode = 'No later episode';
  static const String noEarlierSeason = 'No earlier season';
  static const String noLaterSeason = 'No later season';
  static String previousNamed(String label) => 'Previous: $label';
  static String nextNamed(String label) => 'Next: $label';
  static const String retry = 'Retry';
  static const String cancel = 'Cancel';
  static const String wipeLogs = 'Wipe logs';
  static String confirmCancelJob(String kind) =>
      'Cancel this $kind job. One that has not started never runs, and a '
      'running one is stopped where it can be. Whatever it has already '
      'written is kept.';
  static const String confirmWipeLogs =
      'Permanently remove every log line recorded for this job. The job and '
      'its status are kept, but the lines cannot be recovered.';
  static const String upscale = 'Upscale';
  static const String transcribe = 'Transcribe';
  static const String translate = 'Translate';
  static const String combineSubtitles = 'Combine subtitles';
  static const String searchSubtitles = 'Search subtitles';
  static const String deleteSubtitle = 'Delete subtitle';
  static String confirmDeleteSubtitle(String label) =>
      'Delete this subtitle ($label). The file is removed from disk, together '
      'with any translation made from it, and this cannot be undone.';
  static const String linkedTitleHeading = 'Linked title';
  static const String openTitle = 'Open title';
  static const String openJob = 'Open job';
  static const String openVersion = 'Open version';
  static const String openUser = 'Open user';
  static const String openLibrary = 'Open library';
  static const String currentResolution = 'Current resolution';
  static const String alreadyMaxResolution =
      'This version is already at the highest resolution on offer, so there '
      'is nothing to upscale to.';
  static const String height480 = '480p (SD)';
  static const String height720 = '720p (HD)';
  static const String height1080 = '1080p (Full HD)';
  static const String height1440 = '1440p (2K)';
  static const String height2160 = '2160p (4K)';
  static const String resolve = 'Resolve';
  static const String createLibrary = 'Create library';
  static const String editLibrary = 'Edit library';
  static const String editDetails = 'Edit details';
  static const String forceRefresh = 'Refresh and discard edits';
  static const String keepEdits = 'Refresh and keep edits';
  static const String refreshEditedNotice =
      'Some details have been manually edited';
  static const String refreshChoiceKeep =
      'Refresh and keep edits fetches everything else and leaves the edited '
      'fields exactly as they are.';
  static const String refreshChoiceDiscardMovie =
      'Refresh and discard edits replaces them with freshly fetched details '
      'for this movie. That cannot be undone.';
  static const String refreshChoiceDiscardSeries =
      'Refresh and discard edits replaces them for this series and every '
      'episode in it. That cannot be undone.';
  static const String confirmSaveEditHeading = 'Save these details?';
  static const String confirmSaveEditBody =
      'Saving marks this entry as manually edited, so later refreshes leave '
      'these fields alone until you choose to discard them.';
  static const String fieldImdbId = 'IMDb id';
  static const String idsAreNotEditable =
      'Identifiers come from the match and change only when you relink.';
  static const String noneRecorded = 'None recorded';
  static const String deleteLibrary = 'Delete library';
  static String confirmDeleteLibrary(String name) =>
      'Delete the library $name, along with its scan results, unmatched files '
      'and duplicate candidates. The media files on disk are not touched.';
  static const String scanLibrary = 'Scan library';
  static const String cancelJob = 'Cancel job';
  static const String createUser = 'Create user';
  static const String deleteUser = 'Delete user';
  static const String cannotDeleteSelf =
      'You cannot delete the account you are signed in with.';
  static String confirmDeleteUser(String username) =>
      'Delete the account $username, the library access granted to it, its '
      'sign-ins on every device, and the watch history and preferences stored '
      'for it. This cannot be undone.';
  static const String deactivateUser = 'Deactivate user';
  static const String activateUser = 'Activate user';
  static const String cannotDeactivateSelf =
      'You cannot deactivate the account you are signed in with.';
  static String confirmDeactivateUser(String username) =>
      'Deactivate $username. They are signed out on every device and cannot '
      'sign in again until the account is activated. Nothing they have watched '
      'or set is lost.';
  static const String deactivate = 'Deactivate';
  static const String statusActive = 'Active';
  static const String statusInactive = 'Inactive';
  static const String removeVersion = 'Remove version';
  static const String remove = 'Remove';
  static String confirmRemoveVersion(String path) =>
      'Remove $path from the catalog, along with its tracks, subtitles and '
      'trickplay. The file itself is not deleted, and if it is still there it '
      'comes back on the next scan.';

  static const String deleteMovie = 'Delete movie';
  static const String deleteSeries = 'Delete series';
  static const String deleteSeason = 'Delete season';
  static const String deleteEpisode = 'Delete episode';
  static String confirmDeleteTitle(String title) =>
      'Delete $title from the catalog, along with its artwork, cast and '
      'ratings. Nothing is removed from disk, and a later scan can bring it '
      'back if the files return.';

  static String blockedByVersions(int count) => count == 1
      ? 'Remove its 1 remaining version first.'
      : 'Remove its $count remaining versions first.';
  static String blockedByEpisodes(int count) => count == 1
      ? 'Delete its 1 remaining episode first.'
      : 'Delete its $count remaining episodes first.';
  static String blockedBySeasons(int count) => count == 1
      ? 'Delete its 1 remaining season first.'
      : 'Delete its $count remaining seasons first.';

  static const String relinkBlockedVersion =
      'The file for this version is missing, so there is nothing to read. '
      'Remove the version instead.';
  static String relinkBlockedMovie(int count) => count == 1
      ? 'One version has no file. Remove it, then relink what is left.'
      : '$count versions have no file. Remove them, then relink what is left.';
  static String relinkBlockedSeries(int count) => count == 1
      ? 'One episode has no file. Sort that episode out, then relink.'
      : '$count episodes have no file. Sort those episodes out, then relink.';
  static const String relinkBlockedEmptySeries =
      'There is nothing here to relink.';
  static const String createCollection = 'Create collection';
  static const String editCollection = 'Edit collection';
  static const String deleteCollection = 'Delete collection';
  static String confirmDeleteCollection(String name) =>
      'Delete the collection $name. The movies in it stay in the catalog and '
      'only lose their place in this collection.';
  static const String searchAction = 'Search';
  static const String viewJobs = 'View jobs';
  static const String addMembers = 'Add members';
  static const String addToCollection = 'Add to collection';
  static const String libraryAccess = 'Library access';
  static const String create = 'Create';

  static const String emptyJobs = 'No jobs.';
  static const String emptyActiveJobs = 'No active jobs.';
  static const String emptyLibraries = 'No libraries exist yet.';
  static const String emptyUsers = 'No users.';
  static const String emptyVersions = 'No versions available.';
  static const String emptyUnmatched = 'No unmatched files.';
  static const String emptyDuplicates = 'No duplicates.';
  static const String emptyCandidates = 'No candidates.';
  static const String emptyLogs = 'No logs.';
  static const String emptyActivity = 'Nothing playing.';
  static const String emptySubtitles = 'No subtitle files.';
  static const String emptySubtitleText = 'This file has no text in it.';
  static const String emptyMoviesInCollection = 'No movies in this collection.';
  static const String emptyFetches = 'No queued fetches.';
  static const String noExternalLibraries =
      'Fetching needs a library with an External origin, but none exist. '
      'Create one under Libraries first.';

  static const String couldNotLoadJobs = 'Could not load jobs.';
  static const String couldNotLoadJob = 'Could not load this job.';
  static const String couldNotLoadLibraries = 'Could not load libraries.';
  static const String couldNotLoadLibrary = 'Could not load this library.';
  static const String couldNotLoadUsers = 'Could not load users.';
  static const String couldNotLoadVersions = 'Could not load versions.';
  static const String couldNotLoadActivity = 'Could not load activity.';
  static const String couldNotLoadLogs = 'Could not load logs.';
  static const String loadingLogs = 'Loading logs…';

  static const String toastLibrarySaved = 'Library saved.';
  static const String toastMetadataSaved = 'Details saved.';
  static const String toastLibraryDeleted = 'Library deleted.';
  static const String toastCollectionSaved = 'Collection saved.';
  static const String toastCollectionCreated = 'Collection created.';
  static const String toastCollectionDeleted = 'Collection deleted.';
  static const String toastAccessSaved = 'Access saved.';
  static const String toastScanQueued = 'Scan queued.';
  static const String toastRelinkQueued = 'Relink queued.';
  static const String toastMetadataQueued = 'Metadata refresh queued.';
  static const String toastFetchQueued = 'Fetch queued. Track it under Jobs.';
  static const String toastQueuedTrackJobs = 'Queued. Track it under Jobs.';
  static const String toastUserCreated = 'User created.';
  static const String toastUserDeleted = 'User deleted.';
  static const String toastUserDeactivated = 'User deactivated.';
  static const String toastUserActivated = 'User activated.';
  static const String toastVersionRemoved = 'Version removed.';
  static const String toastJobCancelled = 'Job cancelled.';
  static const String toastLogsWiped = 'Logs wiped.';
  static const String toastRenamed = 'Renamed.';
  static const String toastDownloaded = 'Subtitle added.';
  static const String toastResolved = 'Resolved.';
  static const String toastDismissed = 'Dismissed.';
  static const String toastDeleted = 'Deleted.';
  static String toastScanQueuedFor(String name) => 'Scan queued for $name.';

  static const String errorDelete = 'Delete failed.';
  static const String errorScan = 'Scan failed.';
  static const String errorRelink = 'Relink failed.';
  static const String errorRefresh = 'Refresh failed.';
  static const String errorRename = 'Rename failed.';
  static const String errorDownload = 'Download failed.';
  static const String errorSubtitleSearch = 'Subtitle search failed.';
  static const String errorSubtitleText = 'Could not load the subtitle text.';
  static String errorScanFor(String name) => 'Scan failed for $name.';

  static const String requiredName = 'Name is required.';
  static const String invalidYear = 'Enter a four-digit year.';
  static const String invalidRuntime = 'Enter a whole number of minutes.';
  static const String invalidAirDate = 'Enter a date as YYYY-MM-DD.';
  static const String requiredUrl = 'URL is required.';
  static const String requiredLibrary = 'Pick a library.';
  static const String requiredTitle = 'Title is required.';
  static const String requiredSeason = 'Season is required for a TV library.';
  static const String requiredEpisode = 'Episode is required for a TV library.';
  static const String requiredUsername = 'Username is required.';
  static const String requiredPassword = 'Password is required.';
  static const String requiredTargetLanguage = 'Pick a language.';
  static const String requiredDistinctSubtitles =
      'Pick two different subtitles.';

  static const String statusRunning = 'Running';
  static const String statusQueued = 'Queued';
  static const String statusFailed = 'Failed';
  static const String statusSucceeded = 'Succeeded';
  static const String statusCancelled = 'Cancelled';
  static String jobsActiveCount(int n) => 'Active ($n)';
  static String jobsAllCount(int n) => 'All ($n)';
  static const String columnJob = 'Job';
  static const String columnParent = 'Parent';
  static const String jobChildren = 'Child jobs';
  static const String emptyJobChildren = 'This job has no child jobs.';
  static const String couldNotLoadJobChildren = 'Could not load child jobs.';
  static String showingNewest(int n) => 'Showing the newest $n.';
  static const String fieldFilter = 'Filter';
  static const String filterJobs = 'Filter by id, kind or status';
  static const String jobErrorHeading = 'Job error';
  static const String noMatchingJobs = 'No jobs match the filter.';
  static const String filterVersions =
      'Filter by path, title, library or quality';
  static const String noMatchingVersions = 'No versions match the filter.';

  static const String columnName = 'Name';
  static const String columnKind = 'Kind';
  static const String columnStatus = 'Status';
  static const String columnPriority = 'Priority';
  static const String columnProgress = 'Progress';
  static const String columnUsername = 'Username';
  static const String columnRole = 'Role';
  static const String columnStreams = 'Streams';
  static const String columnMovies = 'Movies';
  static const String columnPath = 'Path';
  static const String columnCodec = 'Codec';
  static const String columnResolution = 'Resolution';
  static const String columnDepth = 'Depth';
  static const String columnHdr = 'HDR';
  static const String columnFrameRate = 'FPS';
  static const String columnChannels = 'Channels';
  static const String columnLanguage = 'Language';
  static const String columnFormat = 'Format';
  static const String columnFlags = 'Flags';
  static const String columnAvailable = 'Available';
  static const String yes = 'Yes';
  static const String no = 'No';
  static const String columnLibrary = 'Library';
  static const String columnQuality = 'Quality';
  static const String columnSize = 'Size';
  static const String columnTitle = 'Title';
  static const String columnUpdated = 'Updated';
  static const String columnCreated = 'Created';
  static const String columnActions = 'Actions';
  static const String columnAttempts = 'Attempts';
  static const String columnStarted = 'Started';
  static const String columnFinished = 'Finished';
  static const String columnLastHeartbeat = 'Last heartbeat';

  static const String fieldName = 'Name';
  static const String fieldYear = 'Year';
  static const String fieldRuntimeMinutes = 'Runtime (minutes)';
  static const String fieldAirDate = 'Air date';
  static const String airDateHint = 'YYYY-MM-DD';
  static const String fieldOverview = 'Overview';
  static const String fieldKind = 'Kind';
  static const String fieldOrigin = 'Origin';
  static const String fieldRoots = 'Roots';
  static const String fieldWatcher = 'Watcher';
  static const String fieldScanSchedule = 'Scan schedule';
  static const String fieldMetadataSources = 'Metadata sources';
  static const String sourceUrlHelp =
      'The page to download from. Paste the address of the page itself, not a '
      'link to the media file.';
  static const String fetchTitleHelp =
      'What to call the result in your library. Leave it empty to keep the '
      'title the source gives it.';
  static const String fetchYearHelp =
      'Optional. The release year, which is kept in the file name and makes a '
      'later rematch far more reliable.';
  static const String fetchKindHelp =
      'Whether to file the result as a movie or as an episode. Choosing TV '
      'asks for a season and episode.';
  static const String targetLibraryHelp =
      'Which library receives the file. Only remote libraries can hold '
      'downloads, so only those are listed.';
  static const String fetchExternalIdHelp =
      'Optional. Pin the result to a specific title instead of letting the '
      'server match it by name. A plain number is a TMDB id, and one starting '
      'with tt is an IMDb id.';
  static const String seasonEpisodeHelp =
      'Where this file belongs in the series. Both are needed for a TV '
      'download, and both start at 1.';
  static const String kindHelp =
      'What this library holds. Movie keeps one entry per film. TV groups '
      'episodes under a season and a series.';
  static const String originHelp =
      'Where this library gets its files. Local means files already on a disk '
      'the server can read. Remote means files the server downloads for you. '
      'This cannot be changed later.';
  static const String watcherHelp =
      'How the server notices new files between scans. Local spots changes '
      'within seconds. Manual waits until you press Scan or a scheduled scan '
      'runs, and is the safer choice for a network share.';
  static const String rootsHelp =
      'The folders that make up this library, comma separated. Give the paths '
      'as the server sees them, not as your own computer does.';
  static const String metadataSourcesHelp =
      'Where to look up artwork and details for this library, comma separated '
      'and tried in order. Leave it empty to use the server default.';
  static const String fieldSortArticles = 'Ignored sort articles';
  static const String fieldSourceUrl = 'Source URL';
  static const String fieldTargetLibrary = 'Target library';
  static const String fieldTitle = 'Title';
  static const String fieldExternalId = 'External ID';
  static const String fieldSeason = 'Season';
  static const String fieldEpisode = 'Episode';
  static const String fieldTargetHeight = 'Target height';
  static const String fieldSourceLanguage = 'Source language';
  static const String fieldTargetLanguage = 'Target language';
  static const String fieldTopSubtitle = 'Top subtitle';
  static const String fieldBottomSubtitle = 'Bottom subtitle';
  static const String fieldLanguage = 'Language';
  static const String fieldSearchQuery = 'Query';

  static const String targetHeightHelp =
      'How tall the upscaled picture should be. Only heights above the one '
      'this version already has are offered. A new version is added and the '
      'original is kept.';
  static const String sourceLanguageHelp =
      'The language spoken in the audio. Auto-detect works out the language '
      'on its own, so set this only when you already know it.';
  static const String targetLanguageHelp =
      'The language to translate into. A new subtitle file is added and the '
      'original is kept.';
  static const String combineSubtitlesHelp =
      'Merges two subtitle files into a single track showing both languages '
      'at once, one line above the other. The two must be different.';
  static const String subtitleQueryHelp =
      'What to search for. Leave it empty to search on the title as already '
      'matched, which usually finds better results.';
  static const String subtitleLanguageHelp =
      'The language this subtitle file is labelled with, which is how the '
      'player lists it.';
  static const String fieldProviderSource = 'Provider source';
  static const String tabMove = 'Move';
  static const String tabRelink = 'Relink';
  static const String relinkSearchLabel = 'Search titles';
  static const String fieldTmdbId = 'TMDB id';
  static const String relinkProviderHint = 'movie/603 or tv/1396';
  static const String moveHere = 'Move here';
  static const String noMatches = 'No matches.';
  static const String searchFailed = 'Search failed.';
  static const String requiredSearch = 'Enter a title to search for.';
  static const String requiredTmdbId = 'Enter a TMDB id.';
  static const String confirmMoveHeading = 'Move title';
  static const String confirmRelinkHeading = 'Relink title';
  static String confirmMoveBody(String target) =>
      'Are you sure you want to move this title to $target?';
  static String confirmRelinkBody(String target) =>
      'Are you sure you want to relink this title to $target?';
  static String confirmRelinkSeries(String title) =>
      'Relink every episode of $title to the series you picked. Titles, '
      'descriptions and artwork are replaced for the series, its seasons and '
      'every episode.\n\n'
      'Episodes are placed by the season and episode numbers already read '
      'from their filenames, so nothing on disk is touched or renamed.';
  static const String relinkSeries = 'Relink series';
  static const String fieldTmdbSeriesId = 'TMDB series id';
  static const String seriesRelinkHelp =
      'The id of the series on TMDB, which is the number in its URL. Every '
      'episode is relinked in one go, so there is no need to do them one at a '
      'time.\n\n'
      'An id starting with tt is an IMDb id and works too.';
  static const String confirmRefreshHeading = 'Refresh metadata';
  static const String confirmRefreshBody =
      'Are you sure you want to refresh this title from its current match? '
      'Existing metadata and artwork are replaced.';
  static const String confirmRefreshLibraryBody =
      'Re-fetch the description, ratings and artwork for every title in this '
      'library. Existing metadata and artwork are replaced.\n\n'
      'Files are not scanned and nothing is re-processed. A large library '
      'queues one job per title.';
  static const String commaSeparatedPaths = 'Comma-separated paths';
  static const String commaSeparatedSources = 'Comma-separated sources';
  static const String commaSeparatedArticles = 'the, a, an';
  static const String sortArticlesHelp =
      'Leading words moved to the end when sorting by title. Applies to titles '
      'on the next scan. Leave empty to sort on the full title.';
  static const String whatIsThis = 'What is this?';
  static const String scheduleOff = 'Off';
  static const String scheduleHourly = 'Hourly';
  static const String scheduleSixHourly = 'Every 6 hours';
  static const String scheduleDaily = 'Daily at 03:00';
  static const String scheduleWeekly = 'Weekly on Sunday at 03:00';
  static const String scheduleCustom = 'Custom';
  static const String fieldCronExpression = 'Cron expression';
  static const String scanScheduleHelp =
      'How often the server rescans this library on its own. Off means it is '
      'only scanned when you press Scan.\n\n'
      'A custom schedule is a cron expression with seven fields, in the order '
      'second, minute, hour, day of month, month, day of week, year. That is '
      'one more field at each end than the usual five, so "0 0 3 * * * *" '
      'means every day at 03:00.';

  static const String jobFactParent = 'Parent';
  static const String jobFactError = 'Error';
  static const String jobLogsHeading = 'Logs';
  static const String scanHeading = 'Scan';
  static const String membersHeading = 'Movies';
  static const String duplicatesHeading = 'Duplicates';
  static const String unmatchedHeading = 'Unmatched';
  static const String searchResultsHeading = 'Search results';
  static const String queuedFetchesHeading = 'Queued fetches';
}
