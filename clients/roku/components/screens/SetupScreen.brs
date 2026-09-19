sub init()
    m.title = m.top.FindNode("title")
    m.serverField = m.top.FindNode("serverField")
    m.codeField = m.top.FindNode("codeField")
    m.deviceField = m.top.FindNode("deviceField")
    m.actions = m.top.FindNode("actions")
    m.status = m.top.FindNode("status")

    m.serverValue = ReadServerUrl()
    m.codeValue = ""
    m.deviceValue = ReadDeviceName()
    m.serverReady = false
    m.probed = false
    m.focused = invalid

    m.serverField.ObserveField("activated", "onFieldActivated")
    m.codeField.ObserveField("activated", "onFieldActivated")
    m.deviceField.ObserveField("activated", "onFieldActivated")
    m.actions.ObserveField("activated", "onAction")
end sub

sub render()
    theme = m.top.theme
    if theme = invalid or theme.Count() = 0 then return

    sizes = TypeScale()
    space = SpacingScale()
    left = ContentLeft()
    width = Int(ContentWidth() / 2)

    m.title.text = Phrase("form.linkDeviceTitle")
    m.title.color = theme.text
    m.title.font = SizedBoldFont(sizes.text2xl)
    m.title.translation = [left, ContentTop()]

    locked = not m.serverReady
    offset = ContentTop() + TextLinePitch(sizes.text2xl) + space.s5
    offset = DrawField(m.serverField, theme, Phrase("form.serverAddress"), m.serverValue, false, width, left, offset)
    offset = DrawField(m.codeField, theme, Phrase("form.linkCode"), m.codeValue, locked, width, left, offset)
    offset = DrawField(m.deviceField, theme, Phrase("form.deviceName"), m.deviceValue, locked, width, left, offset)

    m.actions.theme = theme
    m.actions.barWidth = ContentWidth()
    m.actions.buttons = [{ id: "link", label: Phrase("action.link"), style: "primary", disabled: not LinkReady() }]
    m.actions.translation = [left, offset + space.s3]

    m.status.theme = theme
    m.status.fontSize = sizes.textBase
    m.status.noteWidth = ContentWidth()
    m.status.translation = [left, offset + space.s3 + ActionControlHeight() + space.s6]

    m.top.SetFocus(true)

    if not m.probed
        m.probed = true
        if not IsBlank(m.serverValue) then Probe(m.serverValue)
    end if
end sub

function DrawField(field as object, theme as object, caption as string, text as string, locked as boolean, width as integer, left as integer, offset as integer) as integer
    field.theme = theme
    field.label = caption
    field.text = text
    field.disabled = locked
    field.fieldWidth = width
    field.translation = [left, offset]

    return offset + Int(field.barHeight) + SpacingScale().s4
end function

function LinkReady() as boolean
    return m.serverReady and not IsBlank(m.codeValue)
end function

sub onFieldActivated(event as object)
    id = TextOrBlank(event.GetData())

    if id = "serverField"
        RequestText("server", Phrase("form.serverAddress"), m.serverValue)
        return
    end if

    if id = "codeField"
        RequestText("code", Phrase("form.linkCode"), m.codeValue)
        return
    end if

    if id = "deviceField"
        RequestText("device", Phrase("form.deviceName"), m.deviceValue)
    end if
end sub

sub RequestText(field as string, title as string, text as string)
    m.top.keyboardRequest = { field: field, title: title, text: text }
end sub

sub onKeyboard()
    result = m.top.keyboardResult
    if result = invalid then return
    if result.cancelled then return

    if result.field = "server"
        m.serverValue = result.text.Trim()
        m.serverReady = false
    end if
    if result.field = "code" then m.codeValue = UCase(result.text.Trim())
    if result.field = "device" then m.deviceValue = result.text.Trim()

    render()
    ShowStatus("", "error")

    if result.field = "server" then Probe(m.serverValue)
end sub

sub Probe(address as string)
    normalized = NormalizeServerAddress(address)
    if normalized = invalid
        ShowStatus(Phrase("error.serverAddressInvalid"), "error")
        return
    end if

    m.candidate = normalized
    ShowStatus(Phrase("form.connecting"), "loading")
    m.probe = SendRequest(ProbeRequest(normalized), "onProbe")
end sub

sub onProbe()
    if m.probe = invalid then return

    if not ServerAnswered(m.probe.response.status)
        ShowStatus(Phrase("error.serverAddressUnanswered"), "error")
        return
    end if

    WriteServerUrl(m.candidate)
    m.serverValue = m.candidate
    m.serverReady = true

    render()
    ShowStatus("", "error")
    FocusControl(m.codeField)
end sub

sub onAction()
    Link()
end sub

sub Link()
    if not LinkReady() then return
    if IsBlank(m.deviceValue) then m.deviceValue = DefaultDeviceName()

    ShowStatus(Phrase("form.linking"), "loading")
    m.link = SendRequest(LinkRequest(ReadServerUrl(), m.codeValue, m.deviceValue), "onLinked")
end sub

sub onLinked()
    if m.link = invalid then return

    response = m.link.response
    parsed = ParseResponse(response.status, response.body)

    if not parsed.ok
        ShowStatus(PhraseWith("error.linkFailed", { detail: parsed.error }), "error")
        return
    end if

    token = ValueAt(parsed.json, "token", "")
    if IsBlank(token)
        ShowStatus(PhraseWith("error.linkFailed", { detail: Phrase("error.couldNotLoad") }), "error")
        return
    end if

    WriteToken(token)
    WriteDeviceId(TextOrBlank(ValueAt(parsed.json, "device_id", "")))
    WriteDeviceName(m.deviceValue)
    m.top.advance = "bootstrap"
end sub

sub ShowStatus(message as string, kind as string)
    m.status.kind = kind
    m.status.message = message
end sub

function StepControls() as object
    controls = [m.serverField]
    if m.serverReady
        controls.Push(m.codeField)
        controls.Push(m.deviceField)
    end if
    if LinkReady() then controls.Push(m.actions)

    return controls
end function

sub FocusControl(control as object)
    m.focused = control
    control.SetFocus(true)
end sub

sub onFocus()
    if not m.top.hasFocus()
        for each control in StepControls()
            if control.isInFocusChain() then m.focused = control
        end for
        return
    end if

    controls = StepControls()
    for each control in controls
        if m.focused <> invalid and control.isSameNode(m.focused)
            control.SetFocus(true)
            return
        end if
    end for
    FocusControl(controls[0])
end sub

function onKeyEvent(key as string, press as boolean) as boolean
    if not KeysAreOurs() then return false
    if not press then return false

    if key = "back" and IsPaired()
        m.top.advance = "bootstrap"
        return true
    end if

    return HandledStepKey(key, StepControls())
end function
