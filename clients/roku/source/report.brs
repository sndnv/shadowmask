function ReportSchema() as integer
    return 1
end function

function ReportHistoryLimit() as integer
    return 100
end function

function HistoryTime(iso as string, millis as integer) as string
    return Left(iso, 19) + "." + Right("00" + millis.ToStr(), 3) + "Z"
end function

function HistoryNow() as string
    now = CreateObject("roDateTime")
    return HistoryTime(now.ToISOString(), now.GetMilliseconds())
end function

function HistoryAdd(history as object, event as string, facts as dynamic, at as string) as object
    entry = { at: at, event: event }
    if type(facts) = "roAssociativeArray"
        for each key in facts
            entry[key] = facts[key]
        end for
    end if

    history.Push(entry)
    while history.Count() > ReportHistoryLimit()
        history.Shift()
    end while
    return history
end function

function ReportTracks(selected as dynamic) as dynamic
    if type(selected) <> "roAssociativeArray" then return invalid

    return {
        audio_track: ValueAt(selected, "audio_track"),
        audio_delivery: ValueAt(selected, "audio_delivery"),
        subtitle_track: ValueAt(selected, "subtitle_track"),
        subtitle_delivery: ValueAt(selected, "subtitle_delivery")
    }
end function

function AnsweredFacts(request as string, json as dynamic) as object
    return {
        request: request,
        session: ValueAt(json, "session_id"),
        mode: ValueAt(json, "mode"),
        container: ValueAt(json, "container"),
        origin_ms: ValueAt(json, "origin_ms"),
        sequential: ValueAt(json, "sequential"),
        selected: ReportTracks(ValueAt(json, "selected"))
    }
end function

function FailedFacts(request as string, parsed as dynamic) as object
    return {
        request: request,
        status: ValueAt(parsed, "status"),
        code: ValueAt(parsed, "json.error.code"),
        message: TextOrBlank(ValueAt(parsed, "error", ""))
    }
end function

function OsVersionText(os as dynamic) as string
    parts = []
    for each key in ["major", "minor", "revision"]
        value = TextOrBlank(ValueAt(os, key, ""))
        if not IsBlank(value) then parts.Push(value)
    end for

    buildNumber = TextOrBlank(ValueAt(os, "build", ""))
    text = parts.Join(".")
    if not IsBlank(buildNumber) then text = text + " build " + buildNumber
    return text
end function

function PlatformDescription() as string
    device = CreateObject("roDeviceInfo")
    return device.GetModelDisplayName() + " (" + device.GetModel() + "), Roku OS " + OsVersionText(device.GetOSVersion())
end function

function DiagnosticsLines(rows as object) as object
    lines = []
    for each row in rows
        values = []
        for each value in ValueAt(row, "values", [])
            values.Push(TextOrBlank(value))
        end for
        lines.Push(TextOrBlank(ValueAt(row, "label", "")) + ": " + values.Join(", "))
    end for
    return lines
end function

function ListCopy(list as object) as object
    copy = []
    for each item in list
        copy.Push(item)
    end for
    return copy
end function

function ReportPayload(platform as string, versionText as string, capabilities as object, history as object, panel as object, logLines as object) as object
    return {
        schema: ReportSchema(),
        client: { app: ClientPlatform(), platform: platform, version: versionText },
        capabilities: capabilities,
        history: ListCopy(history),
        panel: ListCopy(panel),
        log: ListCopy(logLines)
    }
end function

function ReportDecoding(measured as dynamic, choices as dynamic, sent as dynamic) as object
    return { measured: measured, choices: choices, sent: sent }
end function

function LastHistoryEntry(history as dynamic, event as string) as dynamic
    if type(history) <> "roArray" then return invalid

    for index = history.Count() - 1 to 0 step -1
        if TextOrBlank(ValueAt(history[index], "event", "")) = event then return history[index]
    end for
    return invalid
end function

function AnswerLine(answer as object) as string
    container = TextOrBlank(ValueAt(answer, "container", ""))
    if IsBlank(container) then container = Phrase("diagnostics.none")
    return PhraseWith("report.summaryAnswer", { mode: ValueAt(answer, "mode", ""), container: container, request: ValueAt(answer, "request", "") })
end function

function FailureLine(failure as object) as string
    what = TextOrBlank(ValueAt(failure, "request", ""))
    detail = (AsText(ValueAt(failure, "status")) + " " + TextOrBlank(ValueAt(failure, "code", ""))).Trim()
    if not IsBlank(detail) then what = what + ", " + detail
    return PhraseWith("report.summaryFailure", { what: what })
end function

function CountLine(key as string, count as integer) as string
    if count = 1 then return Phrase(key + "One")
    return PhraseWith(key, { count: count })
end function

function ReportSummaryText(payload as object) as string
    history = ValueAt(payload, "history", [])
    logLines = ValueAt(payload, "log", [])
    lines = [
        PhraseWith("report.summaryApp", { app: ValueAt(payload, "client.app", ""), version: ValueAt(payload, "client.version", "") }),
        PhraseWith("report.summaryPlatform", { platform: ValueAt(payload, "client.platform", "") }),
        CountLine("report.summaryHistory", history.Count())
    ]

    answer = LastHistoryEntry(history, "answered")
    if answer <> invalid then lines.Push(AnswerLine(answer))
    failure = LastHistoryEntry(history, "failed")
    if failure <> invalid then lines.Push(FailureLine(failure))
    lines.Push(CountLine("report.summaryLog", logLines.Count()))
    return lines.Join(Chr(10))
end function

function SendReportRequest(base as string, token as string, playerId as string, payload as object) as object
    body = {
        player_id: playerId,
        app: ClientPlatform(),
        platform: TextOrBlank(ValueAt(payload, "client.platform", "")),
        app_version: TextOrBlank(ValueAt(payload, "client.version", "")),
        payload: payload
    }
    return BuildRequest(base, "/diagnostics/reports", "POST", body, token)
end function
