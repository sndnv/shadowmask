sub init()
    m.top.functionName = "execute"
end sub

sub execute()
    request = m.top.request
    if request = invalid
        Answer(0, "", "no request")
        return
    end if

    port = CreateObject("roMessagePort")
    transfer = CreateObject("roUrlTransfer")
    transfer.SetMessagePort(port)
    transfer.SetUrl(request.url)
    transfer.SetRequest(request.method)
    transfer.EnableEncodings(true)
    transfer.RetainBodyOnError(true)

    if request.DoesExist("certificates") and Len(request.certificates) > 0
        transfer.SetCertificatesFile(request.certificates)
        transfer.InitClientCertificates()
    end if

    if request.headers <> invalid and request.headers.Count() > 0
        transfer.SetHeaders(request.headers)
    end if

    timeout = 20
    if request.DoesExist("timeout") then timeout = request.timeout

    if request.method = "POST" or request.method = "PUT"
        started = transfer.AsyncPostFromString(request.body)
    else
        started = transfer.AsyncGetToString()
    end if

    if not started
        Answer(0, "", "could not start the request")
        return
    end if

    message = wait(timeout * 1000, port)

    if type(message) = "roUrlEvent"
        Answer(message.GetResponseCode(), message.GetString(), message.GetFailureReason())
        return
    end if

    transfer.AsyncCancel()
    Answer(0, "", "timed out")
end sub

sub Answer(status as integer, body as string, failure as string)
    m.top.response = {
        status: status,
        body: body,
        error: failure
    }
end sub
