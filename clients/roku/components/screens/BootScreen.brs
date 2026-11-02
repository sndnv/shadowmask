sub init()
    m.title = m.top.FindNode("title")
    m.note = m.top.FindNode("note")
    m.actions = m.top.FindNode("actions")

    m.failed = false
    m.failure = ""
    m.detail = ""
    m.started = false
    m.fontStep = 0
    m.fontCount = 0

    m.actions.ObserveField("activated", "onAction")
    m.top.ObserveField("focusedChild", "onFocus")
end sub

sub render()
    theme = m.top.theme
    if theme = invalid or theme.Count() = 0 then return

    sizes = TypeScale()
    space = SpacingScale()
    left = ContentLeft()
    top = ContentTop()

    m.title.visible = m.failed
    m.title.text = FailureTitle()
    m.title.color = theme.text
    m.title.font = SizedBoldFont(sizes.text2xl)
    m.title.width = ContentWidth()
    m.title.translation = [left, top]

    offset = top
    if m.failed then offset = top + TextLinePitch(sizes.text2xl) + space.s4

    m.note.theme = theme
    m.note.kind = NoteKind()
    m.note.fontSize = sizes.textBase
    m.note.noteWidth = ContentWidth()
    m.note.message = NoteMessage()
    m.note.translation = [left, offset]

    m.actions.visible = m.failed
    m.actions.theme = theme
    m.actions.barWidth = ContentWidth()
    m.actions.buttons = FailureButtons()
    m.actions.translation = [left, offset + Int(m.note.noteHeight) + space.s5]

    if m.failed then m.actions.SetFocus(true)

    if not m.started
        m.started = true
        Load()
    end if
end sub

function NoteKind() as string
    if m.failed then return "error"
    return "loading"
end function

function FailureTitle() as string
    if m.failure = "fonts" then return Phrase("error.fontsFailed")
    return Phrase("error.serverUnreachable")
end function

function NoteMessage() as string
    if not m.failed
        if m.fontCount > 0 then return PhraseWith("state.downloadingFonts", { step: m.fontStep, count: m.fontCount })
        return Phrase("state.loading")
    end if

    body = Phrase("error.serverUnreachableBody")
    if m.failure = "fonts" then body = Phrase("error.fontsFailedBody")
    if IsBlank(m.detail) then return body
    if m.detail = Phrase("error.serverUnreachable") then return body
    return body + " " + m.detail
end function

function FailureButtons() as object
    if not m.failed then return []

    if m.failure = "fonts"
        return [
            { id: "retryFonts", label: Phrase("action.retry"), style: "primary" },
            { id: "continueFonts", label: Phrase("action.continueAnyway") }
        ]
    end if

    return [
        { id: "retry", label: Phrase("action.retry"), style: "primary" },
        { id: "changeServer", label: Phrase("action.changeServer") }
    ]
end function

sub Load()
    m.failed = false
    m.failure = ""
    m.detail = ""
    render()

    m.selfTask = SendRequest(SelfRequest(ReadServerUrl(), ReadToken()), "onSelf")
end sub

sub onSelf()
    if m.selfTask = invalid then return

    response = m.selfTask.response
    parsed = ParseResponse(response.status, response.body)

    if IsUnauthorised(parsed.status)
        ClearToken()
        m.top.advance = "reset:SetupScreen"
        return
    end if

    if not parsed.ok
        Fail(parsed.error)
        return
    end if

    shared = m.global
    shared.userId = TextOrBlank(ValueAt(parsed.json, "id", ""))
    shared.username = TextOrBlank(ValueAt(parsed.json, "username", ""))
    shared.role = TextOrBlank(ValueAt(parsed.json, "role", ""))
    shared.accountRole = TextOrBlank(ValueAt(parsed.json, "account_role", ""))

    LoadFonts()
end sub

sub LoadFonts()
    m.failed = false
    m.failure = ""
    m.detail = ""
    m.fontStep = 0
    m.fontCount = 0
    render()

    m.infoTask = SendRequest(ServerInfoRequest(ReadServerUrl(), ReadToken()), "onFontsInfo")
end sub

sub onFontsInfo()
    if m.infoTask = invalid then return

    response = m.infoTask.response
    parsed = ParseResponse(response.status, response.body)
    m.infoTask = invalid
    if IsUnauthorised(parsed.status)
        ClearToken()
        m.top.advance = "reset:SetupScreen"
        return
    end if

    if not parsed.ok
        FailFonts(parsed.error)
        return
    end if

    plan = FontDownloads(ValueAt(parsed.json, "fonts", invalid), ReadServerUrl())
    if plan = invalid
        UseFonts({})
        return
    end if

    m.fontPlan = plan
    m.fontTask = CreateObject("roSGNode", "FontTask")
    m.fontTask.ObserveField("step", "onFontStep")
    m.fontTask.ObserveField("result", "onFontsSaved")
    m.fontTask.job = FontJob(plan)
    m.fontTask.control = "run"
end sub

sub onFontStep()
    if m.fontTask = invalid then return

    m.fontStep = m.fontTask.step
    m.fontCount = m.fontTask.count
    render()
end sub

sub onFontsSaved()
    if m.fontTask = invalid then return

    result = m.fontTask.result
    m.fontTask = invalid
    if ValueAt(result, "ok", false) = true
        UseFonts(ActiveFontsFrom(m.fontPlan))
        return
    end if

    FailFonts(ValueAt(result, "error", ""))
end sub

sub UseFonts(fonts as object)
    m.global.fonts = fonts
    m.top.advance = "reset:HomeScreen"
end sub

sub FailFonts(detail as dynamic)
    m.failure = "fonts"
    Fail(detail)
end sub

sub Fail(detail as dynamic)
    m.failed = true
    m.detail = TextOrBlank(detail)
    render()
end sub

sub onAction(event as object)
    id = TextOrBlank(event.GetData())

    if id = "retry"
        Load()
        return
    end if

    if id = "retryFonts"
        LoadFonts()
        return
    end if

    if id = "continueFonts"
        UseFonts({})
        return
    end if

    if id = "changeServer" then m.top.advance = "reset:SetupScreen"
end sub

sub onFocus()
    if not m.top.hasFocus() then return
    if m.failed then m.actions.SetFocus(true)
end sub
