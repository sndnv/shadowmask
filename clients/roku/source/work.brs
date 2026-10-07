function WorkBusyPollSeconds() as integer
    return 5
end function

function ProviderTimeoutSeconds() as integer
    return 60
end function

function VersionWorkPath(versionId as dynamic, suffix as string) as string
    return "/versions/" + EscapeValue(versionId) + suffix
end function

function VersionJobsRequest(base as string, token as string, versionId as dynamic) as object
    return BuildRequest(base, VersionWorkPath(versionId, "/jobs"), "GET", invalid, token)
end function

function TranscribeRequest(base as string, token as string, versionId as dynamic, audioIndex as integer) as object
    path = PathWithQuery(VersionWorkPath(versionId, "/subtitles/transcribe"), [["audio_track_index", audioIndex]])
    return BuildRequest(base, path, "POST", invalid, token)
end function

function TranslateRequest(base as string, token as string, versionId as dynamic, subtitleId as dynamic, language as dynamic) as object
    path = PathWithQuery(VersionWorkPath(versionId, "/subtitles/" + EscapeValue(subtitleId) + "/translate"), [["target_language", language]])
    return BuildRequest(base, path, "POST", invalid, token)
end function

function CombineRequest(base as string, token as string, versionId as dynamic, topId as dynamic, bottomId as dynamic) as object
    path = PathWithQuery(VersionWorkPath(versionId, "/subtitles/" + EscapeValue(topId) + "/combine"), [["bottom_subtitle_id", bottomId]])
    return BuildRequest(base, path, "POST", invalid, token)
end function

function SubtitleSearchRequest(base as string, token as string, versionId as dynamic, language as dynamic) as object
    path = PathWithQuery(VersionWorkPath(versionId, "/subtitles/search"), [["language", language]])
    request = BuildRequest(base, path, "GET", invalid, token)
    request.timeout = ProviderTimeoutSeconds()
    return request
end function

function SubtitleDownloadRequest(base as string, token as string, versionId as dynamic, candidate as dynamic) as object
    body = { file_id: TextOrBlank(ValueAt(candidate, "file_id", "")) }
    language = TextOrBlank(ValueAt(candidate, "language", ""))
    if not IsBlank(language) then body.language = language
    release = TextOrBlank(ValueAt(candidate, "release_name", ""))
    if not IsBlank(release) then body.release_name = release

    request = BuildRequest(base, VersionWorkPath(versionId, "/subtitles/download"), "POST", body, token)
    request.timeout = ProviderTimeoutSeconds()
    return request
end function

function SubtitleWorkKinds() as object
    return { transcription: "work.kindTranscription", translation: "work.kindTranslation", combine: "work.kindCombine" }
end function

function SubtitleWork(jobs as dynamic) as object
    shown = []
    if type(jobs) <> "roArray" then return shown

    kinds = SubtitleWorkKinds()
    for each job in jobs
        if kinds.DoesExist(LCase(TextOrBlank(ValueAt(job, "kind", "")))) then shown.Push(job)
    end for
    return shown
end function

function JobActive(job as dynamic) as boolean
    status = LCase(TextOrBlank(ValueAt(job, "status", "")))
    return status = "queued" or status = "running"
end function

function AwaitedWork(jobs as dynamic) as boolean
    for each job in SubtitleWork(jobs)
        status = LCase(TextOrBlank(ValueAt(job, "status", "")))
        if status = "running" then return true
        if status = "queued" and ValueAt(job, "ahead", invalid) <> invalid then return true
    end for
    return false
end function

function FinishedJobs(previous as dynamic, current as dynamic) as object
    finished = []
    if type(current) <> "roArray" then return finished

    before = {}
    if type(previous) = "roArray"
        for each job in previous
            before[TextOrBlank(ValueAt(job, "id", ""))] = JobActive(job)
        end for
    end if

    for each job in current
        id = TextOrBlank(ValueAt(job, "id", ""))
        wasOpen = true
        if before.DoesExist(id) then wasOpen = before[id]
        if wasOpen and not JobActive(job) then finished.Push(job)
    end for
    return finished
end function

function JobTitle(job as dynamic) as string
    kinds = SubtitleWorkKinds()
    kind = LCase(TextOrBlank(ValueAt(job, "kind", "")))
    title = kind
    if kinds.DoesExist(kind) then title = Phrase(kinds[kind])

    language = TextOrBlank(ValueAt(job, "language", ""))
    if IsBlank(language) then return title
    return title + " · " + LanguageName(language)
end function

function QueuedLine(ahead as dynamic) as string
    if ahead = invalid then return Phrase("work.queuedOff")

    count = Int(ahead)
    if count = 0 then return Phrase("work.queuedNext")
    if count = 1 then return Phrase("work.queuedOne")
    return PhraseWith("work.queuedAhead", { count: count })
end function

function JobStatusLine(job as dynamic, version as dynamic) as string
    status = LCase(TextOrBlank(ValueAt(job, "status", "")))
    if status = "queued" then return QueuedLine(ValueAt(job, "ahead", invalid))
    if status = "running" then return PhraseWith("work.running", { elapsed: ElapsedText(ValueAt(job, "elapsed_ms", 0)) })
    if status = "succeeded" then return FinishedLine(job, version)
    if status = "cancelled" then return Phrase("work.cancelled")

    reason = FailureReason(job)
    if IsBlank(reason) then return Phrase("work.failed")
    return PhraseWith("work.failedWith", { reason: reason })
end function

function FailureReason(job as dynamic) as string
    if LCase(TextOrBlank(ValueAt(job, "status", ""))) <> "failed" then return ""

    prefix = CreateObject("roRegex", "^(retryable|permanent) job failure:\s*", "i")
    return prefix.Replace(TextOrBlank(ValueAt(job, "last_error", "")), "").Trim()
end function

function ProducedFile(job as dynamic, version as dynamic) as dynamic
    id = TextOrBlank(ValueAt(job, "subtitle_id", ""))
    if IsBlank(id) then return invalid

    for each file in SubtitleFilesOf(version)
        if TextOrBlank(ValueAt(file, "id", "")) = id then return file
    end for
    return invalid
end function

function FinishedLine(job as dynamic, version as dynamic) as string
    if IsBlank(TextOrBlank(ValueAt(job, "subtitle_id", ""))) or version = invalid then return Phrase("work.succeeded")

    file = ProducedFile(job, version)
    if file = invalid then return Phrase("work.noSubtitle")
    return PhraseWith("work.ready", { produced: ProducedLabel(job, file) })
end function

function ProducedLabel(job as dynamic, file as dynamic) as string
    source = LCase(SubtitleSourceLabel(ValueAt(file, "source", invalid)))
    language = TextOrBlank(ValueAt(job, "language", ""))
    if IsBlank(language) then language = TextOrBlank(ValueAt(file, "language", ""))

    if IsBlank(language) then return PhraseWith("work.producedPlain", { source: source })
    return PhraseWith("work.producedNamed", { language: LanguageName(language), source: source })
end function

function SubtitleFilesOf(version as dynamic) as object
    files = ValueAt(version, "subtitle_files", invalid)
    if type(files) <> "roArray" then return []
    return files
end function

function SubtitleFileOptions(version as dynamic, skip = "" as string) as object
    options = []
    for each file in SubtitleFilesOf(version)
        id = TextOrBlank(ValueAt(file, "id", ""))
        if not IsBlank(id) and id <> skip then options.Push({ id: id, label: SubtitleFileLine(file) })
    end for
    return options
end function

function MoreSubtitlesRow() as object
    return { id: "more", label: Phrase("work.more"), detail: Phrase("work.add") }
end function

function WorkRows(version as dynamic, jobs as dynamic) as object
    rows = [{ id: "find", label: Phrase("work.find") }]
    if AudioOptions(version).Count() > 0 then rows.Push({ id: "transcribe", label: Phrase("work.transcribe") })

    files = SubtitleFileOptions(version).Count()
    if files > 0 then rows.Push({ id: "translate", label: Phrase("work.translate") })
    if files > 1 then rows.Push({ id: "combine", label: Phrase("work.combine") })

    shown = SubtitleWork(jobs)
    if shown.Count() = 0 then return rows

    rows.Push({ id: "jobs", label: Phrase("work.jobs"), heading: true })
    for each job in shown
        rows.Push({ id: "job", label: JobTitle(job), detail: JobStatusLine(job, version), info: true, column: true })
    end for
    return rows
end function

function ProviderSubtitleId(version as dynamic, fileId as dynamic) as string
    file = TextOrBlank(fileId)
    if IsBlank(file) then return ""
    return "opensubtitles:" + TextOrBlank(ValueAt(version, "id", "")) + ":" + file
end function

function HeldSubtitleIds(version as dynamic) as object
    held = {}
    held.SetModeCaseSensitive()
    for each file in SubtitleFilesOf(version)
        held[TextOrBlank(ValueAt(file, "id", ""))] = true
    end for
    return held
end function

function CandidateOptions(candidates as dynamic, version = invalid as dynamic) as object
    options = []
    if type(candidates) <> "roArray" then return options

    held = HeldSubtitleIds(version)
    for each candidate in candidates
        fileId = ValueAt(candidate, "file_id", "")
        label = TextOrBlank(ValueAt(candidate, "release_name", ""))
        if IsBlank(label) then label = TextOrBlank(fileId)

        id = ProviderSubtitleId(version, fileId)
        owned = not IsBlank(id) and held.DoesExist(id)
        parts = []
        if owned then parts.Push(Phrase("work.downloaded"))
        parts.Push(UCase(TextOrBlank(ValueAt(candidate, "format", ""))))
        count = CompactCount(ValueAt(candidate, "download_count", invalid))
        icon = ""
        if not IsBlank(count) then icon = "icon-download"
        options.Push({ candidate: candidate, label: label, meta: count, metaIcon: icon, detail: JoinParts(parts), info: owned })
    end for
    return options
end function

function FirstPickable(options as dynamic) as integer
    if type(options) <> "roArray" then return 0

    for index = 0 to options.Count() - 1
        if ValueAt(options[index], "info", false) <> true then return index
    end for
    return 0
end function

function WorkChoiceFields() as object
    return ["transcribeTrack", "translateSource", "translateTarget", "combineTop", "combineBottom", "findLanguage", "findPick"]
end function

function DownloadedNote(status as integer) as string
    if status = 200 then return Phrase("work.alreadyHeld")
    return Phrase("work.added")
end function

function JobAnnouncement(job as dynamic, version as dynamic) as object
    line = JobStatusLine(job, version)
    status = LCase(TextOrBlank(ValueAt(job, "status", "")))
    if status = "succeeded" and ProducedFile(job, version) <> invalid then return { kind: "ok", message: line }

    kind = "ok"
    if status = "failed" then kind = "err"
    return { kind: kind, message: JobTitle(job) + ": " + line }
end function

function WorksOnVersions(session as dynamic) as boolean
    role = LCase(TextOrBlank(ValueAt(session, "role", "")))
    if role = "admin" then return true
    return role = "player" and LCase(TextOrBlank(ValueAt(session, "accountRole", ""))) = "admin"
end function
