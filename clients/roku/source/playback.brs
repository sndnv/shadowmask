function SessionPath(sessionId as dynamic) as string
    return "/sessions/" + EscapeValue(sessionId)
end function

function StartSessionRequest(base as string, token as string, versionId as string, positionMs as integer, capabilities as object, controls = invalid as dynamic) as object
    body = {
        version_id: versionId,
        capabilities: capabilities
    }
    if positionMs > 0 then body.start_position_ms = positionMs

    chosen = ControlsStartBody(controls)
    for each key in chosen
        body[key] = chosen[key]
    end for
    return BuildRequest(base, "/sessions", "POST", body, token)
end function

function HeartbeatRequest(base as string, token as string, sessionId as dynamic, positionMs as integer, state as string) as object
    body = { position_ms: positionMs, state: state }
    return BuildRequest(base, SessionPath(sessionId) + "/progress", "POST", body, token)
end function

function SeekSessionRequest(base as string, token as string, sessionId as dynamic, positionMs as integer) as object
    return BuildRequest(base, SessionPath(sessionId) + "/seek", "POST", { position_ms: positionMs }, token)
end function

function UpdateSessionRequest(base as string, token as string, sessionId as dynamic, controls as dynamic) as object
    return BuildRequest(base, SessionPath(sessionId) + "/update", "POST", ControlsUpdateBody(controls), token)
end function

function EndSessionRequest(base as string, token as string, sessionId as dynamic) as object
    return BuildRequest(base, SessionPath(sessionId), "DELETE", invalid, token)
end function

function HeartbeatFallbackSeconds() as integer
    return 10
end function

function SessionFrom(json as dynamic) as object
    beat = Int(ValueAt(json, "heartbeat_interval_s", 0))
    if beat <= 0 then beat = HeartbeatFallbackSeconds()

    return {
        id: TextOrBlank(ValueAt(json, "session_id", "")),
        mode: TextOrBlank(ValueAt(json, "mode", "")),
        container: TextOrBlank(ValueAt(json, "container", "")),
        manifestUrl: TextOrBlank(ValueAt(json, "manifest_url", "")),
        originMs: Int(ValueAt(json, "origin_ms", 0)),
        sequential: ValueAt(json, "sequential", false) = true,
        heartbeatSeconds: beat,
        selected: ValueAt(json, "selected", invalid)
    }
end function

function StreamUrl(base as string, path as dynamic) as string
    text = TextOrBlank(path)
    if IsBlank(text) then return ""
    if Left(LCase(text), 4) = "http" then return text

    return JoinUrl(base, text)
end function

function StreamFormatFor(mode as dynamic, container as dynamic) as string
    if LCase(TextOrBlank(mode)) <> "direct" then return "hls"

    formats = {
        "mp4": "mp4",
        "m4v": "mp4",
        "mov": "mp4",
        "mkv": "mkv",
        "webm": "mkv",
        "ts": "ts",
        "m2ts": "ts",
        "mts": "ts"
    }
    key = LCase(TextOrBlank(container))
    if formats.DoesExist(key) then return formats[key]
    return "mp4"
end function

function AttachSeconds(resumeMs as integer, originMs as integer, sequential as boolean) as integer
    if sequential then return 0

    at = resumeMs - originMs
    if at <= 0 then return 0
    return Int(at / 1000)
end function

function TimelineMs(positionSeconds as dynamic, originMs as integer, sequential as boolean) as integer
    at = Int(positionSeconds * 1000.0)
    if at < 0 then at = 0
    if sequential then return originMs + at

    return at
end function

function SegmentGraceMs() as integer
    return 4000
end function

function ProducedMs(segment as dynamic, positionSeconds as dynamic) as integer
    at = Int(positionSeconds * 1000.0)
    if at < 0 then at = 0

    fetching = Int(ValueAt(segment, "segStartTime", 0) * 1000.0)
    if fetching > at then at = fetching
    return at + SegmentGraceMs()
end function

function SeekPlan(targetMs as integer, originMs as integer, sequential as boolean, reachMs as integer) as object
    at = targetMs
    if at < 0 then at = 0

    if not sequential then return { server: false, seconds: Int(at / 1000), positionMs: at }
    if at < originMs then return { server: true, seconds: 0, positionMs: at }
    if at - originMs > reachMs then return { server: true, seconds: 0, positionMs: at }

    return { server: false, seconds: Int((at - originMs) / 1000), positionMs: at }
end function

function TimelineSpan(positionMs as dynamic, scrubMs as dynamic, durationMs as dynamic, width as integer) as object
    at = Int(width * TimelineFraction(positionMs, durationMs))

    if Int(scrubMs) < 0 then return { fillTo: at, previewTo: at, thumbAt: at, ghostAt: -1, ghost: false }

    target = Int(width * TimelineFraction(scrubMs, durationMs))

    lo = at
    hi = target
    if target < at
        lo = target
        hi = at
    end if

    return { fillTo: lo, previewTo: hi, thumbAt: at, ghostAt: target, ghost: true }
end function

function TimelineFraction(positionMs as dynamic, durationMs as dynamic) as float
    total = Int(durationMs)
    if total <= 0 then return 0.0

    at = Int(positionMs)
    if at <= 0 then return 0.0
    if at >= total then return 1.0

    return at / total
end function

function ControlsHideSeconds() as integer
    return 3
end function

function SpokenControlsHideSeconds() as integer
    return 12
end function

function HideSecondsFor(audioGuide as boolean) as integer
    if audioGuide then return SpokenControlsHideSeconds()
    return ControlsHideSeconds()
end function

function DoubleBackMs() as integer
    return 750
end function

function StallSeconds() as integer
    return 20
end function

function ProgressMark(positionMs as dynamic, bufferedPercent as dynamic) as integer
    mark = 0
    if positionMs <> invalid then mark = Int(positionMs)
    if bufferedPercent <> invalid then mark = mark + Int(bufferedPercent)
    return mark
end function

function StallTicks(previous as integer, lastMark as integer, mark as integer) as integer
    if mark <> lastMark then return 0
    return previous + 1
end function

function IsStalled(ticks as integer) as boolean
    return ticks >= StallSeconds()
end function

function DefaultCountdownSeconds() as integer
    return 10
end function

function CountdownOptions() as object
    return [
        { value: 5, label: PhraseWith("player.delaySeconds", { seconds: 5 }) },
        { value: 10, label: PhraseWith("player.delaySeconds", { seconds: 10 }) },
        { value: 15, label: PhraseWith("player.delaySeconds", { seconds: 15 }) },
        { value: 30, label: PhraseWith("player.delaySeconds", { seconds: 30 }) }
    ]
end function

function ClampedCountdownSeconds(seconds as integer) as integer
    for each option in CountdownOptions()
        if option.value = seconds then return seconds
    end for
    return DefaultCountdownSeconds()
end function

function CountdownLabel(seconds as dynamic) as string
    wanted = DefaultCountdownSeconds()
    if seconds <> invalid then wanted = ClampedCountdownSeconds(Int(seconds))

    return PhraseWith("player.delaySeconds", { seconds: wanted })
end function

function CountdownSeconds() as integer
    return ReadAutoplayDelay()
end function

function AutoplayOffValue() as integer
    return 0
end function

function AutoplayOptions() as object
    options = [{ value: AutoplayOffValue(), label: Phrase("state.off") }]
    for each option in CountdownOptions()
        options.Push(option)
    end for
    return options
end function

function AutoplayValue(on as dynamic, seconds as dynamic) as integer
    if on <> true then return AutoplayOffValue()
    if seconds = invalid then return DefaultCountdownSeconds()

    return ClampedCountdownSeconds(Int(seconds))
end function

function AutoplayLabel(on as dynamic, seconds as dynamic) as string
    if on <> true then return Phrase("state.off")

    return CountdownLabel(seconds)
end function

sub StoreAutoplay(seconds as integer)
    if seconds = AutoplayOffValue()
        WriteAutoplayNext(false)
        return
    end if

    WriteAutoplayNext(true)
    WriteAutoplayDelay(seconds)
end sub

function FirstEpisodeOf(items as dynamic) as dynamic
    episodes = OrderedEpisodes(items)
    if episodes.Count() = 0 then return invalid

    return episodes[0]
end function

function NextEpisodeAfter(items as dynamic, episodeId as dynamic) as dynamic
    episodes = OrderedEpisodes(items)
    at = IndexOfId(episodes, episodeId)
    if at < 0 or at + 1 >= episodes.Count() then return invalid

    return episodes[at + 1]
end function

function PreviousEpisodeBefore(items as dynamic, episodeId as dynamic) as dynamic
    episodes = OrderedEpisodes(items)
    at = IndexOfId(episodes, episodeId)
    if at < 1 then return invalid

    return episodes[at - 1]
end function

function LastEpisodeOf(items as dynamic) as dynamic
    episodes = OrderedEpisodes(items)
    if episodes.Count() = 0 then return invalid

    return episodes[episodes.Count() - 1]
end function

function FormatKbps(kbps as dynamic) as string
    rate = Int(kbps)
    if rate <= 0 then return ""
    if rate < 1000 then return rate.ToStr() + " kbps"

    scaled = Int(rate / 100.0 + 0.5)
    return (scaled \ 10).ToStr() + "." + (scaled MOD 10).ToStr() + " Mbps"
end function

function YesOrNo(value as dynamic) as string
    if value = true then return Phrase("state.yes")
    return Phrase("state.no")
end function

function PlayerDiagnostics(session as dynamic, info as dynamic) as object
    mode = TextOrBlank(ValueAt(session, "mode", ""))
    if IsBlank(mode) then mode = Phrase("status.unavailable")

    container = TextOrBlank(ValueAt(session, "container", ""))
    if IsBlank(container) then container = Phrase("diagnostics.none")

    bitrate = FormatKbps(ValueAt(info, "measuredBitrate", 0))
    if IsBlank(bitrate) then bitrate = Phrase("status.unavailable")

    started = ValueAt(info, "timeToStartStreaming", 0)
    start = Phrase("status.unavailable")
    if started > 0 then start = ScoreNumber(started) + " s"

    state = TextOrBlank(ValueAt(info, "state", ""))
    if IsBlank(state) then state = Phrase("status.unavailable")

    origin = Phrase("diagnostics.fromStart")
    if ValueAt(session, "sequential", false) = true then origin = FormatDuration(Int(ValueAt(session, "originMs", 0)))

    return [
        { label: Phrase("diagnostics.mode"), values: [mode] },
        { label: Phrase("fact.container"), values: [container] },
        { label: Phrase("diagnostics.state"), values: [state] },
        { label: Phrase("diagnostics.timeline"), values: [origin] },
        { label: Phrase("diagnostics.bitrate"), values: [bitrate] },
        { label: Phrase("diagnostics.buffer"), values: [Int(ValueAt(info, "percentage", 0)).ToStr() + "%"] },
        { label: Phrase("diagnostics.underrun"), values: [YesOrNo(ValueAt(info, "isUnderrun", false))] },
        { label: Phrase("diagnostics.startTime"), values: [start] }
    ]
end function

function VideoInfo(streamInfo as dynamic, buffering as dynamic, startSeconds as dynamic, state = "" as string) as object
    return {
        measuredBitrate: Int(ValueAt(streamInfo, "measuredBitrate", 0)),
        isUnderrun: ValueAt(streamInfo, "isUnderrun", false) = true,
        percentage: Int(ValueAt(buffering, "percentage", 0)),
        timeToStartStreaming: startSeconds,
        state: state
    }
end function

function IsReplayKey(key as dynamic) as boolean
    name = LCase(TextOrBlank(key))
    return name = "replay" or name = "instantreplay"
end function

function SeekStepMs(large as boolean) as integer
    if large then return 30000
    return 10000
end function

function PrebufferGiveUpSeconds() as integer
    return 10
end function

function HoldSpeeds() as object
    return [1, 3, 5, 10]
end function

function HoldStepSeconds() as integer
    return 3
end function

function HoldSpeedAt(heldSeconds as integer) as integer
    speeds = HoldSpeeds()

    rung = 0
    if heldSeconds > 0 then rung = Int(heldSeconds / HoldStepSeconds())
    if rung >= speeds.Count() then rung = speeds.Count() - 1

    return speeds[rung]
end function

function HoldStepMs(heldSeconds as integer) as integer
    return SeekStepMs(false) * HoldSpeedAt(heldSeconds)
end function

function HoldTickMs() as integer
    return 250
end function

function HoldHeldSeconds(ticks as integer) as integer
    return Int(ticks * HoldTickMs() / 1000)
end function

function HoldTickStepMs(ticks as integer) as integer
    return Int(HoldStepMs(HoldHeldSeconds(ticks)) * HoldTickMs() / 1000)
end function

function HoldTickMultiplied(ticks as integer) as boolean
    return HoldSpeedAt(HoldHeldSeconds(ticks)) > HoldSpeeds()[0]
end function

function PlayerButtons(playing as boolean, finished as boolean, episode as boolean, hasPrevious as boolean, hasNext as boolean) as object
    transport = { id: "play", icon: "icon-play", label: Phrase("player.play"), style: "glyph" }
    if playing
        transport.icon = "icon-pause"
        transport.label = Phrase("player.pause")
    end if
    if finished
        transport.id = "replay"
        transport.icon = "icon-play"
        transport.label = Phrase("player.replay")
    end if

    buttons = [
        { id: "quality", icon: "icon-quality", label: Phrase("player.quality"), style: "glyph" },
        { id: "subtitle", icon: "icon-subtitles", label: Phrase("heading.subtitles"), style: "glyph" }
    ]

    if episode
        buttons.Push({ id: "previous", icon: "icon-previous", label: Phrase("player.previousEpisode"), style: "glyph", spaceBefore: true, disabled: not hasPrevious })
        buttons.Push(transport)
        buttons.Push({ id: "next", icon: "icon-next", label: Phrase("player.nextEpisode"), style: "glyph", disabled: not hasNext })
    else
        transport.spaceBefore = true
        buttons.Push(transport)
    end if

    buttons.Push({ id: "audio", icon: "icon-audio", label: Phrase("heading.audio"), style: "glyph", spaceBefore: true })
    buttons.Push({ id: "settings", icon: "icon-settings", label: Phrase("player.settings"), style: "glyph" })

    return buttons
end function

function UpNextLabel(target as dynamic) as string
    label = TextOrBlank(ValueAt(target, "label", ""))
    if not IsBlank(label) then return label

    return TextOrBlank(ValueAt(target, "title", ""))
end function

function TransportIndex(buttons as dynamic) as integer
    if type(buttons) <> "roArray" then return 0

    for index = 0 to buttons.Count() - 1
        id = TextOrBlank(ValueAt(buttons[index], "id", ""))
        if id = "play" or id = "replay" or id = "retry" then return index
    end for

    return 0
end function

function TitleWithYear(title as dynamic, year as dynamic) as string
    name = TextOrBlank(title)
    stamp = NumberText(year)
    if IsBlank(name) or IsBlank(stamp) then return name

    return name + " (" + stamp + ")"
end function

function RetryPosition(lastMs as integer, storedMs as integer) as integer
    if lastMs > 0 then return lastMs
    if storedMs > 0 then return storedMs

    return 0
end function

function AbrSwitchingStrategy() as string
    return "full-adaptation"
end function

function AbrBounds(maxBitrateBps as integer) as object
    bounds = { strategy: AbrSwitchingStrategy(), maxKbps: 0 }
    if maxBitrateBps > 0 then bounds.maxKbps = Int(maxBitrateBps / 1000)

    return bounds
end function

function DefaultControls() as object
    return {
        height: 0,
        burn: false,
        downmix: false,
        audioTrack: -1,
        subtitle: invalid,
        subtitleOff: false,
        offsetMs: 0,
        delivery: "auto"
    }
end function

function ControlsCopy(controls as dynamic) as object
    copy = DefaultControls()
    if type(controls) <> "roAssociativeArray" then return copy

    for each key in copy
        if controls.DoesExist(key) then copy[key] = controls[key]
    end for
    return copy
end function

function SubtitleRef(subtitle as dynamic) as dynamic
    if type(subtitle) <> "roAssociativeArray" then return invalid

    kind = LCase(TextOrBlank(ValueAt(subtitle, "type", "")))
    if kind = "embedded" then return { type: "embedded", index: Int(ValueAt(subtitle, "index", 0)) }

    id = TextOrBlank(ValueAt(subtitle, "id", ""))
    if IsBlank(id) then return invalid
    return { type: "file", id: id }
end function

function TrackLanguage(track as dynamic) as string
    return LCase(TextOrBlank(ValueAt(track, "language", "")))
end function

function AudioLanguageOf(version as dynamic, index as dynamic) as string
    if index = invalid or Int(index) < 0 then return ""

    for each track in ListItems(ValueAt(version, "audio", invalid))
        if Int(ValueAt(track, "index", -1)) = Int(index) then return TrackLanguage(track)
    end for
    return ""
end function

function SubtitleLanguageOf(version as dynamic, subtitle as dynamic) as string
    ref = SubtitleRef(subtitle)
    if ref = invalid then return ""

    if ref.type = "embedded"
        for each track in ListItems(ValueAt(version, "subtitles", invalid))
            if Int(ValueAt(track, "index", -1)) = ref.index then return TrackLanguage(track)
        end for
        return ""
    end if

    for each file in ListItems(ValueAt(version, "subtitle_files", invalid))
        if TextOrBlank(ValueAt(file, "id", "")) = ref.id then return TrackLanguage(file)
    end for
    return ""
end function

function AudioIndexForLanguage(version as dynamic, language as string) as integer
    if IsBlank(language) then return -1

    for each track in ListItems(ValueAt(version, "audio", invalid))
        if TrackLanguage(track) = LCase(language) then return Int(ValueAt(track, "index", -1))
    end for
    return -1
end function

function SubtitleRefForLanguage(version as dynamic, language as string) as dynamic
    if IsBlank(language) then return invalid

    for each track in ListItems(ValueAt(version, "subtitles", invalid))
        if TrackLanguage(track) = LCase(language) then return { type: "embedded", index: Int(ValueAt(track, "index", 0)) }
    end for

    for each file in ListItems(ValueAt(version, "subtitle_files", invalid))
        if TrackLanguage(file) = LCase(language)
            id = TextOrBlank(ValueAt(file, "id", ""))
            if not IsBlank(id) then return { type: "file", id: id }
        end if
    end for
    return invalid
end function

function CarriedTracks(controls as dynamic, version as dynamic) as object
    kept = ControlsCopy(controls)

    subtitle = ""
    if not (kept.subtitleOff = true) then subtitle = SubtitleLanguageOf(version, kept.subtitle)

    return {
        audioLanguage: AudioLanguageOf(version, kept.audioTrack),
        subtitleLanguage: subtitle,
        subtitleOff: kept.subtitleOff = true
    }
end function

function ControlsForCarried(carried as dynamic, version as dynamic) as object
    controls = DefaultControls()
    if type(carried) <> "roAssociativeArray" then return controls

    controls.subtitleOff = ValueAt(carried, "subtitleOff", false) = true

    index = AudioIndexForLanguage(version, TextOrBlank(ValueAt(carried, "audioLanguage", "")))
    if index >= 0 then controls.audioTrack = index

    if not controls.subtitleOff
        ref = SubtitleRefForLanguage(version, TextOrBlank(ValueAt(carried, "subtitleLanguage", "")))
        if ref <> invalid then controls.subtitle = ref
    end if

    return controls
end function

function ControlsStartBody(controls as dynamic) as object
    body = {}
    if type(controls) <> "roAssociativeArray" then return body

    height = Int(ValueAt(controls, "height", 0))
    if height > 0 then body.target_height = height
    if ValueAt(controls, "burn", false) = true then body.force_burn = true
    if ValueAt(controls, "downmix", false) = true then body.downmix_stereo = true

    track = Int(ValueAt(controls, "audioTrack", -1))
    if track >= 0 then body.audio_track = track

    delivery = TextOrBlank(ValueAt(controls, "delivery", "auto"))
    if not IsBlank(delivery) and delivery <> "auto" then body.delivery = delivery

    if ValueAt(controls, "subtitleOff", false) = true
        body.subtitle_off = true
        return body
    end if

    ref = SubtitleRef(ValueAt(controls, "subtitle", invalid))
    if ref <> invalid
        selection = { track: ref }
        offset = Int(ValueAt(controls, "offsetMs", 0))
        if offset <> 0 then selection.offset_ms = offset
        body.subtitle = selection
    end if
    return body
end function

function ControlsUpdateBody(controls as dynamic) as object
    kept = ControlsCopy(controls)

    height = invalid
    if Int(kept.height) > 0 then height = Int(kept.height)

    body = {
        target_height: height,
        force_burn: kept.burn = true,
        downmix_stereo: kept.downmix = true,
        delivery: TextOrBlank(kept.delivery)
    }
    if IsBlank(body.delivery) then body.delivery = "auto"

    track = Int(kept.audioTrack)
    if track >= 0 then body.audio_track = track

    ref = SubtitleRef(kept.subtitle)
    if kept.subtitleOff = true or ref = invalid
        body.subtitle = { action: "disable" }
        return body
    end if

    change = { action: "set", track: ref }
    offset = Int(kept.offsetMs)
    if offset <> 0 then change.offset_ms = offset
    body.subtitle = change
    return body
end function

function ControlsWithSelection(controls as dynamic, selected as dynamic) as object
    merged = ControlsCopy(controls)
    if type(selected) <> "roAssociativeArray" then return merged

    track = ValueAt(selected, "audio_track", invalid)
    if track <> invalid then merged.audioTrack = Int(track)

    ref = SubtitleRef(ValueAt(selected, "subtitle_track", invalid))
    if ref <> invalid
        merged.subtitle = ref
        merged.subtitleOff = false
    end if
    return merged
end function

function QualityHeights() as object
    return [0, 1080, 720, 480, 320]
end function

function RungLabel(height as integer) as string
    if height <= 0 then return Phrase("player.original")

    return FormatQualityRung(height)
end function

function QualityRungs(sourceHeight as integer) as object
    rungs = []
    for each height in QualityHeights()
        if height = 0 or sourceHeight <= 0 or height < sourceHeight
            rungs.Push({ height: height, label: RungLabel(height) })
        end if
    end for
    return rungs
end function

function AudioOptions(version as dynamic) as object
    options = []
    tracks = ValueAt(version, "audio", invalid)
    if type(tracks) <> "roArray" then return options

    for each track in tracks
        options.Push({ index: Int(ValueAt(track, "index", 0)), label: AudioTrackLine(track) })
    end for
    return options
end function

function AudioLabelFor(version as dynamic, index as integer) as string
    for each option in AudioOptions(version)
        if option.index = index then return option.label
    end for
    return Phrase("status.unavailable")
end function

function SubtitleOptions(version as dynamic) as object
    options = [{ ref: invalid, label: Phrase("state.off") }]

    tracks = ValueAt(version, "subtitles", invalid)
    if type(tracks) = "roArray"
        for each track in tracks
            options.Push({
                ref: { type: "embedded", index: Int(ValueAt(track, "index", 0)) },
                label: SubtitleTrackLine(track)
            })
        end for
    end if

    files = ValueAt(version, "subtitle_files", invalid)
    if type(files) = "roArray"
        for each file in files
            id = TextOrBlank(ValueAt(file, "id", ""))
            if not IsBlank(id) then options.Push({ ref: { type: "file", id: id }, label: SubtitleFileLine(file) })
        end for
    end if
    return options
end function

function SameSubtitle(left as dynamic, right as dynamic) as boolean
    if left = invalid or right = invalid then return left = invalid and right = invalid
    if TextOrBlank(ValueAt(left, "type", "")) <> TextOrBlank(ValueAt(right, "type", "")) then return false

    if TextOrBlank(ValueAt(left, "type", "")) = "embedded"
        return Int(ValueAt(left, "index", -1)) = Int(ValueAt(right, "index", -2))
    end if
    return TextOrBlank(ValueAt(left, "id", "")) = TextOrBlank(ValueAt(right, "id", ""))
end function

function SubtitleLabelFor(version as dynamic, controls as dynamic) as string
    kept = ControlsCopy(controls)
    if kept.subtitleOff = true then return Phrase("state.off")

    wanted = SubtitleRef(kept.subtitle)
    for each option in SubtitleOptions(version)
        if SameSubtitle(option.ref, wanted) then return option.label
    end for
    return Phrase("state.off")
end function

function OffsetStepMs() as integer
    return 250
end function

function OffsetBoundMs() as integer
    return 30000
end function

function SteppedOffset(current as integer, delta as integer) as integer
    return ClampInt(current + delta, 0 - OffsetBoundMs(), OffsetBoundMs())
end function

function OffsetLabel(offsetMs as integer) as string
    if offsetMs = 0 then return PhraseWith("player.offsetMs", { ms: 0 })
    if offsetMs > 0 then return "+" + PhraseWith("player.offsetMs", { ms: offsetMs })

    return PhraseWith("player.offsetMs", { ms: offsetMs })
end function

function DeliveryOptions() as object
    return [
        { value: "auto", label: Phrase("player.deliveryAuto") },
        { value: "never", label: Phrase("player.deliveryNever") },
        { value: "always", label: Phrase("player.deliveryAlways") }
    ]
end function

function DeliveryLabel(delivery as dynamic) as string
    wanted = TextOrBlank(delivery)
    for each option in DeliveryOptions()
        if option.value = wanted then return option.label
    end for
    return Phrase("player.deliveryAuto")
end function

function OnOrOff(value as dynamic) as string
    if value = true then return Phrase("state.on")
    return Phrase("state.off")
end function

function SettingRow(name as string, value as string) as string
    return name + ": " + value
end function

function SubtitleRows(controls as dynamic, version as dynamic) as object
    kept = ControlsCopy(controls)

    rows = [{ id: "subtitle", label: Phrase("heading.subtitles"), detail: SubtitleLabelFor(version, kept) }]
    if SubtitleRef(kept.subtitle) = invalid then return rows

    rows.Push({ id: "offset", label: Phrase("player.subtitleOffset"), detail: OffsetLabel(Int(kept.offsetMs)) })
    rows.Push({ id: "burn", label: Phrase("player.burnSubtitles"), detail: OnOrOff(kept.burn) })
    return rows
end function

function AudioRows(controls as dynamic, version as dynamic) as object
    kept = ControlsCopy(controls)

    return [
        { id: "audio", label: Phrase("heading.audio"), detail: AudioLabelFor(version, Int(kept.audioTrack)) },
        { id: "downmix", label: Phrase("player.downmix"), detail: OnOrOff(kept.downmix) }
    ]
end function

function SettingsRows(controls as dynamic, flags as dynamic) as object
    kept = ControlsCopy(controls)

    return [
        { id: "delivery", label: Phrase("player.converting"), detail: DeliveryLabel(kept.delivery) },
        { id: "remaining", label: Phrase("player.timeDisplay"), detail: TimeDisplayLabel(ValueAt(flags, "remaining", false)) },
        { id: "autoplay", label: Phrase("player.autoplayNext"), detail: AutoplayLabel(ValueAt(flags, "autoplay", true), ValueAt(flags, "autoplayDelay", invalid)) },
        { id: "diagnostics", label: Phrase("player.diagnostics"), detail: OnOrOff(ValueAt(flags, "diagnostics", false)) }
    ]
end function

function PositionText(positionMs as integer, durationMs as integer, remaining as boolean) as string
    if not remaining then return FormatDuration(positionMs)
    if durationMs <= 0 then return FormatDuration(positionMs)

    unwatched = durationMs - positionMs
    if unwatched < 0 then unwatched = 0
    return "-" + FormatDuration(unwatched)
end function

function TimeDisplayLabel(remaining as dynamic) as string
    if remaining = true then return Phrase("player.timeRemaining")
    return Phrase("player.timeElapsed")
end function

function TimeDisplayOptions() as object
    return [
        { value: 0, label: Phrase("player.timeElapsed") },
        { value: 1, label: Phrase("player.timeRemaining") }
    ]
end function

function TimeDisplayValue(remaining as dynamic) as integer
    if remaining = true then return 1
    return 0
end function

function IndexOfValue(options as dynamic, key as string, value as dynamic) as integer
    if type(options) <> "roArray" then return 0

    index = 0
    for each option in options
        if ValueAt(option, key, invalid) = value then return index
        index = index + 1
    end for
    return 0
end function

function IndexOfSubtitle(options as dynamic, controls as dynamic) as integer
    kept = ControlsCopy(controls)
    if kept.subtitleOff = true or type(options) <> "roArray" then return 0

    wanted = SubtitleRef(kept.subtitle)
    index = 0
    for each option in options
        if SameSubtitle(ValueAt(option, "ref", invalid), wanted) then return index
        index = index + 1
    end for
    return 0
end function

function SourceHeightOf(version as dynamic) as integer
    tracks = ValueAt(version, "video", invalid)
    if type(tracks) <> "roArray" or tracks.Count() = 0 then return 0

    return Int(ValueAt(tracks[0], "height", 0))
end function

function SubtitlesAreBurned(selected as dynamic) as boolean
    return LCase(TextOrBlank(ValueAt(selected, "subtitle_delivery", ""))) = "burned"
end function
