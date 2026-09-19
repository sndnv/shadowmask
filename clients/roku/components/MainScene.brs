sub init()
    m.background = m.top.FindNode("background")
    m.backdrop = m.top.FindNode("backdrop")
    m.backdropFade = m.top.FindNode("backdropFade")
    m.backdropOpacity = m.top.FindNode("backdropOpacity")
    m.backdropOut = m.top.FindNode("backdropOut")
    m.backdropFalloff = m.top.FindNode("backdropFalloff")
    m.navHint = m.top.FindNode("navHint")
    m.navHintRing = m.top.FindNode("navHintRing")
    m.navHintDisc = m.top.FindNode("navHintDisc")
    m.navHintChevron = m.top.FindNode("navHintChevron")
    m.texture = m.top.FindNode("texture")
    m.screens = m.top.FindNode("screens")
    m.rail = m.top.FindNode("rail")
    m.toast = m.top.FindNode("toast")
    m.choice = m.top.FindNode("choice")

    m.themeName = ReadThemeName()
    m.highContrast = ReadHighContrast()
    m.stack = []

    m.background.muteAudioGuide = true
    m.navHint.muteAudioGuide = true
    m.background.width = CanvasWidth()
    m.background.height = CanvasHeight()

    m.backdrop.width = CanvasWidth()
    m.backdrop.height = CanvasHeight()
    m.backdrop.loadDisplayMode = "scaleToFill"
    m.backdrop.loadWidth = CanvasWidth()
    m.backdrop.loadHeight = CanvasHeight()
    m.backdrop.ObserveField("loadStatus", "onBackdropLoaded")

    m.backdropFade.duration = BackdropFadeInSeconds()
    m.backdropOpacity.key = [0.0, 1.0]
    m.backdropOpacity.keyValue = [0.0, BackdropOpacity()]

    m.backdropOut.duration = BackdropFadeSeconds()
    m.backdropFalloff.key = [0.0, 1.0]
    m.backdropFalloff.keyValue = [BackdropOpacity(), 0.0]

    m.backdrop.muteAudioGuide = true
    m.texture.muteAudioGuide = true

    m.texture.uri = "pkg:/images/hex-texture.png"
    m.texture.width = CanvasWidth()
    m.texture.height = CanvasHeight()
    m.texture.opacity = TextureOpacity()

    m.rail.ObserveField("selected", "onNavSelected")
    m.rail.ObserveField("dismissed", "onNavDismissed")
    m.choice.ObserveField("result", "onChoiceResult")

    PublishSession()
    m.global.ObserveField("backdropUrl", "onBackdrop")
    m.global.ObserveField("lowMemory", "onLowMemory")
    ApplyTheme()
    Bootstrap()
    m.top.SetFocus(true)
end sub

sub PublishSession()
    shared = m.global
    if not shared.HasField("serverUrl")
        shared.AddFields({ serverUrl: "", token: "", userId: "", username: "", role: "", theme: {}, backdropUrl: "", decoding: {}, profileVersion: 0, negotiated: {}, lowMemory: 0, captionMode: "", sessionId: "" })
    end if

    shared.serverUrl = ReadServerUrl()
    shared.token = ReadToken()

    device = DeviceInfo()
    if device <> invalid then shared.captionMode = device.GetCaptionsMode()
end sub

sub Bootstrap()
    if not IsPaired()
        ResetTo("SetupScreen")
        return
    end if

    MeasuredDecoding()
    ResetTo("BootScreen")
    m.infoTask = SendRequest(ServerInfoRequest(ReadServerUrl(), ReadToken()), "onServerInfo")
end sub

sub onServerInfo()
    if m.infoTask = invalid then return

    parsed = ParseResponse(m.infoTask.response.status, m.infoTask.response.body)
    if not parsed.ok then return

    reported = ValueAt(parsed.json, "profile_version", invalid)
    if reported <> invalid then m.global.profileVersion = Int(reported)

    if not ProfileVersionMatches(reported)
        m.toast.kind = "err"
        m.toast.message = PhraseWith("error.profileVersion", { reported: reported, expected: ExpectedProfileVersion() })
    end if
end sub

sub onBackdrop()
    url = m.global.backdropUrl

    m.backdropFade.control = "stop"
    m.backdropOut.control = "stop"

    if IsBlank(url)
        if m.backdrop.opacity > 0
            m.backdropOut.duration = BackdropFadeOutSeconds(ScreenIsImmersive(m.stack.Peek()))
            m.backdropOut.control = "start"
        end if
        return
    end if

    m.backdrop.opacity = 0

    if m.backdrop.uri = url
        FadeBackdropIn()
        return
    end if

    m.backdrop.uri = url
end sub

sub onBackdropLoaded()
    FadeBackdropIn()
end sub

sub FadeBackdropIn()
    if m.backdrop.loadStatus <> "ready" then return
    if IsBlank(m.backdrop.uri) then return

    m.backdrop.opacity = 0
    m.backdropFade.control = "start"
end sub

sub ApplyTheme()
    m.theme = ThemedTokens(m.themeName, m.highContrast)
    m.global.theme = m.theme
    m.background.color = m.theme.bg
    m.texture.blendColor = m.theme.text

    space = SpacingScale()
    m.rail.theme = m.theme
    m.rail.translation = [- m.rail.railWidth, 0]

    m.toast.theme = m.theme
    m.toast.fontSize = TypeScale().textBase
    m.toast.translation = [ContentLeft(), CanvasHeight() - space.s10 - space.s6]

    m.choice.theme = m.theme

    for each screen in m.stack
        screen.theme = m.theme
    end for

    ApplyDialogPalette()
    RenderNavHint()
end sub

sub ApplyDialogPalette()
    palette = CreateObject("roSGNode", "RSGPalette")
    palette.colors = {
        DialogBackgroundColor: m.theme.surface,
        DialogItemColor: m.theme.text,
        DialogTextColor: m.theme.text,
        DialogFocusColor: m.theme.accent,
        DialogFocusItemColor: m.theme.accentContrast,
        DialogSecondaryTextColor: m.theme.muted,
        DialogSecondaryItemColor: m.theme.muted,
        DialogInputFieldColor: m.theme.surfaceAlt,
        DialogKeyboardColor: m.theme.surfaceAlt,
        DialogFootprintColor: m.theme.surfaceAlt,
        DialogDividerColor: m.theme.border
    }
    m.top.palette = palette
end sub

sub PushScreen(nodeType as string, target = invalid as dynamic)
    screen = CreateObject("roSGNode", nodeType)
    if screen = invalid
        m.toast.kind = "err"
        m.toast.message = PhraseWith("error.screenMissing", { screen: nodeType })
        return
    end if

    if screen.HasField("keyboardRequest")
        screen.ObserveField("keyboardRequest", "onKeyboardRequest")
    end if
    if screen.HasField("choiceRequest")
        screen.ObserveField("choiceRequest", "onChoiceRequest")
    end if
    if screen.HasField("toastRequest")
        screen.ObserveField("toastRequest", "onToastRequest")
    end if
    if screen.HasField("themeRequest")
        screen.ObserveField("themeRequest", "onThemeRequest")
    end if
    if screen.HasField("advance")
        screen.ObserveField("advance", "onAdvance")
    end if
    if screen.HasField("sessionEnd")
        screen.ObserveField("sessionEnd", "onSessionEnd")
    end if

    if m.stack.Count() > 0 then m.stack.Peek().visible = false

    if not ScreenIsImmersive(screen) then m.global.backdropUrl = ""
    m.screens.AppendChild(screen)
    m.stack.Push(screen)
    RestDeepScreens()

    screen.themeName = m.themeName
    if target <> invalid and screen.HasField("target") then screen.target = target
    screen.theme = m.theme

    screen.SetFocus(true)
    m.rail.section = SectionFor(screen)
    RenderNavHint()
end sub

function PopScreen() as boolean
    if m.stack.Count() <= 1 then return false

    leaving = m.stack.Pop()
    if leaving.HasField("release") then leaving.release = true
    RetireScreen(leaving)

    revealed = m.stack.Peek()
    RestScreen(revealed, false)
    revealed.visible = true
    revealed.SetFocus(true)

    m.global.backdropUrl = ""
    if revealed.HasField("backdropUrl") then m.global.backdropUrl = revealed.backdropUrl
    m.rail.section = SectionFor(revealed)
    return true
end function

sub ClearStack()
    while m.stack.Count() > 0
        leaving = m.stack.Pop()
        if leaving.HasField("release") then leaving.release = true
        RetireScreen(leaving)
    end while
end sub

sub RestScreen(screen as dynamic, sleeping as boolean)
    if screen = invalid then return

    for each holder in ContentHolders(screen)
        if holder.dormant <> sleeping then holder.dormant = sleeping
    end for
end sub

sub RestDeepScreens()
    keep = RetainedScreens()
    if m.stack.Count() <= keep then return

    for index = 0 to m.stack.Count() - keep - 1
        RestScreen(m.stack[index], true)
    end for
end sub

sub onLowMemory()
    for index = 0 to m.stack.Count() - 2
        RestScreen(m.stack[index], true)
    end for
end sub

sub RetireScreen(leaving as object)
    for each name in ["advance", "keyboardRequest", "choiceRequest", "toastRequest", "themeRequest"]
        if leaving.HasField(name) then leaving.UnobserveField(name)
    end for
    leaving.SetFocus(false)
    leaving.visible = false
    m.screens.RemoveChild(leaving)
end sub

sub ResetTo(nodeType as string)
    ClearStack()
    PushScreen(nodeType)
end sub

function ScreenIsImmersive(screen as dynamic) as boolean
    if screen = invalid then return false
    if not screen.HasField("immersive") then return false

    return screen.immersive = true
end function

function SectionFor(screen as dynamic) as string
    if screen = invalid then return ""
    if screen.HasField("section") and not IsBlank(screen.section) then return screen.section
    return screen.subtype()
end function

sub onAdvance()
    screen = m.stack.Peek()
    if screen = invalid then return

    target = screen.advance
    if IsBlank(target) then return

    if target = "bootstrap"
        ClearStack()
        PublishSession()
        Bootstrap()
        return
    end if

    if target = "back"
        PopScreen()
        return
    end if

    if Left(target, 6) = "reset:"
        if Mid(target, 7) = "SetupScreen" then PublishSession()
        ResetTo(Mid(target, 7))
        return
    end if

    payload = invalid
    if screen.HasField("advanceTarget") then payload = screen.advanceTarget
    PushScreen(target, payload)
end sub

sub onNavSelected()
    target = m.rail.selected
    if IsBlank(target)
        HideRail()
        return
    end if

    staying = SectionFor(m.stack.Peek()) = target
    HideRail(staying)
    if staying then return

    ResetTo(target)
end sub

sub onNavDismissed()
    HideRail()
end sub

sub ShowRail()
    if not IsPaired() then return
    if ScreenIsImmersive(m.stack.Peek()) then return
    if m.railVisible = true and m.rail.visible then return

    m.rail.identity = m.global.username
    m.rail.section = SectionFor(m.stack.Peek())
    m.rail.translation = [0, 0]
    m.rail.visible = true
    m.railVisible = true
    m.rail.SetFocus(true)
    RenderNavHint()
end sub

sub HideRail(restore = true as boolean)
    m.rail.translation = [- m.rail.railWidth, 0]
    m.rail.visible = false
    m.railVisible = false
    m.rail.SetFocus(false)

    screen = m.stack.Peek()
    if restore and screen <> invalid then screen.SetFocus(true)
    RenderNavHint()
end sub

sub RenderNavHint()
    if m.theme = invalid or m.theme.Count() = 0 then return

    space = SpacingScale()
    chevron = ChevronSize()
    padding = space.s2
    half = chevron + padding * 2
    disc = half * 2

    m.navHint.visible = IsPaired() and not ScreenIsImmersive(m.stack.Peek())
    if not m.navHint.visible then return

    origin = 0 - half
    if m.railVisible = true
        origin = Int(m.rail.railWidth) - half
        m.navHintChevron.uri = ChevronUri("right")
    else
        m.navHintChevron.uri = ChevronUri("left")
    end if
    m.navHint.translation = [origin, Int((CanvasHeight() - disc) / 2)]

    edge = BorderThickness()

    m.navHintRing.uri = GlyphUri("disc")
    m.navHintRing.width = disc
    m.navHintRing.height = disc
    m.navHintRing.blendColor = m.theme.border
    m.navHintRing.translation = [0, 0]

    m.navHintDisc.uri = GlyphUri("disc")
    m.navHintDisc.width = disc - edge * 2
    m.navHintDisc.height = disc - edge * 2
    m.navHintDisc.blendColor = m.theme.surfaceAlt
    m.navHintDisc.translation = [edge, edge]

    m.navHintChevron.width = chevron
    m.navHintChevron.height = chevron
    m.navHintChevron.blendColor = m.theme.muted
    m.navHintChevron.translation = [half + padding, (disc - chevron) / 2]
end sub

sub onSessionEnd(event as object)
    id = TextOrBlank(event.GetData())
    if IsBlank(id) then return

    m.endTask = SendRequest(EndSessionRequest(ReadServerUrl(), ReadToken(), id), "onSessionEnded")
end sub

sub onSessionEnded()
    m.endTask = invalid
end sub

sub onThemeRequest(event as object)
    wanted = TextOrBlank(event.GetData())
    if IsBlank(wanted) then return

    if wanted = ContrastRequest()
        SetHighContrast(not m.highContrast)
        return
    end if

    SetTheme(wanted)
end sub

sub SetHighContrast(value as boolean)
    m.highContrast = value
    WriteHighContrast(value)
    ApplyTheme()
end sub

sub onToastRequest(event as object)
    request = event.GetData()
    if request = invalid or request.Count() = 0 then return

    message = TextOrBlank(ValueAt(request, "message", ""))
    if IsBlank(message) then return

    m.toast.kind = TextOrBlank(ValueAt(request, "kind", "ok"))
    m.toast.message = message
end sub

sub onKeyboardRequest()
    screen = m.stack.Peek()
    if screen = invalid then return

    request = screen.keyboardRequest
    if request = invalid or request.Count() = 0 then return

    dialog = CreateObject("roSGNode", "KeyboardDialog")
    dialog.title = ValueAt(request, "title", "")
    dialog.text = ValueAt(request, "text", "")
    dialog.buttons = [Phrase("action.ok"), Phrase("action.cancel")]
    dialog.ObserveField("buttonSelected", "onKeyboardDone")

    m.keyboardDialog = dialog
    m.keyboardField = ValueAt(request, "field", "")
    m.top.dialog = dialog
end sub

sub onKeyboardDone()
    dialog = m.keyboardDialog
    if dialog = invalid then return

    screen = m.stack.Peek()
    if screen <> invalid and screen.HasField("keyboardResult")
        screen.keyboardResult = {
            field: m.keyboardField,
            text: dialog.text,
            cancelled: dialog.buttonSelected <> 0
        }
    end if

    m.keyboardDialog = invalid
    m.top.dialog = invalid
    FocusTopScreen()
end sub

sub FocusTopScreen()
    screen = m.stack.Peek()
    if screen <> invalid then screen.SetFocus(true)
end sub

sub onChoiceRequest()
    screen = m.stack.Peek()
    if screen = invalid then return

    request = screen.choiceRequest
    if request = invalid then return

    if IsDialogClose(request)
        CloseChoice()
        return
    end if

    m.choice.request = request
    if not m.choice.visible then return

    m.choice.SetFocus(true)
end sub

sub CloseChoice()
    if not m.choice.visible then return

    m.choice.request = DialogCloseRequest()
    m.choice.visible = false
    FocusTopScreen()
end sub

sub onChoiceResult(event as object)
    DeliverChoice(event.GetData())
end sub

sub DeliverChoice(result as object)
    screen = m.stack.Peek()
    if screen = invalid then return

    if screen.HasField("choiceResult") then screen.choiceResult = result
    if not m.choice.visible then FocusTopScreen()
end sub

sub SetTheme(name as string)
    wanted = ValidThemeName(name)
    if wanted = m.themeName then return

    m.themeName = wanted
    WriteThemeName(m.themeName)

    for each screen in m.stack
        screen.themeName = m.themeName
    end for

    ApplyTheme()

    m.toast.kind = "ok"
    m.toast.message = ThemeLabel(m.themeName)
end sub

function onKeyEvent(key as string, press as boolean) as boolean
    if not press then return false

    if m.choice.visible then return false

    if m.railVisible = true
        if key = "back" or key = "right"
            HideRail()
            return true
        end if
        return false
    end if

    if key = "left"
        ShowRail()
        return m.railVisible = true
    end if

    if key = "back"
        if PopScreen() then return true
        if not IsPaired() then return false
        if m.stack.Count() = 0 then return false

        top = m.stack.Peek()
        if SectionFor(top) = "HomeScreen" then return false
        if top.HasField("rootScreen") and top.rootScreen = true then return false

        ResetTo("HomeScreen")
        return true
    end if

    return false
end function
