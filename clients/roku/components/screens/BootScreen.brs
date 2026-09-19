sub init()
    m.title = m.top.FindNode("title")
    m.note = m.top.FindNode("note")
    m.actions = m.top.FindNode("actions")

    m.failed = false
    m.detail = ""
    m.started = false

    m.actions.ObserveField("activated", "onAction")
end sub

sub render()
    theme = m.top.theme
    if theme = invalid or theme.Count() = 0 then return

    sizes = TypeScale()
    space = SpacingScale()
    left = ContentLeft()
    top = ContentTop()

    m.title.visible = m.failed
    m.title.text = Phrase("error.serverUnreachable")
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

function NoteMessage() as string
    if not m.failed then return Phrase("state.loading")

    body = Phrase("error.serverUnreachableBody")
    if IsBlank(m.detail) then return body
    if m.detail = Phrase("error.serverUnreachable") then return body
    return body + " " + m.detail
end function

function FailureButtons() as object
    if not m.failed then return []

    return [
        { id: "retry", label: Phrase("action.retry"), style: "primary" },
        { id: "changeServer", label: Phrase("action.changeServer") }
    ]
end function

sub Load()
    m.failed = false
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

    m.top.advance = "reset:HomeScreen"
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

    if id = "changeServer" then m.top.advance = "reset:SetupScreen"
end sub

sub onFocus()
    if not m.top.hasFocus() then return
    if m.failed then m.actions.SetFocus(true)
end sub
