sub Main(args as dynamic)
    screen = CreateObject("roSGScreen")
    port = CreateObject("roMessagePort")
    screen.SetMessagePort(port)

    device = CreateObject("roDeviceInfo")
    if device <> invalid
        device.SetMessagePort(port)
        device.EnableLowGeneralMemoryEvent(true)
    end if

    input = CreateObject("roInput")
    if input <> invalid then input.SetMessagePort(port)

    screen.CreateScene("MainScene")
    shared = screen.GetGlobalNode()
    screen.Show()
    PassDeepLink(shared, args)

    while true
        message = wait(0, port)
        kind = type(message)

        if kind = "roSGScreenEvent" and message.IsScreenClosed()
            EndSessionOnExit(shared)
            return
        end if
        if kind = "roDeviceInfoEvent" then NoteLowMemory(shared)
        if kind = "roInputEvent" and message.IsInput() then PassDeepLink(shared, message.GetInfo())
    end while
end sub

sub PassDeepLink(shared as dynamic, info as dynamic)
    if shared = invalid then return
    if not shared.HasField("deepLink") then return
    if IsBlank(DeepLinkCommand(info)) then return

    shared.deepLink = info
end sub

sub EndSessionOnExit(shared as dynamic)
    if shared = invalid then return
    if not shared.HasField("sessionId") then return
    if IsBlank(shared.sessionId) then return

    request = EndSessionRequest(shared.serverUrl, shared.token, shared.sessionId)

    port = CreateObject("roMessagePort")
    transfer = CreateObject("roUrlTransfer")
    transfer.SetMessagePort(port)
    transfer.SetUrl(request.url)
    transfer.SetRequest(request.method)
    transfer.SetCertificatesFile(request.certificates)
    transfer.InitClientCertificates()
    transfer.SetHeaders(request.headers)

    if not transfer.AsyncGetToString() then return

    if type(wait(ExitRequestMs(), port)) = "roUrlEvent" then return

    transfer.AsyncCancel()
end sub

sub NoteLowMemory(shared as dynamic)
    if shared = invalid then return
    if not shared.HasField("lowMemory") then return

    shared.lowMemory = Int(shared.lowMemory) + 1
end sub
