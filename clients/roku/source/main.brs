sub Main()
    screen = CreateObject("roSGScreen")
    port = CreateObject("roMessagePort")
    screen.SetMessagePort(port)

    device = CreateObject("roDeviceInfo")
    if device <> invalid
        device.SetMessagePort(port)
        device.EnableLowGeneralMemoryEvent(true)
    end if

    screen.CreateScene("MainScene")
    shared = screen.GetGlobalNode()
    screen.Show()

    while true
        message = wait(0, port)
        kind = type(message)

        if kind = "roSGScreenEvent" and message.IsScreenClosed()
            EndSessionOnExit(shared)
            return
        end if
        if kind = "roDeviceInfoEvent" then NoteLowMemory(shared)
    end while
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
