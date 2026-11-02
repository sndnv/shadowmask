sub init()
    m.top.functionName = "execute"
end sub

sub execute()
    job = m.top.job
    fs = CreateObject("roFileSystem")
    fs.CreateDirectory(job.directory)

    wanted = {}
    for each file in job.files
        wanted[file.path] = true
    end for
    for each name in fs.GetDirectoryListing(job.directory)
        path = job.directory + "/" + name
        if not wanted.DoesExist(path) then fs.Delete(path)
    end for

    missing = []
    for each file in job.files
        stat = fs.Stat(file.path)
        if stat = invalid or stat.size <> file.size then missing.Push(file)
    end for
    m.top.count = missing.Count()

    for index = 0 to missing.Count() - 1
        m.top.step = index + 1
        failure = Download(fs, missing[index], job)
        if failure <> ""
            m.top.result = { ok: false, error: failure }
            return
        end if
    end for

    m.top.result = { ok: true, error: "" }
end sub

function Download(fs as object, file as object, job as object) as string
    part = file.path + ".part"
    fs.Delete(part)

    port = CreateObject("roMessagePort")
    transfer = CreateObject("roUrlTransfer")
    transfer.SetMessagePort(port)
    transfer.SetUrl(file.url)
    if Len(job.certificates) > 0
        transfer.SetCertificatesFile(job.certificates)
        transfer.InitClientCertificates()
    end if

    if not transfer.AsyncGetToFile(part) then return "could not start the download"

    message = wait(job.timeout * 1000, port)
    if type(message) <> "roUrlEvent"
        transfer.AsyncCancel()
        fs.Delete(part)
        return "timed out"
    end if

    code = message.GetResponseCode()
    if code <> 200
        fs.Delete(part)
        reason = message.GetFailureReason()
        if Len(reason) = 0 then reason = "HTTP " + code.ToStr()
        return reason
    end if

    stat = fs.Stat(part)
    if stat = invalid or stat.size <> file.size
        fs.Delete(part)
        return "the download was incomplete"
    end if

    fs.Delete(file.path)
    if not fs.Rename(part, file.path)
        fs.Delete(part)
        return "the file could not be saved"
    end if
    return ""
end function
