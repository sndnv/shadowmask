sub init()
    m.top.functionName = "execute"
end sub

sub execute()
    job = m.top.job

    port = CreateObject("roMessagePort")
    transfer = CreateObject("roUrlTransfer")
    transfer.SetMessagePort(port)
    transfer.SetUrl(job.url)
    transfer.EnableEncodings(true)
    if Len(job.certificates) > 0
        transfer.SetCertificatesFile(job.certificates)
        transfer.InitClientCertificates()
    end if

    if not transfer.AsyncGetToString()
        SubtitlesDone(job, invalid, "could not start the request")
        return
    end if

    message = wait(job.timeout * 1000, port)
    if type(message) <> "roUrlEvent"
        transfer.AsyncCancel()
        SubtitlesDone(job, invalid, "timed out")
        return
    end if

    code = message.GetResponseCode()
    if code <> 200
        reason = message.GetFailureReason()
        if Len(reason) = 0 then reason = "HTTP " + code.ToStr()
        SubtitlesDone(job, invalid, reason)
        return
    end if

    SubtitlesDone(job, SubtitleCues(message.GetString()), "")
end sub

sub SubtitlesDone(job as object, cues as dynamic, failure as string)
    m.top.result = { attachId: job.attachId, ok: cues <> invalid, cues: cues, error: failure }
end sub
