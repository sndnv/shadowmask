function ApiPrefix() as string
    return "/api/v1"
end function

function ProbeTimeoutSeconds() as integer
    return 5
end function

function RequestTimeoutSeconds() as integer
    return 20
end function

function SignOutTimeoutSeconds() as integer
    return 5
end function

function ExitRequestMs() as integer
    return 1000
end function

function CertificatesFile() as string
    return "common:/certs/ca-bundle.crt"
end function

function MaxConcurrentRequests() as integer
    return 4
end function

function ServerAnswered(status as integer) as boolean
    return status = 200 or status = 401
end function

function NormalizeServerAddress(input as dynamic) as dynamic
    if input = invalid then return invalid
    if type(input) <> "String" and type(input) <> "roString" then return invalid

    trimmed = input.Trim()
    if Len(trimmed) = 0 then return invalid

    scheme = "http"
    rest = trimmed

    marker = Instr(1, trimmed, "://")
    if marker > 0
        scheme = LCase(Left(trimmed, marker - 1))
        rest = Mid(trimmed, marker + 3)
    end if

    if scheme <> "http" and scheme <> "https" then return invalid

    rest = CutAt(CutAt(rest, "#"), "?")

    authority = rest
    path = ""
    slash = Instr(1, rest, "/")
    if slash > 0
        authority = Left(rest, slash - 1)
        path = Mid(rest, slash)
    end if

    at = LastIndexOf(authority, "@")
    if at > 0 then authority = Mid(authority, at + 1)

    if Len(authority) = 0 then return invalid
    if Instr(1, authority, " ") > 0 then return invalid
    authority = LCase(authority)

    while Len(path) > 0 and Right(path, 1) = "/"
        path = Left(path, Len(path) - 1)
    end while

    return scheme + "://" + authority + path
end function

function CutAt(value as string, marker as string) as string
    found = Instr(1, value, marker)
    if found > 0 then return Left(value, found - 1)
    return value
end function

function LastIndexOf(value as string, marker as string) as integer
    found = 0
    at = Instr(1, value, marker)
    while at > 0
        found = at
        at = Instr(at + 1, value, marker)
    end while
    return found
end function

function JoinUrl(base as string, path as string) as string
    trimmedBase = base
    while Len(trimmedBase) > 0 and Right(trimmedBase, 1) = "/"
        trimmedBase = Left(trimmedBase, Len(trimmedBase) - 1)
    end while

    if Len(path) = 0 then return trimmedBase
    if Left(path, 1) <> "/" then return trimmedBase + "/" + path
    return trimmedBase + path
end function

function BuildRequest(base as string, path as string, method = "GET" as string, body = invalid as dynamic, token = "" as string) as object
    request = {
        url: JoinUrl(base, ApiPrefix() + path),
        method: UCase(method),
        headers: {},
        body: "",
        timeout: RequestTimeoutSeconds(),
        certificates: CertificatesFile()
    }

    if not IsBlank(token)
        request.headers["Authorization"] = "Bearer " + token
    end if

    if body <> invalid
        request.headers["Content-Type"] = "application/json"
        request.body = FormatJson(body)
    end if

    return request
end function

function ProbeRequest(base as string) as object
    request = BuildRequest(base, "/server/info")
    request.timeout = ProbeTimeoutSeconds()
    return request
end function

function LinkRequest(base as string, code as string, deviceName as string) as object
    body = {
        code: code,
        device: {
            name: deviceName,
            platform: ClientPlatform()
        }
    }
    return BuildRequest(base, "/auth/link", "POST", body)
end function

function ClientPlatform() as string
    return "roku"
end function

function ExpectedProfileVersion() as integer
    return 1
end function

function ProfileVersionMatches(reported as dynamic) as boolean
    if reported = invalid then return true
    return reported = ExpectedProfileVersion()
end function

function ParseResponse(status as integer, body as dynamic) as object
    result = {
        ok: status >= 200 and status < 300,
        status: status,
        json: invalid,
        error: ""
    }

    if type(body) = "String" or type(body) = "roString"
        if Len(body) > 0 then result.json = ParseJson(body)
    end if

    if not result.ok
        result.error = ErrorTextFor(status, result.json)
    end if

    return result
end function

function ErrorTextFor(status as integer, json as dynamic) as string
    detail = TextOrBlank(ValueAt(json, "error.message", invalid))
    if IsBlank(detail) then detail = TextOrBlank(ValueAt(json, "message", invalid))
    if not IsBlank(detail) then return detail

    if status = 401 then return Phrase("error.notAuthorised")
    if status = 0 then return Phrase("error.serverUnreachable")
    return Phrase("error.couldNotLoad")
end function

function IsUnauthorised(status as integer) as boolean
    return status = 401
end function

function RequestFinished(task as dynamic) as boolean
    if task = invalid then return true

    response = task.response
    if type(response) = "roAssociativeArray" and response.DoesExist("status") then return true

    state = LCase(TextOrBlank(task.state))
    return state = "done" or state = "stop"
end function

function LiveRequests(tasks as dynamic) as object
    live = []
    if type(tasks) <> "roArray" then return live

    for each task in tasks
        if not RequestFinished(task) then live.Push(task)
    end for
    return live
end function

function QueuedRequests(queue as dynamic, task as dynamic) as object
    pending = []
    if type(queue) = "roArray" then pending.Append(queue)
    if task <> invalid then pending.Push(task)
    return pending
end function

function NextRequests(active as integer, queue as dynamic, limit as integer) as object
    plan = { start: [], queue: [] }
    if type(queue) <> "roArray" then return plan

    running = active
    for each task in queue
        if running < limit
            plan.start.Push(task)
            running = running + 1
        else
            plan.queue.Push(task)
        end if
    end for
    return plan
end function
